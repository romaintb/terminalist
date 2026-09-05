use crate::entities::{label, project, section, task, task_label};
use crate::repositories::{LabelRepository, ProjectRepository, SectionRepository, TaskRepository};
use crate::storage::LocalStorage;
use crate::sync::SyncService;
use anyhow::{Context, Result};
use sea_orm::{ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

impl SyncService {
    /// Atomically replace the locally cached values represented by a remote snapshot.
    ///
    /// Fetching happens before this method is called. If any write fails, the transaction
    /// rolls back and the last valid cache remains available to the UI.
    pub(super) async fn store_snapshot(
        &self,
        storage: &LocalStorage,
        projects: &[crate::backend::BackendProject],
        labels: &[crate::backend::BackendLabel],
        sections: &[crate::backend::BackendSection],
        tasks: &[crate::backend::BackendTask],
    ) -> Result<()> {
        let transaction = storage
            .conn
            .begin()
            .await
            .context("Failed to start cache refresh transaction")?;

        // A full remote snapshot is authoritative about *which* objects exist, so drop the
        // rows it stopped returning. It says nothing about their local UUIDs, which the UI
        // keys its selection and cursor on, so the rows it still returns are updated in
        // place by the batches below rather than deleted and re-inserted.
        //
        // Parent links are detached first. A parent the remote stopped returning is about to
        // be deleted, and Todoist never returns completed tasks, so completing a parent makes
        // it vanish while its still-open subtasks keep coming. The task hierarchy FK cascades
        // and the project one restricts, so leaving the links in place would either delete a
        // live subtask or fail the delete outright. The batches relink from the snapshot.
        task::Entity::update_many()
            .col_expr(
                task::Column::ParentUuid,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .filter(task::Column::BackendUuid.eq(self.backend_uuid))
            .exec(&transaction)
            .await
            .context("Failed to detach cached task hierarchy")?;
        project::Entity::update_many()
            .col_expr(
                project::Column::ParentUuid,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .filter(project::Column::BackendUuid.eq(self.backend_uuid))
            .exec(&transaction)
            .await
            .context("Failed to detach cached project hierarchy")?;

        task::Entity::delete_many()
            .filter(task::Column::BackendUuid.eq(self.backend_uuid))
            .filter(task::Column::RemoteId.is_not_in(tasks.iter().map(|task| task.remote_id.as_str())))
            .exec(&transaction)
            .await
            .context("Failed to drop tasks missing from the snapshot")?;
        // An empty section list is ambiguous: `sync` turns a failed section fetch into an
        // empty slice and carries on, so treat it as "nothing to reconcile". Every other
        // fetch aborts the sync on failure, where empty honestly means empty.
        if !sections.is_empty() {
            section::Entity::delete_many()
                .filter(section::Column::BackendUuid.eq(self.backend_uuid))
                .filter(section::Column::RemoteId.is_not_in(sections.iter().map(|s| s.remote_id.as_str())))
                .exec(&transaction)
                .await
                .context("Failed to drop sections missing from the snapshot")?;
        }
        project::Entity::delete_many()
            .filter(project::Column::BackendUuid.eq(self.backend_uuid))
            .filter(project::Column::RemoteId.is_not_in(projects.iter().map(|p| p.remote_id.as_str())))
            .exec(&transaction)
            .await
            .context("Failed to drop projects missing from the snapshot")?;
        label::Entity::delete_many()
            .filter(label::Column::BackendUuid.eq(self.backend_uuid))
            .filter(label::Column::RemoteId.is_not_in(labels.iter().map(|l| l.remote_id.as_str())))
            .exec(&transaction)
            .await
            .context("Failed to drop labels missing from the snapshot")?;

        self.store_projects_batch(&transaction, projects)
            .await
            .context("Failed to store projects")?;
        self.store_labels_batch(&transaction, labels)
            .await
            .context("Failed to store labels")?;
        if !sections.is_empty() {
            self.store_sections_batch(&transaction, sections)
                .await
                .context("Failed to store sections")?;
        }
        self.store_tasks_batch(&transaction, tasks)
            .await
            .context("Failed to store tasks")?;

        transaction
            .commit()
            .await
            .context("Failed to commit cache refresh transaction")?;
        Ok(())
    }

    /// Look up local project UUID from remote project_id.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `remote_project_id` - Remote project ID from remote backend
    /// * `context` - Context string for error message (e.g., "task creation", "section sync")
    ///
    /// # Returns
    /// Local project UUID
    ///
    /// # Errors
    /// Returns error if project with given remote_id doesn't exist locally
    pub(super) async fn lookup_project_uuid<C>(
        conn: &C,
        backend_uuid: &Uuid,
        remote_project_id: &str,
        context: &str,
    ) -> Result<Uuid>
    where
        C: ConnectionTrait,
    {
        if let Some(project) = ProjectRepository::get_by_remote_id(conn, backend_uuid, remote_project_id).await? {
            Ok(project.uuid)
        } else {
            Err(anyhow::anyhow!(
                "Project with remote_id {} not found locally during {}. Please sync projects first.",
                remote_project_id,
                context
            ))
        }
    }

    /// Look up local section UUID from remote section_id.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `remote_section_id` - Remote section ID from remote backend
    ///
    /// # Returns
    /// Optional local section UUID (None if section_id is not provided)
    ///
    /// # Errors
    /// Returns error if database query fails
    pub(super) async fn lookup_section_uuid<C>(
        conn: &C,
        backend_uuid: &Uuid,
        remote_section_id: Option<&String>,
    ) -> Result<Option<Uuid>>
    where
        C: ConnectionTrait,
    {
        if let Some(remote_id) = remote_section_id {
            let section_uuid = SectionRepository::get_by_remote_id(conn, backend_uuid, remote_id)
                .await?
                .map(|s| s.uuid);
            Ok(section_uuid)
        } else {
            Ok(None)
        }
    }

    /// Store projects in batch
    pub(super) async fn store_projects_batch<C>(
        &self,
        conn: &C,
        projects: &[crate::backend::BackendProject],
    ) -> Result<()>
    where
        C: ConnectionTrait,
    {
        use sea_orm::sea_query::OnConflict;

        // First pass: Upsert all projects without parent_uuid relationships
        for backend_project in projects {
            let local_project = project::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_project.remote_id.clone()),
                name: ActiveValue::Set(backend_project.name.clone()),
                is_favorite: ActiveValue::Set(backend_project.is_favorite),
                is_inbox_project: ActiveValue::Set(backend_project.is_inbox),
                order_index: ActiveValue::Set(backend_project.order_index),
                parent_uuid: ActiveValue::Set(None),
            };

            let mut insert = project::Entity::insert(local_project);
            insert = insert.on_conflict(
                OnConflict::columns([project::Column::BackendUuid, project::Column::RemoteId])
                    .update_columns([
                        project::Column::Name,
                        project::Column::IsFavorite,
                        project::Column::IsInboxProject,
                        project::Column::OrderIndex,
                        project::Column::ParentUuid,
                    ])
                    .to_owned(),
            );
            insert.exec(conn).await?;
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for backend_project in projects {
            if let Some(remote_parent_id) = &backend_project.parent_remote_id {
                if let Some(parent) =
                    ProjectRepository::get_by_remote_id(conn, &self.backend_uuid, remote_parent_id).await?
                {
                    if let Some(project) =
                        ProjectRepository::get_by_remote_id(conn, &self.backend_uuid, &backend_project.remote_id)
                            .await?
                    {
                        let mut active_model: project::ActiveModel = project.into();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        ProjectRepository::update(conn, active_model).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Store labels in batch
    pub(super) async fn store_labels_batch<C>(&self, conn: &C, labels: &[crate::backend::BackendLabel]) -> Result<()>
    where
        C: ConnectionTrait,
    {
        use sea_orm::sea_query::OnConflict;

        for backend_label in labels {
            let local_label = label::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_label.remote_id.clone()),
                name: ActiveValue::Set(backend_label.name.clone()),
                order_index: ActiveValue::Set(backend_label.order_index),
                is_favorite: ActiveValue::Set(backend_label.is_favorite),
            };

            let mut insert = label::Entity::insert(local_label);
            insert = insert.on_conflict(
                OnConflict::columns([label::Column::BackendUuid, label::Column::RemoteId])
                    .update_columns([label::Column::Name, label::Column::OrderIndex, label::Column::IsFavorite])
                    .to_owned(),
            );
            insert.exec(conn).await?;
        }

        Ok(())
    }

    /// Store tasks in batch
    pub(super) async fn store_tasks_batch<C>(&self, conn: &C, tasks: &[crate::backend::BackendTask]) -> Result<()>
    where
        C: ConnectionTrait,
    {
        use sea_orm::sea_query::OnConflict;

        // Track task labels for later processing
        let mut task_labels_map: Vec<(Uuid, Vec<String>)> = Vec::new();

        // First pass: Upsert all tasks without parent_uuid relationships
        for backend_task in tasks {
            let label_names = backend_task.labels.clone();

            // Look up local project UUID from remote project_id
            let project_uuid = match Self::lookup_project_uuid(
                conn,
                &self.backend_uuid,
                &backend_task.project_remote_id,
                "task batch sync",
            )
            .await
            {
                Ok(uuid) => uuid,
                Err(_) => {
                    // Skip tasks whose projects don't exist locally (can happen with free tier API limitations)
                    continue;
                }
            };

            // Look up local section UUID from remote section_id if present
            let section_uuid =
                Self::lookup_section_uuid(conn, &self.backend_uuid, backend_task.section_remote_id.as_ref()).await?;

            let local_task = task::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_task.remote_id.clone()),
                content: ActiveValue::Set(backend_task.content.clone()),
                description: ActiveValue::Set(backend_task.description.clone()),
                project_uuid: ActiveValue::Set(project_uuid),
                section_uuid: ActiveValue::Set(section_uuid),
                parent_uuid: ActiveValue::Set(None),
                priority: ActiveValue::Set(backend_task.priority),
                order_index: ActiveValue::Set(backend_task.order_index),
                due_date: ActiveValue::Set(backend_task.due_date.clone()),
                due_datetime: ActiveValue::Set(backend_task.due_datetime.clone()),
                is_recurring: ActiveValue::Set(backend_task.is_recurring),
                deadline: ActiveValue::Set(backend_task.deadline.clone()),
                duration: ActiveValue::Set(backend_task.duration.clone()),
                is_completed: ActiveValue::Set(backend_task.is_completed),
                is_deleted: ActiveValue::Set(false),
            };

            let mut insert = task::Entity::insert(local_task);
            insert = insert.on_conflict(
                OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                    .update_columns([
                        task::Column::Content,
                        task::Column::Description,
                        task::Column::ProjectUuid,
                        task::Column::SectionUuid,
                        task::Column::ParentUuid,
                        task::Column::Priority,
                        task::Column::OrderIndex,
                        task::Column::DueDate,
                        task::Column::DueDatetime,
                        task::Column::IsRecurring,
                        task::Column::Deadline,
                        task::Column::Duration,
                        task::Column::IsCompleted,
                        task::Column::IsDeleted,
                    ])
                    .to_owned(),
            );
            insert.exec(conn).await?;

            // Get the uuid of the task we just inserted/updated
            if let Some(task) =
                TaskRepository::get_by_remote_id(conn, &self.backend_uuid, &backend_task.remote_id).await?
            {
                task_labels_map.push((task.uuid, label_names));
            }
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for backend_task in tasks {
            if let Some(remote_parent_id) = &backend_task.parent_remote_id {
                if let Some(parent) =
                    TaskRepository::get_by_remote_id(conn, &self.backend_uuid, remote_parent_id).await?
                {
                    if let Some(task) =
                        TaskRepository::get_by_remote_id(conn, &self.backend_uuid, &backend_task.remote_id).await?
                    {
                        let mut active_model: task::ActiveModel = task.into();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        TaskRepository::update(conn, active_model).await?;
                    }
                }
            }
        }

        // Delete task-label relationships only for tasks being synced
        for backend_task in tasks {
            if let Some(task) =
                TaskRepository::get_by_remote_id(conn, &self.backend_uuid, &backend_task.remote_id).await?
            {
                task_label::Entity::delete_many()
                    .filter(task_label::Column::TaskUuid.eq(task.uuid))
                    .exec(conn)
                    .await?;
            }
        }

        // Recreate relationships
        for (task_uuid, label_names) in task_labels_map {
            if !label_names.is_empty() {
                // Find label UUIDs by names
                for label_name in label_names {
                    if let Some(label) = LabelRepository::get_by_name(conn, &label_name).await? {
                        let task_label_relation = task_label::ActiveModel {
                            task_uuid: ActiveValue::Set(task_uuid),
                            label_uuid: ActiveValue::Set(label.uuid),
                        };
                        task_label::Entity::insert(task_label_relation)
                            .on_conflict(
                                sea_orm::sea_query::OnConflict::columns([
                                    task_label::Column::TaskUuid,
                                    task_label::Column::LabelUuid,
                                ])
                                .do_nothing()
                                .to_owned(),
                            )
                            .exec(conn)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Store sections in batch
    pub(super) async fn store_sections_batch<C>(
        &self,
        conn: &C,
        sections: &[crate::backend::BackendSection],
    ) -> Result<()>
    where
        C: ConnectionTrait,
    {
        use sea_orm::sea_query::OnConflict;

        for backend_section in sections {
            // Look up local project UUID from remote project_id
            let project_uuid = Self::lookup_project_uuid(
                conn,
                &self.backend_uuid,
                &backend_section.project_remote_id,
                "section sync",
            )
            .await?;

            let local_section = section::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_section.remote_id.clone()),
                name: ActiveValue::Set(backend_section.name.clone()),
                project_uuid: ActiveValue::Set(project_uuid),
                order_index: ActiveValue::Set(backend_section.order_index),
            };

            let mut insert = section::Entity::insert(local_section);
            insert = insert.on_conflict(
                OnConflict::columns([section::Column::BackendUuid, section::Column::RemoteId])
                    .update_columns([section::Column::Name, section::Column::ProjectUuid, section::Column::OrderIndex])
                    .to_owned(),
            );
            insert.exec(conn).await?;
        }

        Ok(())
    }

    /// Look up remote_id from local task UUID (with automatic locking).
    ///
    /// # Arguments
    /// * `task_uuid` - Local task UUID
    ///
    /// # Returns
    /// Remote task ID for remote backend
    ///
    /// # Errors
    /// Returns error if task with given UUID doesn't exist locally
    pub(super) async fn get_task_remote_id(&self, task_uuid: &Uuid) -> Result<String> {
        let storage = self.storage.lock().await;
        TaskRepository::get_remote_id(&storage.conn, task_uuid).await
    }

    /// Look up remote_id from local project UUID (with automatic locking).
    ///
    /// # Arguments
    /// * `project_uuid` - Local project UUID
    ///
    /// # Returns
    /// Remote project ID for remote backend
    ///
    /// # Errors
    /// Returns error if project with given UUID doesn't exist locally
    pub(super) async fn get_project_remote_id(&self, project_uuid: &Uuid) -> Result<String> {
        let storage = self.storage.lock().await;
        ProjectRepository::get_remote_id(&storage.conn, project_uuid).await
    }

    /// Look up remote_id from local label UUID (with automatic locking).
    ///
    /// # Arguments
    /// * `label_uuid` - Local label UUID
    ///
    /// # Returns
    /// Remote label ID for remote backend
    ///
    /// # Errors
    /// Returns error if label with given UUID doesn't exist locally
    pub(super) async fn get_label_remote_id(&self, label_uuid: &Uuid) -> Result<String> {
        let storage = self.storage.lock().await;
        LabelRepository::get_remote_id(&storage.conn, label_uuid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{BackendProject, BackendSection, BackendTask};
    use crate::entities::backend;
    use sea_orm::{EntityTrait, Set};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// A sync service over a throwaway database file, with its backend row already present.
    async fn test_service(label: &str) -> (PathBuf, Arc<Mutex<LocalStorage>>, SyncService) {
        let db_path = std::env::temp_dir().join(format!("terminalist-{label}-{}.db", Uuid::new_v4()));
        let storage = LocalStorage::new_at(db_path.clone()).await.unwrap();
        let backend_uuid = Uuid::new_v4();

        backend::Entity::insert(backend::ActiveModel {
            uuid: Set(backend_uuid),
            backend_type: Set("test".to_string()),
            name: Set("Test".to_string()),
            is_enabled: Set(true),
            credentials: Set("{}".to_string()),
            settings: Set("{}".to_string()),
        })
        .exec(&storage.conn)
        .await
        .unwrap();

        let storage = Arc::new(Mutex::new(storage));
        let service = SyncService::new_for_test(storage.clone(), backend_uuid);
        (db_path, storage, service)
    }

    /// Closing the pool before removing the file keeps Windows and NFS happy.
    async fn teardown(db_path: PathBuf, storage: Arc<Mutex<LocalStorage>>) {
        storage.lock().await.conn.clone().close().await.unwrap();
        let _ = std::fs::remove_file(db_path);
    }

    fn project(remote_id: &str, order_index: i32) -> BackendProject {
        BackendProject {
            remote_id: remote_id.to_string(),
            name: format!("Project {remote_id}"),
            is_favorite: false,
            is_inbox: false,
            order_index,
            parent_remote_id: None,
        }
    }

    fn task(remote_id: &str, project_remote_id: &str, parent_remote_id: Option<&str>) -> BackendTask {
        BackendTask {
            remote_id: remote_id.to_string(),
            content: format!("Task {remote_id}"),
            description: None,
            project_remote_id: project_remote_id.to_string(),
            section_remote_id: None,
            parent_remote_id: parent_remote_id.map(str::to_string),
            priority: 1,
            order_index: 1,
            due_date: None,
            due_datetime: None,
            is_recurring: false,
            deadline: None,
            duration: None,
            is_completed: false,
            labels: Vec::new(),
        }
    }

    #[tokio::test]
    async fn failed_snapshot_write_preserves_the_previous_cache() {
        let (db_path, storage, service) = test_service("snapshot").await;

        let cached_project = project("cached-project", 1);
        {
            let storage = storage.lock().await;
            service
                .store_snapshot(&storage, std::slice::from_ref(&cached_project), &[], &[], &[])
                .await
                .unwrap();
        }

        // The section names a project the snapshot does not carry, so storing it fails and
        // must take the whole transaction down with it.
        let invalid_section = BackendSection {
            remote_id: "invalid-section".to_string(),
            name: "Invalid section".to_string(),
            project_remote_id: "missing-project".to_string(),
            order_index: 1,
        };
        {
            let storage = storage.lock().await;
            let result = service
                .store_snapshot(
                    &storage,
                    &[project("replacement-project", 2)],
                    &[],
                    &[invalid_section],
                    &[],
                )
                .await;
            assert!(result.is_err());

            let projects = ProjectRepository::get_all(&storage.conn).await.unwrap();
            assert_eq!(projects.len(), 1);
            assert_eq!(projects[0].remote_id, "cached-project");
        }

        teardown(db_path, storage).await;
    }

    #[tokio::test]
    async fn successful_snapshot_removes_objects_missing_from_backend() {
        let (db_path, storage, service) = test_service("replace-snapshot").await;

        {
            let storage = storage.lock().await;
            service
                .store_snapshot(&storage, &[project("old-project", 1)], &[], &[], &[])
                .await
                .unwrap();
            service
                .store_snapshot(&storage, &[project("new-project", 2)], &[], &[], &[])
                .await
                .unwrap();

            let projects = ProjectRepository::get_all(&storage.conn).await.unwrap();
            assert_eq!(projects.len(), 1);
            assert_eq!(projects[0].remote_id, "new-project");
        }

        teardown(db_path, storage).await;
    }

    /// The UI keys its selection on local UUIDs, so a row the remote still returns has to
    /// keep the UUID it had, and a subtask has to survive its parent being completed away.
    #[tokio::test]
    async fn snapshot_keeps_uuids_and_orphans_subtasks_of_vanished_parents() {
        let (db_path, storage, service) = test_service("stable-uuids").await;
        let backend_uuid = service.backend_uuid;

        let (project_uuid, subtask_uuid) = {
            let storage = storage.lock().await;
            service
                .store_snapshot(
                    &storage,
                    &[project("p1", 1)],
                    &[],
                    &[],
                    &[task("parent", "p1", None), task("subtask", "p1", Some("parent"))],
                )
                .await
                .unwrap();

            let subtask = TaskRepository::get_by_remote_id(&storage.conn, &backend_uuid, "subtask")
                .await
                .unwrap()
                .expect("subtask cached");
            let parent = TaskRepository::get_by_remote_id(&storage.conn, &backend_uuid, "parent")
                .await
                .unwrap()
                .expect("parent cached");
            assert_eq!(subtask.parent_uuid, Some(parent.uuid));

            (
                ProjectRepository::get_all(&storage.conn).await.unwrap()[0].uuid,
                subtask.uuid,
            )
        };

        // Completing the parent makes Todoist stop returning it, while the subtask keeps
        // coming. Renaming the project proves the row was updated rather than replaced.
        {
            let storage = storage.lock().await;
            let mut renamed = project("p1", 1);
            renamed.name = "Renamed".to_string();
            service
                .store_snapshot(&storage, &[renamed], &[], &[], &[task("subtask", "p1", Some("parent"))])
                .await
                .unwrap();

            let projects = ProjectRepository::get_all(&storage.conn).await.unwrap();
            assert_eq!(projects.len(), 1);
            assert_eq!(projects[0].uuid, project_uuid, "project uuid churned across a sync");
            assert_eq!(projects[0].name, "Renamed");

            let subtask = TaskRepository::get_by_remote_id(&storage.conn, &backend_uuid, "subtask")
                .await
                .unwrap()
                .expect("subtask survives its parent being completed");
            assert_eq!(subtask.uuid, subtask_uuid, "task uuid churned across a sync");
            assert_eq!(subtask.parent_uuid, None);

            assert!(TaskRepository::get_by_remote_id(&storage.conn, &backend_uuid, "parent")
                .await
                .unwrap()
                .is_none());
        }

        teardown(db_path, storage).await;
    }

    /// `sync` turns a failed section fetch into an empty slice, so an empty section list
    /// cannot be read as "the remote has no sections".
    #[tokio::test]
    async fn empty_section_list_leaves_cached_sections_alone() {
        let (db_path, storage, service) = test_service("empty-sections").await;

        let section = BackendSection {
            remote_id: "s1".to_string(),
            name: "Section".to_string(),
            project_remote_id: "p1".to_string(),
            order_index: 1,
        };
        {
            let storage = storage.lock().await;
            service
                .store_snapshot(&storage, &[project("p1", 1)], &[], &[section], &[])
                .await
                .unwrap();
            service
                .store_snapshot(&storage, &[project("p1", 1)], &[], &[], &[])
                .await
                .unwrap();

            let sections = SectionRepository::get_all(&storage.conn).await.unwrap();
            assert_eq!(sections.len(), 1);
        }

        teardown(db_path, storage).await;
    }
}
