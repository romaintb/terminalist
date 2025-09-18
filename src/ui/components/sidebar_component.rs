//! Sidebar navigation component for the Terminalist application.
//!
//! This component provides the main navigation interface, allowing users to switch
//! between different views (Today, Tomorrow, Upcoming) and browse projects and labels.
//! It handles keyboard and mouse navigation with proper visual feedback.

use crate::icons::IconService;
use crate::todoist::{LabelDisplay, ProjectDisplay};
use crate::ui::core::SidebarSelection;
use crate::ui::core::{actions::Action, Component};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{block::BorderType, Block, Borders, List, ListItem, ListState},
    Frame,
};

/// Navigation sidebar component for switching between views, projects, and labels.
///
/// The sidebar provides a hierarchical navigation structure:
/// - Special views (Today, Tomorrow, Upcoming)
/// - Projects (user-created project list)
/// - Labels (for filtering tasks by label)
///
/// Features:
/// - Keyboard navigation (Up/Down arrows, Enter to select)
/// - Mouse support (click to select)
/// - Visual indicators for the current selection
/// - Dynamic updates when projects/labels change
/// - Icon support for better visual organization
pub struct SidebarComponent {
    pub selection: SidebarSelection,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub icons: IconService,
    list_state: ListState,
}

impl Default for SidebarComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarComponent {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0)); // Start with Today selected
        Self {
            selection: SidebarSelection::Today,
            projects: Vec::new(),
            labels: Vec::new(),
            icons: IconService::default(),
            list_state,
        }
    }

    pub fn update_data(&mut self, projects: Vec<ProjectDisplay>, labels: Vec<LabelDisplay>) {
        self.projects = projects;
        self.labels = labels;
    }

    fn get_next_selection(&self) -> SidebarSelection {
        match &self.selection {
            SidebarSelection::Today => SidebarSelection::Tomorrow,
            SidebarSelection::Tomorrow => SidebarSelection::Upcoming,
            SidebarSelection::Upcoming => {
                if !self.labels.is_empty() {
                    SidebarSelection::Label(0)
                } else if !self.projects.is_empty() {
                    let sorted_projects = self.get_sorted_projects();
                    if let Some((original_index, _)) = sorted_projects.first() {
                        SidebarSelection::Project(*original_index)
                    } else {
                        SidebarSelection::Today
                    }
                } else {
                    SidebarSelection::Today
                }
            }
            SidebarSelection::Label(index) => {
                let next_index = index + 1;
                if next_index < self.labels.len() {
                    SidebarSelection::Label(next_index)
                } else if !self.projects.is_empty() {
                    let sorted_projects = self.get_sorted_projects();
                    if let Some((original_index, _)) = sorted_projects.first() {
                        SidebarSelection::Project(*original_index)
                    } else {
                        SidebarSelection::Today
                    }
                } else {
                    SidebarSelection::Today
                }
            }
            SidebarSelection::Project(index) => {
                let sorted_projects = self.get_sorted_projects();
                if let Some(current_sorted_index) = sorted_projects.iter().position(|(orig_idx, _)| orig_idx == index) {
                    let next_sorted_index = current_sorted_index + 1;
                    if next_sorted_index < sorted_projects.len() {
                        if let Some((original_index, _)) = sorted_projects.get(next_sorted_index) {
                            SidebarSelection::Project(*original_index)
                        } else {
                            SidebarSelection::Today
                        }
                    } else {
                        SidebarSelection::Today
                    }
                } else {
                    SidebarSelection::Today
                }
            }
        }
    }

    fn get_previous_selection(&self) -> SidebarSelection {
        match &self.selection {
            SidebarSelection::Today => {
                if !self.projects.is_empty() {
                    let sorted_projects = self.get_sorted_projects();
                    if let Some((original_index, _)) = sorted_projects.last() {
                        SidebarSelection::Project(*original_index)
                    } else {
                        SidebarSelection::Tomorrow
                    }
                } else if !self.labels.is_empty() {
                    SidebarSelection::Label(self.labels.len() - 1)
                } else {
                    SidebarSelection::Tomorrow
                }
            }
            SidebarSelection::Tomorrow => SidebarSelection::Today,
            SidebarSelection::Upcoming => SidebarSelection::Tomorrow,
            SidebarSelection::Label(index) => {
                if *index > 0 {
                    SidebarSelection::Label(index - 1)
                } else {
                    SidebarSelection::Upcoming
                }
            }
            SidebarSelection::Project(index) => {
                let sorted_projects = self.get_sorted_projects();
                if let Some(current_sorted_index) = sorted_projects.iter().position(|(orig_idx, _)| orig_idx == index) {
                    if current_sorted_index > 0 {
                        if let Some((original_index, _)) = sorted_projects.get(current_sorted_index - 1) {
                            SidebarSelection::Project(*original_index)
                        } else {
                            SidebarSelection::Today
                        }
                    } else if !self.labels.is_empty() {
                        SidebarSelection::Label(self.labels.len() - 1)
                    } else {
                        SidebarSelection::Upcoming
                    }
                } else {
                    SidebarSelection::Today
                }
            }
        }
    }

    fn get_sorted_projects(&self) -> Vec<(usize, &ProjectDisplay)> {
        let mut projects_with_indices: Vec<(usize, &ProjectDisplay)> = self.projects.iter().enumerate().collect();

        // Sort projects hierarchically: root → parent → favorites → name
        projects_with_indices.sort_by(|(_, a_project), (_, b_project)| {
            // First, sort by root project to keep tree structures together
            let a_root_project = self.get_root_project(a_project);
            let b_root_project = self.get_root_project(b_project);

            // Sort root projects: Inbox first, then alphabetically by name
            let root_cmp = match (a_root_project.is_inbox_project, b_root_project.is_inbox_project) {
                (true, false) => std::cmp::Ordering::Less,          // Inbox first
                (false, true) => std::cmp::Ordering::Greater,       // Inbox first
                _ => a_root_project.name.cmp(&b_root_project.name), // Both inbox or both regular, sort by name
            };

            if root_cmp != std::cmp::Ordering::Equal {
                return root_cmp;
            }

            // Same root, now sort by immediate parent to keep siblings together
            let a_parent = &a_project.parent_id;
            let b_parent = &b_project.parent_id;
            let parent_cmp = a_parent.cmp(b_parent);
            if parent_cmp != std::cmp::Ordering::Equal {
                return parent_cmp;
            }

            // Same immediate parent (siblings), sort favorites first, then by name
            match (a_project.is_favorite, b_project.is_favorite) {
                (true, false) => std::cmp::Ordering::Less, // a (favorite) comes before b (non-favorite)
                (false, true) => std::cmp::Ordering::Greater, // a (non-favorite) comes after b (favorite)
                _ => a_project.name.cmp(&b_project.name),  // Same favorite status, sort by name
            }
        });
        projects_with_indices
    }

    /// Get the root project ID (top-level parent)
    /// Since Todoist only has parent/child, root is either the project itself or its parent
    fn get_root_project_id(&self, project: &ProjectDisplay) -> String {
        project.parent_id.clone().unwrap_or_else(|| project.id.clone())
    }

    /// Get the root project (top-level parent) - always returns from self.projects
    fn get_root_project(&self, project: &ProjectDisplay) -> &ProjectDisplay {
        let root_id = self.get_root_project_id(project);
        self.projects
            .iter()
            .find(|p| p.id == root_id)
            .expect("Root project should exist in projects list")
    }

    /// Calculate the tree depth of a project for indentation
    /// Since Todoist only has parent/child (no deeper nesting), depth is either 0 or 1
    fn calculate_tree_depth(&self, project: &ProjectDisplay) -> usize {
        if project.parent_id.is_some() {
            1
        } else {
            0
        }
    }

    /// Convert list index to SidebarSelection
    fn index_to_selection(&self, index: usize) -> SidebarSelection {
        if index == 0 {
            return SidebarSelection::Today;
        }
        if index == 1 {
            return SidebarSelection::Tomorrow;
        }
        if index == 2 {
            return SidebarSelection::Upcoming;
        }

        let label_count = self.labels.len();
        if index < 3 + label_count {
            return SidebarSelection::Label(index - 3);
        }

        let project_index = index - 3 - label_count;
        let sorted_projects = self.get_sorted_projects();
        if let Some((original_index, _)) = sorted_projects.get(project_index) {
            SidebarSelection::Project(*original_index)
        } else {
            SidebarSelection::Today
        }
    }

    /// Handle mouse events
    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) -> Action {
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && mouse.column >= area.x
            && mouse.column < area.x + area.width
            && mouse.row > area.y
            && mouse.row < area.y + area.height - 1
        {
            let clicked_index = (mouse.row - area.y - 1) as usize;
            let selection = self.index_to_selection(clicked_index);
            self.list_state.select(Some(clicked_index));
            return Action::NavigateToSidebar(selection);
        }
        Action::None
    }
}

