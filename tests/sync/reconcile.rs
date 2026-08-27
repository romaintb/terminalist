//! Reconciliation tests for the `store_*_batch` functions in `src/sync/storage.rs`.
//!
//! These drive `SyncService::store_projects_batch` / `store_labels_batch` /
//! `store_sections_batch` / `store_tasks_batch` directly with fixture data (bypassing the
//! network-fetching `SyncService::sync()`), then assert against the repositories. Every test
//! opens storage via `LocalStorage::new_at` in a fresh `tempfile` directory — never a user path.

use sea_orm::{EntityTrait, PaginatorTrait};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use terminalist::backend::{BackendLabel, BackendProject, BackendSection, BackendTask};
use terminalist::backend_registry::BackendRegistry;
use terminalist::entities::task_label;
use terminalist::repositories::{LabelRepository, ProjectRepository, SectionRepository, TaskRepository};
use terminalist::storage::LocalStorage;
use terminalist::sync::SyncService;

/// Open a fresh local cache, register a fake "todoist" backend for it (no network calls: the
/// underlying `TodoistWrapper::new` only builds an HTTP client, it never dials out), and return
/// a ready-to-use `SyncService` alongside the storage handle and the backend's UUID.
async fn new_sync_service(dir: &Path) -> (SyncService, Arc<Mutex<LocalStorage>>, Uuid) {
    let storage = LocalStorage::new_at(dir).await.expect("open local storage");
    let storage = Arc::new(Mutex::new(storage));

    let registry = Arc::new(BackendRegistry::new(storage.clone()));
    let backend_uuid = registry
        .add_backend(
            "todoist".to_string(),
            "reconcile-test-backend".to_string(),
            r#"{"api_token":"fake-token"}"#.to_string(),
            "{}".to_string(),
        )
        .await
        .expect("register fake backend");

    let sync_service = SyncService::new(registry, backend_uuid, false)
        .await
        .expect("construct sync service");

    (sync_service, storage, backend_uuid)
}

fn project(remote_id: &str, name: &str) -> BackendProject {
    BackendProject {
        remote_id: remote_id.to_string(),
        name: name.to_string(),
        is_favorite: false,
        is_inbox: false,
        order_index: 0,
        parent_remote_id: None,
    }
}

fn project_with_parent(remote_id: &str, name: &str, parent_remote_id: &str) -> BackendProject {
    BackendProject {
        parent_remote_id: Some(parent_remote_id.to_string()),
        ..project(remote_id, name)
    }
}

fn label(remote_id: &str, name: &str) -> BackendLabel {
    BackendLabel {
        remote_id: remote_id.to_string(),
        name: name.to_string(),
        order_index: 0,
        is_favorite: false,
    }
}

fn section(remote_id: &str, project_remote_id: &str, name: &str) -> BackendSection {
    BackendSection {
        remote_id: remote_id.to_string(),
        name: name.to_string(),
        project_remote_id: project_remote_id.to_string(),
        order_index: 0,
    }
}

fn task(remote_id: &str, project_remote_id: &str, content: &str, labels: Vec<String>) -> BackendTask {
    BackendTask {
        remote_id: remote_id.to_string(),
        content: content.to_string(),
        description: None,
        project_remote_id: project_remote_id.to_string(),
        section_remote_id: None,
        parent_remote_id: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        labels,
    }
}

fn task_with_parent(remote_id: &str, project_remote_id: &str, content: &str, parent_remote_id: &str) -> BackendTask {
    BackendTask {
        parent_remote_id: Some(parent_remote_id.to_string()),
        ..task(remote_id, project_remote_id, content, vec![])
    }
}

fn remote_ids<T>(rows: &[T], f: impl Fn(&T) -> &str) -> Vec<&str> {
    rows.iter().map(f).collect()
}

// 1. A row absent from the second fetch is deleted.
#[tokio::test]
async fn project_missing_from_second_sync_is_deleted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox"), project("p2", "Work")])
        .await
        .expect("first sync");
    drop(guard);

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("second sync");
    let remaining = ProjectRepository::get_all(&guard.conn).await.expect("query projects");
    drop(guard);

    let ids = remote_ids(&remaining, |p| p.remote_id.as_str());
    assert_eq!(ids, vec!["p1"], "project p2 should have been deleted, got {ids:?}");
}

