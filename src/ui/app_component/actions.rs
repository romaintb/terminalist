//! App-level action handling: the arms that need business logic rather than a component's
//! own state. `handle_event` routes an event to a component, the component turns it into an
//! `Action`, and whatever the component hierarchy does not consume lands here.

use super::AppComponent;
use crate::constants::*;
use crate::sync::SyncStatus;
use crate::ui::components::toast::Toast;
use crate::ui::core::actions::Action;
use crate::ui::core::operations::{Due, Operation};
use crate::ui::core::{LoadKind, SidebarSelection};
use log::info;

impl AppComponent {
    /// Handle app-level actions that require business logic
    pub async fn handle_app_action(&mut self, action: Action) -> Action {
        match action {
            Action::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                Action::None
            }
            Action::Quit => {
                self.should_quit = true;
                Action::None
            }
            Action::StartSync => {
                if self.active_sync_task.is_none() {
                    info!("Starting background sync");
                    self.state.loading = true;
                    self.start_background_sync();
                } else {
                    info!("Sync already in progress, ignoring");
                }
                Action::None
            }
            Action::RefreshLocalData => {
                info!("Refreshing local data from database (debug mode)");
                // Schedule a data fetch directly from local storage without API sync
                self.schedule_data_load(LoadKind::User);
                Action::None
            }
            Action::SyncCompleted(status) => {
                info!("Sync: Completed with status {:?}", status);
                self.active_sync_task = None;
                self.state.loading = false;

                match status {
                    SyncStatus::Success => {
                        self.update_data_from_sync(SyncStatus::Success);
                        self.sync_component_data();
                        self.toast = Some(Toast::success(SUCCESS_SYNC_COMPLETED, &self.config.theme));
                        Action::None
                    }
                    SyncStatus::Error { message } => {
                        self.is_initial_sync = false;
                        self.toast = Some(Toast::error(&message, &self.config.theme));
                        Action::None
                    }
                    SyncStatus::Idle | SyncStatus::InProgress => Action::None,
                }
            }
            Action::SyncFailed(error) => {
                info!("Sync: Failed with error: {}", error);
                self.active_sync_task = None;
                self.state.loading = false;
                self.is_initial_sync = false; // Reset flag on failure
                self.toast = Some(Toast::error(&error, &self.config.theme));
                Action::None
            }
            Action::ShowDialog(ref dialog_type) => {
                info!("Dialog: Showing dialog {:?}", dialog_type);
                // Dialog component will handle the actual dialog setup
                action
            }
            Action::HideDialog => {
                info!("Dialog: Hiding current dialog");
                // Dialog component will handle hiding
                action
            }
            Action::NavigateToSidebar(selection) => {
                // Create a more detailed log message with names
                let selection_desc = match &selection {
                    SidebarSelection::Today => "Today".to_string(),
                    SidebarSelection::Tomorrow => "Tomorrow".to_string(),
                    SidebarSelection::Upcoming => "Upcoming".to_string(),
                    SidebarSelection::Project(uuid) => match self.state.projects.iter().find(|p| p.uuid == *uuid) {
                        Some(project) => format!("Project {uuid} '{}'", project.name),
                        None => format!("Project {uuid} [unknown]"),
                    },
                    SidebarSelection::Label(uuid) => match self.state.labels.iter().find(|l| l.uuid == *uuid) {
                        Some(label) => format!("Label {uuid} '{}'", label.name),
                        None => format!("Label {uuid} [unknown]"),
                    },
                };

                info!("Navigation: Sidebar selection changed to {}", selection_desc);
                // Once the user has picked a view, the initial sync no longer owns the
                // selection: completing it must not drag them back to `default_project`.
                self.is_initial_sync = false;
                self.state.sidebar_selection = selection;
                // Reload data for the new selection
                self.schedule_data_load(LoadKind::User);
                info!("Navigation: Scheduled data fetch for new selection");
                Action::None
            }
            // Task operations with background execution
            Action::CreateTask { content, project_uuid } => {
                match project_uuid {
                    Some(uuid) => info!("Task: Creating '{}' in project {}", content, uuid),
                    None => info!("Task: Creating '{}' in inbox", content),
                }
                self.spawn(Operation::CreateTask {
                    content,
                    project: project_uuid,
                });
                Action::None
            }
            Action::CompleteTask(task) => {
                // Kept from before the operations rework: a task storage does not know about
                // is not sent. See the note in the PR about that silently doing nothing.
                let existing = self.sync_service.get_task_by_id(&task).await;
                match existing {
                    Ok(Some(found)) => {
                        info!("Task: Completing ID {} '{}'", task, found.content);
                        // The backend completes subtasks along with their parent.
                        self.spawn(Operation::CompleteTask(task));
                    }
                    Ok(None) => info!("Task: Cannot complete, task {} not found", task),
                    Err(e) => info!("Task: Cannot complete task {}: {}", task, e),
                }
                Action::None
            }
            Action::CyclePriority(task) => {
                // The current priority has to come from storage: state.tasks holds whatever
                // the active view last loaded, which is not necessarily this task.
                let existing = self.sync_service.get_task_by_id(&task).await;
                match existing {
                    Ok(Some(current)) => {
                        // 1 (Normal) through 4 (Highest), then back around.
                        let priority = if current.priority == 4 { 1 } else { current.priority + 1 };
                        info!(
                            "Task: Cycling priority for {} (P{} -> P{})",
                            self.describe_task(task),
                            current.priority,
                            priority
                        );
                        self.spawn(Operation::CyclePriority { task, priority });
                    }
                    Ok(None) => info!("Task: Cannot cycle priority, task {} not found", task),
                    Err(e) => info!("Task: Cannot cycle priority for task {}: {}", task, e),
                }
                Action::None
            }
            Action::DeleteTask(task) => {
                info!("Task: Deleting {}", self.describe_task(task));
                self.spawn(Operation::DeleteTask(task));
                Action::None
            }
            Action::SetTaskDueToday(task) => self.set_due(task, Due::Today),
            Action::SetTaskDueTomorrow(task) => self.set_due(task, Due::Tomorrow),
            Action::SetTaskDueNextWeek(task) => self.set_due(task, Due::NextWeek),
            Action::SetTaskDueWeekEnd(task) => self.set_due(task, Due::Weekend),
            Action::EditTask { task_uuid, content } => {
                info!("Task: Editing {} to '{}'", self.describe_task(task_uuid), content);
                self.spawn(Operation::EditTask {
                    task: task_uuid,
                    content,
                });
                Action::None
            }
            Action::RestoreTask(task) => {
                info!("Task: Restoring {}", self.describe_task(task));
                self.spawn(Operation::RestoreTask(task));
                Action::None
            }
            Action::CreateProject { name, parent_uuid } => {
                match parent_uuid {
                    Some(uuid) => info!("Project: Creating '{}' under {}", name, uuid),
                    None => info!("Project: Creating '{}' at root", name),
                }
                self.spawn(Operation::CreateProject {
                    name,
                    parent: parent_uuid,
                });
                Action::None
            }
            Action::DeleteProject(project) => {
                info!("Project: Deleting {}", self.describe_project(project));
                self.spawn(Operation::DeleteProject(project));
                Action::None
            }
            Action::DeleteLabel(label) => {
                info!("Label: Deleting {}", self.describe_label(label));
                self.spawn(Operation::DeleteLabel(label));
                Action::None
            }
            Action::CreateLabel { name } => {
                info!("Label: Creating '{}'", name);
                self.spawn(Operation::CreateLabel { name });
                Action::None
            }
            Action::EditProject { project_uuid, name } => {
                info!("Project: Editing {} -> '{}'", self.describe_project(project_uuid), name);
                self.spawn(Operation::EditProject {
                    project: project_uuid,
                    name,
                });
                Action::None
            }
            Action::EditLabel { label_uuid, name } => {
                info!("Label: Editing {} -> '{}'", self.describe_label(label_uuid), name);
                self.spawn(Operation::EditLabel {
                    label: label_uuid,
                    name,
                });
                Action::None
            }
            Action::DataLoaded {
                kind,
                projects,
                labels,
                sections,
                tasks,
            } => {
                info!(
                    "Data: Loaded {} projects, {} labels, {} sections, {} tasks ({:?})",
                    projects.len(),
                    labels.len(),
                    sections.len(),
                    tasks.len(),
                    kind
                );

                // Read before the rebuild: the task list still holds the pre-reload items.
                let anchor = match kind {
                    LoadKind::Background => self.task_list.get_selected_task().map(|task| task.uuid),
                    _ => None,
                };

                self.state.update_data(projects, labels, sections, tasks);

                if kind == LoadKind::Initial {
                    // `default_project` is only resolvable post-load.
                    self.set_initial_sidebar_selection();
                    self.schedule_data_load(LoadKind::User);
                } else if !self.state.selection_is_live() {
                    // The project or label being viewed was deleted from another client.
                    info!("Navigation: selection no longer exists, falling back to Today");
                    self.state.sidebar_selection = SidebarSelection::Today;
                    self.schedule_data_load(LoadKind::User);
                }

                self.sync_component_data();

                if let Some(task_uuid) = anchor {
                    self.task_list.select_task(task_uuid);
                }
                Action::None
            }
            Action::SearchTasks(query) => {
                info!("Search: Starting database search for '{}'", query);
                let sync_service = self.sync_service.clone();
                let _task_id = self.task_manager.spawn_task_search(sync_service, query);
                Action::None
            }
            Action::SearchResultsLoaded { query, results } => {
                info!("Search: Loaded {} results for query '{}'", results.len(), query);
                // Update dialog with search results
                self.dialog.update_search_results(&query, results);
                Action::None
            }
            Action::NextTask => {
                info!("Navigation: Next task (j/down)");
                action
            }
            Action::PreviousTask => {
                info!("Navigation: Previous task (k/up)");
                action
            }
            Action::RefreshData => {
                info!("Data: Refreshing UI data after task operation");
                // Schedule a data fetch to reload current view with updated data
                self.schedule_data_load(LoadKind::User);
                Action::None
            }
            // Help panel scrolling actions
            Action::HelpScrollUp => {
                if self.state.help_scroll_offset > 0 {
                    self.state.help_scroll_offset -= 1;
                }
                info!("Help: Scrolled up, offset now {}", self.state.help_scroll_offset);
                Action::None
            }
            Action::HelpScrollDown => {
                self.state.help_scroll_offset += 1;
                info!("Help: Scrolled down, offset now {}", self.state.help_scroll_offset);
                Action::None
            }
            Action::HelpScrollToTop => {
                self.state.help_scroll_offset = 0;
                info!("Help: Scrolled to top");
                Action::None
            }
            Action::HelpScrollToBottom => {
                // Set to a large value - dialog component will handle bounds checking
                self.state.help_scroll_offset = usize::MAX;
                info!("Help: Scrolled to bottom");
                Action::None
            }
            Action::ShowHelp(show) => {
                self.state.show_help = show;
                if !show {
                    // Reset scroll when hiding help
                    self.state.help_scroll_offset = 0;
                }
                info!("Help: {} help panel", if show { "Showing" } else { "Hiding" });
                action
            }
            // Pass through other actions
            _ => action,
        }
    }
}
