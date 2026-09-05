//! Global key bindings: the last stop for a key event, after the dialog, the sidebar
//! and the task list have each had their turn in `handle_event`.

use super::AppComponent;
use crate::constants::{UI_CANNOT_DELETE_TODAY_VIEW, UI_NO_TASK_SELECTED_DUE_DATE};
use crate::ui::core::actions::{Action, DialogType};
use crate::ui::core::SidebarSelection;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use log::info;

impl AppComponent {
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
            KeyCode::Char('D') => {
                // Delete current project (only if a project is selected)
                match &self.state.sidebar_selection {
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
                            info!(
                                "Global key: 'D' - deleting project '{}' (ID: {})",
                                project.name, project.uuid
                            );
                            Action::ShowDialog(DialogType::DeleteConfirmation {
                                item_type: "project".to_string(),
                                item_uuid: project.uuid,
                            })
                        } else {
                            info!("Global key: 'D' - no project selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No project selected to delete".to_string()))
                        }
                    }
                    SidebarSelection::Today => {
                        info!("Global key: 'D' - cannot delete Today view");
                        Action::ShowDialog(DialogType::Info(UI_CANNOT_DELETE_TODAY_VIEW.to_string()))
                    }
                    SidebarSelection::Tomorrow => {
                        info!("Global key: 'D' - cannot delete Tomorrow view");
                        Action::ShowDialog(DialogType::Info("Cannot delete the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        info!("Global key: 'D' - cannot delete Upcoming view");
                        Action::ShowDialog(DialogType::Info("Cannot delete the Upcoming view".to_string()))
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
                            info!("Global key: 'D' - deleting label '{}' (ID: {})", label.name, label.uuid);
                            Action::ShowDialog(DialogType::DeleteConfirmation {
                                item_type: "label".to_string(),
                                item_uuid: label.uuid,
                            })
                        } else {
                            info!("Global key: 'D' - no label selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No label selected to delete".to_string()))
                        }
                    }
                }
            }
            KeyCode::Char('E') => {
                // Edit current sidebar selection (project or label)
                match &self.state.sidebar_selection {
                    SidebarSelection::Project(index) => {
                        if let Some(project) = self.state.projects.get(*index) {
                            info!(
                                "Global key: 'E' - editing project '{}' (ID: {})",
                                project.name, project.uuid
                            );
                            Action::ShowDialog(DialogType::ProjectEdit {
                                project_uuid: project.uuid,
                                name: project.name.clone(),
                            })
                        } else {
                            info!("Global key: 'E' - no project selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No project selected to edit".to_string()))
                        }
                    }
                    SidebarSelection::Today => {
                        info!("Global key: 'E' - cannot edit Today view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Today view".to_string()))
                    }
                    SidebarSelection::Tomorrow => {
                        info!("Global key: 'E' - cannot edit Tomorrow view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Tomorrow view".to_string()))
                    }
                    SidebarSelection::Upcoming => {
                        info!("Global key: 'E' - cannot edit Upcoming view");
                        Action::ShowDialog(DialogType::Info("Cannot edit the Upcoming view".to_string()))
                    }
                    SidebarSelection::Label(index) => {
                        if let Some(label) = self.state.labels.get(*index) {
                            info!("Global key: 'E' - editing label '{}' (ID: {})", label.name, label.uuid);
                            Action::ShowDialog(DialogType::LabelEdit {
                                label_uuid: label.uuid,
                                name: label.name.clone(),
                            })
                        } else {
                            info!("Global key: 'E' - no label selected (invalid index)");
                            Action::ShowDialog(DialogType::Error("No label selected to edit".to_string()))
                        }
                    }
                }
            }
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