// 1b. ...but an EMPTY fetch is not treated as "the remote has nothing". `is_not_in(vec![])`
// matches every row, so without a guard a transient empty-but-200 response would blank the
// user's list (and cascade their tasks away with it) until a later sync repaired it. Each
// `store_*_batch` treats an empty slice as "nothing to reconcile" and returns early; that is
// the single layer where this policy lives, so all four entity types agree.
//
// The accepted trade: an account whose last project/label/task is genuinely deleted keeps a
// stale local copy until something else comes back. Stale beats blank.
#[tokio::test]
async fn empty_fetch_leaves_existing_rows_untouched() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox"), project("p2", "Work")])
        .await
        .expect("store projects");
    sync_service
        .store_labels_batch(&guard, &[label("l1", "urgent")])
        .await
        .expect("store labels");
    sync_service
        .store_sections_batch(&guard, &[section("s1", "p1", "Backlog")])
        .await
        .expect("store sections");
    sync_service
        .store_tasks_batch(&guard, &[task("t1", "p1", "Buy milk", vec!["urgent".to_string()])])
        .await
        .expect("store tasks");
    drop(guard);

    // Every fetch comes back empty — the shape of a transient backend hiccup that still
    // answers 200.
    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[])
        .await
        .expect("empty project fetch");
    sync_service.store_labels_batch(&guard, &[]).await.expect("empty label fetch");
    sync_service
        .store_sections_batch(&guard, &[])
        .await
        .expect("empty section fetch");
    sync_service.store_tasks_batch(&guard, &[]).await.expect("empty task fetch");

    let projects = ProjectRepository::get_all(&guard.conn).await.expect("query projects");
    let labels = LabelRepository::get_all(&guard.conn).await.expect("query labels");
    let sections = SectionRepository::get_all(&guard.conn).await.expect("query sections");
    let tasks = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    drop(guard);

    assert_eq!(
        remote_ids(&projects, |p| p.remote_id.as_str()),
        vec!["p1", "p2"],
        "an empty fetch must not wipe the cached projects"
    );
    assert_eq!(
        remote_ids(&labels, |l| l.remote_id.as_str()),
        vec!["l1"],
        "an empty fetch must not wipe the cached labels"
    );
    assert_eq!(
        remote_ids(&sections, |s| s.remote_id.as_str()),
        vec!["s1"],
        "an empty fetch must not wipe the cached sections"
    );
    assert_eq!(
        remote_ids(&tasks, |t| t.remote_id.as_str()),
        vec!["t1"],
        "an empty fetch must not wipe the cached tasks"
    );
}

// 2. A row present in both is updated, not duplicated, and KEEPS its local uuid.
#[tokio::test]
async fn project_present_in_both_syncs_is_updated_not_duplicated_and_keeps_uuid() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Old Name")])
        .await
        .expect("first sync");
    let original = ProjectRepository::get_by_remote_id(&guard.conn, &backend_uuid, "p1")
        .await
        .expect("query project")
        .expect("project p1 stored after first sync");
    drop(guard);

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "New Name")])
        .await
        .expect("second sync");
    let all = ProjectRepository::get_all(&guard.conn).await.expect("query projects");
    drop(guard);

    assert_eq!(all.len(), 1, "expected exactly one project row, got {all:?}");
    assert_eq!(all[0].remote_id, "p1");
    assert_eq!(all[0].uuid, original.uuid, "local uuid must survive the update");
    assert_eq!(all[0].name, "New Name", "the update should still apply");
}

// 3. Deleting a project removes its tasks but not another project's tasks.
#[tokio::test]
async fn deleting_a_project_cascades_its_tasks_but_not_another_projects_tasks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Project One"), project("p2", "Project Two")])
        .await
        .expect("store projects");
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("t1", "p1", "Task in project one", vec![]),
                task("t2", "p2", "Task in project two", vec![]),
            ],
        )
        .await
        .expect("store tasks");
    drop(guard);

    // Second project sync: p1 is gone remotely, p2 survives. We never re-sync tasks here, so
    // any task-list survivorship is caused purely by the project delete pass's FK cascade.
    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p2", "Project Two")])
        .await
        .expect("second project sync");
    let remaining_projects = ProjectRepository::get_all(&guard.conn).await.expect("query projects");
    let remaining_tasks = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    drop(guard);

    let project_ids = remote_ids(&remaining_projects, |p| p.remote_id.as_str());
    assert_eq!(
        project_ids,
        vec!["p2"],
        "project p1 should have been deleted, got {project_ids:?}"
    );

    let task_ids = remote_ids(&remaining_tasks, |t| t.remote_id.as_str());
    assert_eq!(
        task_ids,
        vec!["t2"],
        "task t1 should have cascade-deleted with project p1, and task t2 (under surviving \
         project p2) should be untouched, got {task_ids:?}"
    );
}