impl Component for SidebarComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        use crossterm::event::KeyModifiers;

        match key.code {
            KeyCode::Char('J') => {
                let next_selection = self.get_next_selection();
                Action::NavigateToSidebar(next_selection)
            }
            KeyCode::Char('K') => {
                let prev_selection = self.get_previous_selection();
                Action::NavigateToSidebar(prev_selection)
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let next_selection = self.get_next_selection();
                Action::NavigateToSidebar(next_selection)
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let prev_selection = self.get_previous_selection();
                Action::NavigateToSidebar(prev_selection)
            }
            _ => Action::None,
        }
    }

    fn update(&mut self, action: Action) -> Action {
        match action {
            Action::NavigateToSidebar(selection) => {
                self.selection = selection.clone();
                // Pass the action through to AppComponent for further processing
                Action::NavigateToSidebar(selection)
            }
            _ => action,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let mut all_items: Vec<ListItem> = Vec::new();

        // Add Today item
        let is_today_selected = matches!(self.selection, SidebarSelection::Today);
        let today_style = if is_today_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        all_items.push(ListItem::new(Line::from(vec![
            Span::styled(self.icons.today().to_string(), today_style),
            Span::styled("Today".to_string(), today_style),
        ])));

        // Add Tomorrow item
        let is_tomorrow_selected = matches!(self.selection, SidebarSelection::Tomorrow);
        let tomorrow_style = if is_tomorrow_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        all_items.push(ListItem::new(Line::from(vec![
            Span::styled(self.icons.tomorrow().to_string(), tomorrow_style),
            Span::styled("Tomorrow".to_string(), tomorrow_style),
        ])));

        // Add Upcoming item
        let is_upcoming_selected = matches!(self.selection, SidebarSelection::Upcoming);
        let upcoming_style = if is_upcoming_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        all_items.push(ListItem::new(Line::from(vec![
            Span::styled(self.icons.upcoming().to_string(), upcoming_style),
            Span::styled("Upcoming".to_string(), upcoming_style),
        ])));

        // Add labels
        for (index, label) in self.labels.iter().enumerate() {
            let is_selected = matches!(self.selection, SidebarSelection::Label(i) if i == index);
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            all_items.push(ListItem::new(Line::from(vec![
                Span::styled(self.icons.label().to_string(), style),
                Span::styled(label.name.clone(), style),
            ])));
        }

        // Add projects (sorted hierarchically)
        let sorted_projects = self.get_sorted_projects();
        for (i, (original_index, project)) in sorted_projects.iter().enumerate() {
            let is_selected = matches!(self.selection, SidebarSelection::Project(idx) if idx == *original_index);
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let depth = self.calculate_tree_depth(project);
            let tree_prefix = if depth > 0 {
                let is_last = i + 1 == sorted_projects.len() || sorted_projects[i + 1].1.parent_id != project.parent_id;
                if is_last {
                    "└─"
                } else {
                    "├─"
                }
            } else {
                ""
            };

            let icon = if project.is_favorite {
                self.icons.project_favorite()
            } else {
                self.icons.project_regular()
            };

            let mut spans = vec![];
            if !tree_prefix.is_empty() {
                spans.push(Span::styled(tree_prefix, Style::default().fg(Color::DarkGray)));
            }
            spans.extend([Span::styled(icon.to_string(), style), Span::styled(project.name.clone(), style)]);

            all_items.push(ListItem::new(Line::from(spans)));
        }

        let list = List::new(all_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Navigation")
                    .title_style(Style::default().fg(Color::White))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::White));

        f.render_stateful_widget(list, rect, &mut self.list_state);
    }
}
