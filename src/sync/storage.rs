use crate::entities::{label, project, section, task, task_label};
use crate::repositories::{LabelRepository, ProjectRepository, SectionRepository, TaskRepository};
use crate::storage::LocalStorage;
use crate::sync::SyncService;
use anyhow::Result;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

impl SyncService {
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
    pub(super) async fn lookup_project_uuid(
        txn: &sea_orm::DatabaseTransaction,
        backend_uuid: &Uuid,
        remote_project_id: &str,
        context: &str,
    ) -> Result<Uuid> {
        if let Some(project) = ProjectRepository::get_by_remote_id(txn, backend_uuid, remote_project_id).await? {
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
    pub(super) async fn lookup_section_uuid(
        txn: &sea_orm::DatabaseTransaction,
        backend_uuid: &Uuid,
        remote_section_id: Option<&String>,
    ) -> Result<Option<Uuid>> {
        if let Some(remote_id) = remote_section_id {
            let section_uuid = SectionRepository::get_by_remote_id(txn, backend_uuid, remote_id)
                .await?
                .map(|s| s.uuid);
            Ok(section_uuid)
        } else {
            Ok(None)
        }
    }

    /// Store projects in batch
    ///
    /// An empty `projects` slice means "nothing to reconcile", not "the remote has nothing":
    /// `is_not_in(vec![])` matches every row, so an empty-but-successful fetch would otherwise
    /// blank the cache. A genuinely emptied account therefore keeps a stale local copy until
    /// something comes back — stale beats blank. All four `store_*_batch` functions apply this
    /// same guard, so the policy lives in one layer rather than at each call site.
    ///
    /// Widened to `pub` (from `pub(super)`) solely so integration tests under `tests/sync/`
    /// can drive it with fixtures; `perform_sync` remains the only production caller.
    pub async fn store_projects_batch(
        &self,
        storage: &LocalStorage,
        projects: &[crate::backend::BackendProject],
    ) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        if projects.is_empty() {
            return Ok(());
        }

        let txn = storage.conn.begin().await?;

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
            insert.exec(&txn).await?;
        }

        // Reconcile: anything this backend has locally that the remote no longer returns is
        // gone. A failed fetch aborts before reaching this point, so `projects` is always the
        // authoritative full list. This MUST run before the parent-relinking pass below: pass 1
        // just set every surviving row's `parent_uuid` to NULL, so nothing currently references
        // a project that is about to be deleted here. Running this after pass 2 instead would
        // let a still-fetched child project get re-linked to a parent that is being deleted in
        // this same call (e.g. the parent was completed/archived remotely), and
        // `ON DELETE CASCADE` on the self-referential parent relation would then destroy the
        // child too, even though the child was in the fetch. The project entity's parent FK has
        // no `on_delete` clause (defaults to `NO ACTION`), so in practice that ordering wouldn't
        // silently cascade like the task case below does -- the DELETE would fail an FK check and
        // roll back the whole transaction instead. Still wrong, so keep the ordering here too.
        let seen: Vec<String> = projects.iter().map(|p| p.remote_id.clone()).collect();
        project::Entity::delete_many()
            .filter(project::Column::BackendUuid.eq(self.backend_uuid))
            .filter(project::Column::RemoteId.is_not_in(seen))
            .exec(&txn)
            .await?;

        // Second pass: Update parent_uuid references to use local UUIDs
        for backend_project in projects {
            if let Some(remote_parent_id) = &backend_project.parent_remote_id {
                if let Some(parent) =
                    ProjectRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id).await?
                {
                    if let Some(project) =
                        ProjectRepository::get_by_remote_id(&txn, &self.backend_uuid, &backend_project.remote_id)
                            .await?
                    {
                        let mut active_model: project::ActiveModel = project.into();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        ProjectRepository::update(&txn, active_model).await?;
                    }
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    /// Store labels in batch
    ///
    /// An empty `labels` slice means "nothing to reconcile", not "the remote has nothing":
    /// `is_not_in(vec![])` matches every row, so an empty-but-successful fetch would otherwise
    /// blank the cache. A genuinely emptied account therefore keeps a stale local copy until
    /// something comes back — stale beats blank. All four `store_*_batch` functions apply this
    /// same guard, so the policy lives in one layer rather than at each call site.
    ///
    /// Widened to `pub` (from `pub(super)`) solely so integration tests under `tests/sync/`
    /// can drive it with fixtures; `perform_sync` remains the only production caller.
    pub async fn store_labels_batch(
        &self,
        storage: &LocalStorage,
        labels: &[crate::backend::BackendLabel],
    ) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        if labels.is_empty() {
            return Ok(());
        }

        let txn = storage.conn.begin().await?;

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
            insert.exec(&txn).await?;
        }

        // Reconcile: anything this backend has locally that the remote no longer returns is
        // gone. A failed fetch aborts before reaching this point, so `labels` is always the
        // authoritative full list.
        let seen: Vec<String> = labels.iter().map(|l| l.remote_id.clone()).collect();
        label::Entity::delete_many()
            .filter(label::Column::BackendUuid.eq(self.backend_uuid))
            .filter(label::Column::RemoteId.is_not_in(seen))
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    /// Store tasks in batch
    ///
    /// An empty `tasks` slice means "nothing to reconcile", not "the remote has nothing":
    /// `is_not_in(vec![])` matches every row, so an empty-but-successful fetch would otherwise
    /// blank the cache. A genuinely emptied account therefore keeps a stale local copy until
    /// something comes back — stale beats blank. All four `store_*_batch` functions apply this
    /// same guard, so the policy lives in one layer rather than at each call site.
    ///
    /// Widened to `pub` (from `pub(super)`) solely so integration tests under `tests/sync/`
    /// can drive it with fixtures; `perform_sync` remains the only production caller.
    pub async fn store_tasks_batch(&self, storage: &LocalStorage, tasks: &[crate::backend::BackendTask]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        if tasks.is_empty() {
            return Ok(());
        }

        let txn = storage.conn.begin().await?;

        // Everything below is driven by the tasks this call actually stores, never by the input
        // slice: the loop `continue`s over tasks whose project does not resolve locally, and a
        // skipped task must not be treated as seen (its stale row would survive the delete pass
        // with stale content and a stale `project_uuid`), must not be relinked, and must not
        // become the parent something else is relinked to.
        let mut task_labels_map: Vec<(Uuid, Vec<String>)> = Vec::new();
        let mut parent_links: Vec<(Uuid, String)> = Vec::new();
        let mut seen: Vec<String> = Vec::with_capacity(tasks.len());

        // First pass: Upsert all tasks without parent_uuid relationships
        for backend_task in tasks {
            let label_names = backend_task.labels.clone();

            // Look up local project UUID from remote project_id
            let project_uuid = match Self::lookup_project_uuid(
                &txn,
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
                Self::lookup_section_uuid(&txn, &self.backend_uuid, backend_task.section_remote_id.as_ref()).await?;

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
            insert.exec(&txn).await?;

            // Get the uuid of the task we just inserted/updated
            if let Some(task) =
                TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, &backend_task.remote_id).await?
            {
                seen.push(backend_task.remote_id.clone());
                task_labels_map.push((task.uuid, label_names));
                if let Some(remote_parent_id) = &backend_task.parent_remote_id {
                    parent_links.push((task.uuid, remote_parent_id.clone()));
                }
            }
        }

        // Reconcile: anything this backend has locally that the remote no longer returns is
        // gone. A failed fetch aborts before reaching this point, so `tasks` is always the
        // authoritative full list. `task_labels` rows for deleted tasks cascade automatically.
        //
        // This MUST run before the parent-relinking pass below, and it MUST run before that pass
        // for a specific reason: Todoist's fetch never returns completed tasks (see
        // `TodoistBackend::task_to_backend`), so completing a parent task makes it vanish from
        // `tasks` while its still-open subtasks remain. Pass 1 already set every surviving row's
        // `parent_uuid` to NULL, so at this point nothing references the parent that is about to
        // be deleted. If this delete ran after pass 2 instead, pass 2 would look the parent up
        // by remote_id against the stale (not yet reconciled) local row and re-link the subtask
        // to it, and the task entity's self-referential parent relation is `ON DELETE CASCADE`
        // -- deleting the parent would then destroy the subtask too, even though the subtask WAS
        // in the fetch. The subtask would reappear on the next sync as a brand-new INSERT with a
        // new uuid, breaking the uuid-stability invariant the UI now anchors selection to.
        task::Entity::delete_many()
            .filter(task::Column::BackendUuid.eq(self.backend_uuid))
            .filter(task::Column::RemoteId.is_not_in(seen))
            .exec(&txn)
            .await?;

        // Second pass: Update parent_uuid references to use local UUIDs. The parent lookup runs
        // against the post-delete state on purpose, so a parent that dropped out of the fetch
        // (completed remotely) or was skipped above simply leaves the child's parent NULL
        // instead of relinking it to a row that is gone.
        for (task_uuid, remote_parent_id) in parent_links {
            if let Some(parent) = TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, &remote_parent_id).await? {
                let active_model = task::ActiveModel {
                    uuid: ActiveValue::Unchanged(task_uuid),
                    parent_uuid: ActiveValue::Set(Some(parent.uuid)),
                    ..Default::default()
                };
                TaskRepository::update(&txn, active_model).await?;
            }
        }

        // Delete task-label relationships only for the tasks actually stored above
        for (task_uuid, _) in &task_labels_map {
            task_label::Entity::delete_many()
                .filter(task_label::Column::TaskUuid.eq(*task_uuid))
                .exec(&txn)
                .await?;
        }

        // Recreate relationships
        for (task_uuid, label_names) in task_labels_map {
            if !label_names.is_empty() {
                // Find label UUIDs by names
                for label_name in label_names {
                    if let Some(label) = LabelRepository::get_by_name(&txn, &label_name).await? {
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
                            .exec(&txn)
                            .await?;
                    }
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    /// Store sections in batch
    ///
    /// An empty `sections` slice means "nothing to reconcile", not "the remote has nothing":
    /// `is_not_in(vec![])` matches every row, so an empty-but-successful fetch would otherwise
    /// blank the cache. A genuinely emptied account therefore keeps a stale local copy until
    /// something comes back — stale beats blank. All four `store_*_batch` functions apply this
    /// same guard, so the policy lives in one layer rather than at each call site.
    ///
    /// Widened to `pub` (from `pub(super)`) solely so integration tests under `tests/sync/`
    /// can drive it with fixtures; `perform_sync` remains the only production caller.
    pub async fn store_sections_batch(
        &self,
        storage: &LocalStorage,
        sections: &[crate::backend::BackendSection],
    ) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        if sections.is_empty() {
            return Ok(());
        }

        let txn = storage.conn.begin().await?;

        for backend_section in sections {
            // Look up local project UUID from remote project_id
            let project_uuid = Self::lookup_project_uuid(
                &txn,
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
            insert.exec(&txn).await?;
        }

        // Reconcile: anything this backend has locally that the remote no longer returns is
        // gone. A failed fetch aborts before reaching this point, so `sections` is always the
        // authoritative full list, and the empty case returned early above.
        let seen: Vec<String> = sections.iter().map(|s| s.remote_id.clone()).collect();
        section::Entity::delete_many()
            .filter(section::Column::BackendUuid.eq(self.backend_uuid))
            .filter(section::Column::RemoteId.is_not_in(seen))
            .exec(&txn)
            .await?;

        txn.commit().await?;
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