// 4. Syncing the same fixture twice leaves exactly one copy of every row.
#[tokio::test]
async fn syncing_the_same_fixture_twice_leaves_one_copy_of_every_row() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let projects = [project("p1", "Inbox")];
    let labels = [label("l1", "urgent")];
    let sections = [section("s1", "p1", "Backlog")];
    let tasks = [task("t1", "p1", "Buy milk", vec!["urgent".to_string()])];

    for _ in 0..2 {
        let guard = storage.lock().await;
        sync_service
            .store_projects_batch(&guard, &projects)
            .await
            .expect("store projects");
        sync_service.store_labels_batch(&guard, &labels).await.expect("store labels");
        sync_service
            .store_sections_batch(&guard, &sections)
            .await
            .expect("store sections");
        sync_service.store_tasks_batch(&guard, &tasks).await.expect("store tasks");
    }

    let guard = storage.lock().await;
    let all_projects = ProjectRepository::get_all(&guard.conn).await.expect("query projects");
    let all_labels = LabelRepository::get_all(&guard.conn).await.expect("query labels");
    let all_sections = SectionRepository::get_all(&guard.conn).await.expect("query sections");
    let all_tasks = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    let task_label_count = task_label::Entity::find().count(&guard.conn).await.expect("count task_labels");
    drop(guard);

    assert_eq!(remote_ids(&all_projects, |p| p.remote_id.as_str()), vec!["p1"]);
    assert_eq!(remote_ids(&all_labels, |l| l.remote_id.as_str()), vec!["l1"]);
    assert_eq!(remote_ids(&all_sections, |s| s.remote_id.as_str()), vec!["s1"]);
    assert_eq!(remote_ids(&all_tasks, |t| t.remote_id.as_str()), vec!["t1"]);
    assert_eq!(task_label_count, 1, "exactly one task-label relationship should remain");
}

// Extra: the label delete pass mirrors the project one.
#[tokio::test]
async fn label_missing_from_second_sync_is_deleted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_labels_batch(&guard, &[label("l1", "urgent"), label("l2", "later")])
        .await
        .expect("first sync");
    drop(guard);

    let guard = storage.lock().await;
    sync_service
        .store_labels_batch(&guard, &[label("l1", "urgent")])
        .await
        .expect("second sync");
    let remaining = LabelRepository::get_all(&guard.conn).await.expect("query labels");
    drop(guard);

    let ids = remote_ids(&remaining, |l| l.remote_id.as_str());
    assert_eq!(ids, vec!["l1"], "label l2 should have been deleted, got {ids:?}");
}

// Extra: the section delete pass mirrors the project one. (A *non-empty* fetch that omits a
// section deletes it; an empty one is guarded — see `empty_fetch_leaves_existing_rows_untouched`.)
#[tokio::test]
async fn section_missing_from_second_sync_is_deleted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("store project");
    sync_service
        .store_sections_batch(&guard, &[section("s1", "p1", "Backlog"), section("s2", "p1", "Doing")])
        .await
        .expect("first sync");
    drop(guard);

    let guard = storage.lock().await;
    sync_service
        .store_sections_batch(&guard, &[section("s1", "p1", "Backlog")])
        .await
        .expect("second sync");
    let remaining = SectionRepository::get_all(&guard.conn).await.expect("query sections");
    drop(guard);

    let ids = remote_ids(&remaining, |s| s.remote_id.as_str());
    assert_eq!(ids, vec!["s1"], "section s2 should have been deleted, got {ids:?}");
}

