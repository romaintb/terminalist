//! Global key bindings: the last stop for a key event, after the dialog, the sidebar
//! and the task list have each had their turn in `handle_event`.

use super::AppComponent;
use crate::constants::UI_NO_TASK_SELECTED_DUE_DATE;
use crate::entities::{label, project};
use crate::ui::core::actions::{Action, DialogType};
use crate::ui::core::SidebarSelection;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use log::info;

/// What the sidebar is pointing at, resolved to the item itself. Both 'D' and 'E' need
/// the same five-way answer and differ only in what they do with it.
enum Selected<'a> {
    Project(&'a project::Model),
    Label(&'a label::Model),
    /// A built-in view, named as it appears in "Cannot delete the {} view".
    View(&'static str),
    /// A project or label slot whose index no longer resolves. Names which one.
    Missing(&'static str),
}

impl AppComponent {
    fn selected(&self) -> Selected<'_> {
        match &self.state.sidebar_selection {
            SidebarSelection::Today => Selected::View("Today"),
            SidebarSelection::Tomorrow => Selected::View("Tomorrow"),
            SidebarSelection::Upcoming => Selected::View("Upcoming"),
            SidebarSelection::Project(index) => match self.state.projects.get(*index) {
                Some(project) => Selected::Project(project),
                None => Selected::Missing("project"),
            },
            SidebarSelection::Label(index) => match self.state.labels.get(*index) {
                Some(label) => Selected::Label(label),
                None => Selected::Missing("label"),
            },
        }
    }
    pub(super) fn handle_global_key(&mut self, key: KeyEvent) -> Action {
        // Handle help panel scrolling when help is open
        if self.state.show_help {
            match key.code {
                KeyCode::Up => return Action::HelpScrollUp,
                KeyCode::Down => return Action::HelpScrollDown,
                KeyCode::Home => return Action::HelpScrollToTop,
                KeyCode::End => return Action::HelpScrollToBottom,
                KeyCode::Char('?') | KeyCode::Esc => return Action::ShowHelp(false),
                _ => {} // Continue to other key handling
            }
        }

        match key.code {
            KeyCode::Char('b') => {
                info!("Global key: 'b' - toggling sidebar visibility");
                Action::ToggleSidebar
            }
            KeyCode::Char('q') => {
                info!("Global key: 'q' - quitting application");
                Action::Quit
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                info!("Global key: Ctrl+C - quitting application");
                Action::Quit
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                info!("Global key: '?' or 'h' - opening help dialog");
                Action::ShowDialog(DialogType::Help)
            }
            KeyCode::Char('G') => {
                info!("Global key: 'G' - opening logs dialog");
                Action::ShowDialog(DialogType::Logs)
            }
            KeyCode::Char('A') => {
                info!("Global key: 'A' - opening project creation dialog");
                Action::ShowDialog(DialogType::ProjectCreation)
            }
            KeyCode::Char('D') => match self.selected() {
                Selected::Project(project) => {
                    info!(
                        "Global key: 'D' - deleting project '{}' (ID: {})",
                        project.name, project.uuid
                    );
                    Action::ShowDialog(DialogType::DeleteConfirmation {
                        item_type: "project".to_string(),
                        item_uuid: project.uuid,
                    })
                }
                Selected::Label(label) => {
                    info!("Global key: 'D' - deleting label '{}' (ID: {})", label.name, label.uuid);
                    Action::ShowDialog(DialogType::DeleteConfirmation {
                        item_type: "label".to_string(),
                        item_uuid: label.uuid,
                    })
                }
                Selected::View(view) => {
                    info!("Global key: 'D' - cannot delete {} view", view);
                    Action::ShowDialog(DialogType::Info(format!("Cannot delete the {view} view")))
                }
                Selected::Missing(kind) => {
                    info!("Global key: 'D' - no {} selected (invalid index)", kind);
                    Action::ShowDialog(DialogType::Error(format!("No {kind} selected to delete")))
                }
            },
            KeyCode::Char('E') => match self.selected() {
                Selected::Project(project) => {
                    info!(
                        "Global key: 'E' - editing project '{}' (ID: {})",
                        project.name, project.uuid
                    );
                    Action::ShowDialog(DialogType::ProjectEdit {
                        project_uuid: project.uuid,
                        name: project.name.clone(),
                    })
                }
                Selected::Label(label) => {
                    info!("Global key: 'E' - editing label '{}' (ID: {})", label.name, label.uuid);
                    Action::ShowDialog(DialogType::LabelEdit {
                        label_uuid: label.uuid,
                        name: label.name.clone(),
                    })
                }
                Selected::View(view) => {
                    info!("Global key: 'E' - cannot edit {} view", view);
                    Action::ShowDialog(DialogType::Info(format!("Cannot edit the {view} view")))
                }
                Selected::Missing(kind) => {
                    info!("Global key: 'E' - no {} selected (invalid index)", kind);
                    Action::ShowDialog(DialogType::Error(format!("No {kind} selected to edit")))
                }
            },
            KeyCode::Char('r') => {
                info!("Global key: 'r' - starting manual sync");
                Action::StartSync
            }
            KeyCode::Char('R') => {
                if self.sync_service.is_debug_mode() {
                    info!("Global key: 'R' - refreshing local data (debug mode)");
                    Action::RefreshLocalData
                } else {
                    Action::None
                }
            }
            KeyCode::Char('/') => {
                info!("Global key: '/' - opening task search dialog");
                Action::ShowDialog(DialogType::TaskSearch)
            }
            KeyCode::Char('t') => {
                // Set task due date to today
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 't' - setting task '{}' due today", task.content);
                    Action::SetTaskDueToday(task.uuid)
                } else {
                    info!("Global key: 't' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Char('T') => {
                // Set task due date to tomorrow
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 'T' - setting task '{}' due tomorrow", task.content);
                    Action::SetTaskDueTomorrow(task.uuid)
                } else {
                    info!("Global key: 'T' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Char('w') => {
                // Set task due date to next week (Monday)
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 'w' - setting task '{}' due next week", task.content);
                    Action::SetTaskDueNextWeek(task.uuid)
                } else {
                    info!("Global key: 'w' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Char('W') => {
                // Set task due date to weekend (Saturday)
                if let Some(task) = self.task_list.get_selected_task() {
                    info!("Global key: 'W' - setting task '{}' due weekend", task.content);
                    Action::SetTaskDueWeekEnd(task.uuid)
                } else {
                    info!("Global key: 'W' - no task selected");
                    Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
                }
            }
            KeyCode::Esc => {
                if self.dialog.is_visible() {
                    info!("Global key: Esc - closing dialog");
                    Action::HideDialog
                } else {
                    info!("Global key: Esc - quitting application");
                    Action::Quit
                }
            }
            _ => Action::None,
        }
    }
}