// Extra: deleting a task directly (not via a project cascade) also removes its task_labels rows.
#[tokio::test]
async fn task_missing_from_second_sync_is_deleted_and_its_label_link_goes_with_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("store project");
    sync_service
        .store_labels_batch(&guard, &[label("l1", "urgent")])
        .await
        .expect("store label");
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("t1", "p1", "Keep me", vec![]),
                task("t2", "p1", "Delete me", vec!["urgent".to_string()]),
            ],
        )
        .await
        .expect("first sync");
    drop(guard);

    let guard = storage.lock().await;
    sync_service
        .store_tasks_batch(&guard, &[task("t1", "p1", "Keep me", vec![])])
        .await
        .expect("second sync");
    let remaining_tasks = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    let task_label_count = task_label::Entity::find().count(&guard.conn).await.expect("count task_labels");
    drop(guard);

    let ids = remote_ids(&remaining_tasks, |t| t.remote_id.as_str());
    assert_eq!(ids, vec!["t1"], "task t2 should have been deleted, got {ids:?}");
    assert_eq!(
        task_label_count, 0,
        "deleting task t2 should cascade-remove its task_labels row"
    );
}

// Regression (fix round 1): the delete-missing pass must run BEFORE the parent-relinking pass,
// not after. Todoist's fetch never returns completed tasks (see
// `TodoistBackend::task_to_backend`'s `is_completed: false` comment), so completing a parent
// task makes it vanish from the very next fetch while its still-open subtasks remain. Pass 1
// nulls every surviving row's `parent_uuid`; if the delete pass ran after pass 2 instead, pass 2
// would re-link the still-fetched subtask to the stale (not yet reconciled) parent row, and
// `ON DELETE CASCADE` on the task entity's self-referential parent relation would then destroy
// the subtask along with its parent -- even though the subtask WAS in the fetch. It would
// reappear on the next sync as a fresh INSERT with a NEW uuid, violating the uuid-stability
// invariant the UI anchors selection to.
#[tokio::test]
async fn completing_a_parent_task_does_not_cascade_delete_its_still_fetched_subtask() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("store project");
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("parent", "p1", "Parent task", vec![]),
                task_with_parent("child", "p1", "Child task", "parent"),
            ],
        )
        .await
        .expect("first sync");
    let child_before = TaskRepository::get_by_remote_id(&guard.conn, &backend_uuid, "child")
        .await
        .expect("query child task")
        .expect("child task stored after first sync");
    drop(guard);

    // Second sync: the parent was completed remotely, so it drops out of the fetch. The child
    // is still open and still names "parent" as its parent, exactly as Todoist would report it.
    let guard = storage.lock().await;
    sync_service
        .store_tasks_batch(&guard, &[task_with_parent("child", "p1", "Child task", "parent")])
        .await
        .expect("second sync");
    let remaining_tasks = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    drop(guard);

    let ids = remote_ids(&remaining_tasks, |t| t.remote_id.as_str());
    assert_eq!(
        ids,
        vec!["child"],
        "parent task should be gone (it was completed / dropped from the fetch) but the still-fetched \
         child must survive, got {ids:?}"
    );
    assert_eq!(
        remaining_tasks[0].uuid, child_before.uuid,
        "the surviving child task must keep its original local uuid, not be recreated"
    );
}

// Sibling of the above for projects: a parent project dropping out of the fetch (e.g. archived
// remotely) must not take a still-fetched child project down with it. The project entity's
// self-referential parent relation has no `on_delete` clause (defaults to `NO ACTION`), so
// getting the ordering wrong here fails the whole transaction with an FK violation rather than
// silently cascading -- fails safe, but still wrong, and this pins the correct ordering directly.
#[tokio::test]
async fn removing_a_parent_project_does_not_cascade_delete_its_still_fetched_child_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(
            &guard,
            &[
                project("parent", "Parent Project"),
                project_with_parent("child", "Child Project", "parent"),
            ],
        )
        .await
        .expect("first sync");
    let child_before = ProjectRepository::get_by_remote_id(&guard.conn, &backend_uuid, "child")
        .await
        .expect("query child project")
        .expect("child project stored after first sync");
    drop(guard);

    // Second sync: the parent project is gone from the fetch, but the child is still there and
    // still names "parent" as its parent.
    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project_with_parent("child", "Child Project", "parent")])
        .await
        .expect("second sync");
    let remaining_projects = ProjectRepository::get_all(&guard.conn).await.expect("query projects");
    drop(guard);

    let ids = remote_ids(&remaining_projects, |p| p.remote_id.as_str());
    assert_eq!(
        ids,
        vec!["child"],
        "parent project should be gone but the still-fetched child must survive, got {ids:?}"
    );
    assert_eq!(
        remaining_projects[0].uuid, child_before.uuid,
        "the surviving child project must keep its original local uuid, not be recreated"
    );
}

// Regression (fix round 2): a task whose project does not resolve locally is skipped by the
// upsert loop, so the delete pass must not treat it as "seen". Building the keep-set from the
// input slice let a skipped task's pre-existing row survive with stale content and a stale
// `project_uuid` — a row the sync claimed to have written but never touched.
#[tokio::test]
async fn a_task_skipped_because_its_project_is_missing_does_not_keep_a_stale_row() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, _backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("store project");
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("t1", "p1", "Original content", vec![]),
                task("t2", "p1", "Untouched", vec![]),
            ],
        )
        .await
        .expect("first sync");
    drop(guard);

    // Second sync: t1 now claims a project that was never stored locally (the free-tier case the
    // `continue` exists for), so it is skipped and never upserted.
    let guard = storage.lock().await;
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("t1", "p-not-local", "Updated content", vec![]),
                task("t2", "p1", "Untouched", vec![]),
            ],
        )
        .await
        .expect("second sync");
    let remaining = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    drop(guard);

    let ids = remote_ids(&remaining, |t| t.remote_id.as_str());
    assert_eq!(
        ids,
        vec!["t2"],
        "a task the upsert loop skipped must not survive the delete pass with stale content, got {remaining:?}"
    );
}

// Sibling of the above for the parent-relinking pass: that pass must be driven by the tasks
// actually stored, not by the input slice, so a skipped task can neither be relinked itself nor
// become the parent a surviving task is relinked to.
#[tokio::test]
async fn a_skipped_task_is_never_relinked_as_a_parent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, backend_uuid) = new_sync_service(tmp.path()).await;

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("store project");
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("parent", "p1", "Parent task", vec![]),
                task_with_parent("child", "p1", "Child task", "parent"),
            ],
        )
        .await
        .expect("first sync");
    drop(guard);

    // The parent moved to a project that is not cached locally, so it is skipped; the child is
    // still fetched and still names it as its parent.
    let guard = storage.lock().await;
    sync_service
        .store_tasks_batch(
            &guard,
            &[
                task("parent", "p-not-local", "Parent task", vec![]),
                task_with_parent("child", "p1", "Child task", "parent"),
            ],
        )
        .await
        .expect("second sync");
    let remaining = TaskRepository::get_all(&guard.conn).await.expect("query tasks");
    let child = TaskRepository::get_by_remote_id(&guard.conn, &backend_uuid, "child")
        .await
        .expect("query child")
        .expect("child survives");
    drop(guard);

    let ids = remote_ids(&remaining, |t| t.remote_id.as_str());
    assert_eq!(
        ids,
        vec!["child"],
        "the skipped parent must not survive, and the fetched child must, got {ids:?}"
    );
    assert_eq!(
        child.parent_uuid, None,
        "the child must not be relinked to a parent row that no longer exists"
    );
}

// The positive case for the parent-relinking pass, pinned explicitly because every other test
// around it asserts the *absence* of a link: a subtask must actually end up pointing at its
// parent's local uuid, and must still point at it after a second identical sync (pass 1 nulls
// `parent_uuid` on every surviving row, so pass 2 has to re-establish the link each time).
#[tokio::test]
async fn a_subtask_is_linked_to_its_parents_local_uuid() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (sync_service, storage, backend_uuid) = new_sync_service(tmp.path()).await;

    let fixture = [
        task("parent", "p1", "Parent task", vec![]),
        task_with_parent("child", "p1", "Child task", "parent"),
    ];

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Inbox")])
        .await
        .expect("store project");
    drop(guard);

    for pass in 1..=2 {
        let guard = storage.lock().await;
        sync_service.store_tasks_batch(&guard, &fixture).await.expect("store tasks");
        let parent = TaskRepository::get_by_remote_id(&guard.conn, &backend_uuid, "parent")
            .await
            .expect("query parent")
            .expect("parent stored");
        let child = TaskRepository::get_by_remote_id(&guard.conn, &backend_uuid, "child")
            .await
            .expect("query child")
            .expect("child stored");
        drop(guard);

        assert_eq!(
            child.parent_uuid,
            Some(parent.uuid),
            "the child must be linked to its parent's local uuid on sync pass {pass}"
        );
    }
}
