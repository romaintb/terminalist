//! Sidebar navigation component for the Terminalist application.
//!
//! This component provides the main navigation interface, allowing users to switch
//! between different views (Today, Tomorrow, Upcoming) and browse projects and labels.
//! It handles keyboard and mouse navigation with proper visual feedback.

use crate::entities::{label, project};
use crate::icons::IconService;
use crate::ui::components::scrollbar_helper::ScrollbarHelper;
use crate::ui::components::sidebar_item_component::{SidebarItem, SidebarItemType};
use crate::ui::core::SidebarSelection;
use crate::ui::core::{actions::Action, Component};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{block::BorderType, Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::collections::HashMap;
use uuid::Uuid;

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
    pub projects: Vec<project::Model>,
    pub labels: Vec<label::Model>,
    pub icons: IconService,
    items: Vec<SidebarItemType>,
    folder_states: HashMap<String, bool>,
    list_state: ListState,
    scroll_position: usize, // Virtual scroll position for view
    scrollbar_helper: ScrollbarHelper,
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
            items: Vec::new(),
            folder_states: HashMap::new(),
            list_state,
            scroll_position: 0,
            scrollbar_helper: ScrollbarHelper::new(),
        }
    }

    pub fn update_data(&mut self, projects: Vec<project::Model>, labels: Vec<label::Model>) {
        self.projects = projects;
        self.labels = labels;
        // Rebuild items list when data changes
        self.build_item_list();
        // Reset scroll when data changes
        self.scroll_position = 0;
        self.update_list_state();
    }

    /// Build the flattened list of sidebar items, respecting folder expanded/collapsed states
    fn build_item_list(&mut self) {
        self.items.clear();

        // Add special views (always visible)
        self.items.push(SidebarItemType::SpecialView {
            name: "Today".to_string(),
            selection: SidebarSelection::Today,
        });
        self.items.push(SidebarItemType::SpecialView {
            name: "Tomorrow".to_string(),
            selection: SidebarSelection::Tomorrow,
        });
        self.items.push(SidebarItemType::SpecialView {
            name: "Upcoming".to_string(),
            selection: SidebarSelection::Upcoming,
        });

        // Use placeholder account ID for now
        let account_id = "main".to_string();

        // Add labels
        for (index, label) in self.labels.iter().enumerate() {
            self.items.push(SidebarItemType::Label {
                label: label.clone(),
                account_id: account_id.clone(),
                original_index: index,
            });
        }

        // Add projects (sorted hierarchically), respecting fold states
        // Clone the data we need before mutating self.items
        let sorted_projects: Vec<_> = self.get_sorted_projects()
            .into_iter()
            .map(|(idx, proj)| (idx, proj.clone()))
            .collect();

        // Build a map of which projects have children
        let mut has_children_map: HashMap<Uuid, bool> = HashMap::new();
        for (_, project) in sorted_projects.iter() {
            if let Some(parent_uuid) = project.parent_uuid {
                has_children_map.insert(parent_uuid, true);
            }
        }

        for (i, (original_index, project)) in sorted_projects.iter().enumerate() {
            // Check if this project is a child of a collapsed parent
            if let Some(parent_uuid) = project.parent_uuid {
                let parent_key = parent_uuid.to_string();
                if let Some(&is_expanded) = self.folder_states.get(&parent_key) {
                    if !is_expanded {
                        // Skip this child project if parent is collapsed
                        continue;
                    }
                }
            }

            let depth = if project.parent_uuid.is_some() { 1 } else { 0 };
            let is_last_sibling = i + 1 == sorted_projects.len()
                || sorted_projects[i + 1].1.parent_uuid != project.parent_uuid;
            let has_children = has_children_map.get(&project.uuid).copied().unwrap_or(false);
            let is_expanded = self.folder_states
                .get(&project.uuid.to_string())
                .copied()
                .unwrap_or(true); // Default to expanded

            self.items.push(SidebarItemType::Project {
                project: project.clone(),
                account_id: account_id.clone(),
                original_index: *original_index,
                depth,
                is_last_sibling,
                has_children,
                is_expanded,
            });
        }
    }

    /// Toggle the expanded/collapsed state of a project folder
    pub fn toggle_folder(&mut self, key: &str) {
        if let Some(is_expanded) = self.folder_states.get_mut(key) {
            *is_expanded = !*is_expanded;
        } else {
            // If folder state doesn't exist, create it as collapsed
            self.folder_states.insert(key.to_string(), false);
        }
        // Rebuild items list after toggling
        self.build_item_list();
    }

    /// Check if the current position is on a foldable item and return its key
    fn get_folder_at_position(&self, index: usize) -> Option<String> {
        if let Some(item) = self.items.get(index) {
            if item.is_foldable() {
                match item {
                    SidebarItemType::AccountFolder { account_id, .. } => {
                        return Some(account_id.clone());
                    }
                    SidebarItemType::Project { project, has_children, .. } => {
                        if *has_children {
                            return Some(project.uuid.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Get total number of items in the sidebar
    fn total_items(&self) -> usize {
        self.items.len()
    }

    /// Scroll the viewport up (showing earlier items)
    fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
            self.update_scroll_state();
        }
    }

    /// Scroll the viewport down (showing later items)
    fn scroll_down(&mut self) {
        let total_items = self.total_items();
        if self.scroll_position + 1 < total_items {
            self.scroll_position += 1;
            self.update_scroll_state();
        }
    }

    /// Update list state and scrollbar for scrolling (different from selection)
    fn update_scroll_state(&mut self) {
        // Set list state to show the scroll position, which causes ratatui to scroll
        self.list_state.select(Some(self.scroll_position));

        // Update scrollbar based on scroll position
        let total_items = self.total_items();
        self.scrollbar_helper.update_state(total_items, self.scroll_position, None);
    }

    /// Update list state to reflect current selection (not scroll position)
    fn update_list_state(&mut self) {
        // Find the index of the current selection
        let selection_index = self.selection_to_index(&self.selection);
        self.list_state.select(Some(selection_index));

        // Update scrollbar state for selection-based positioning
        let total_items = self.total_items();
        self.scrollbar_helper.update_state(total_items, selection_index, None);
    }

    fn get_sorted_projects(&self) -> Vec<(usize, &project::Model)> {
        let mut projects_with_indices: Vec<(usize, &project::Model)> = self.projects.iter().enumerate().collect();

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
            let a_parent = &a_project.parent_uuid;
            let b_parent = &b_project.parent_uuid;
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
    fn get_root_project_id(&self, project: &project::Model) -> Uuid {
        project.parent_uuid.unwrap_or(project.uuid)
    }

    /// Get the root project (top-level parent) - always returns from self.projects
    fn get_root_project(&self, project: &project::Model) -> &project::Model {
        let root_id = self.get_root_project_id(project);
        self.projects
            .iter()
            .find(|p| p.uuid == root_id)
            .expect("Root project should exist in projects list")
    }

    /// Convert list index to SidebarSelection
    fn index_to_selection(&self, index: usize) -> SidebarSelection {
        if let Some(item) = self.items.get(index) {
            // Try to get selection from the item
            if let Some(selection) = item.get_selection() {
                return selection;
            }
        }
        // Default to Today if index is out of bounds or item is not selectable
        SidebarSelection::Today
    }

    /// Convert SidebarSelection to list index
    fn selection_to_index(&self, selection: &SidebarSelection) -> usize {
        for (index, item) in self.items.iter().enumerate() {
            if let Some(item_selection) = item.get_selection() {
                if &item_selection == selection {
                    return index;
                }
            }
        }
        // If not found, default to 0 (Today)
        0
    }

    /// Handle mouse events
    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) -> Action {
        // Check if mouse is within the sidebar area
        let is_in_area = mouse.column >= area.x
            && mouse.column < area.x + area.width
            && mouse.row >= area.y
            && mouse.row < area.y + area.height;

        if !is_in_area {
            return Action::None;
        }

        match mouse.kind {
            // Left click for selection
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row > area.y && mouse.row < area.y + area.height - 1 {
                    let local_index = (mouse.row - area.y - 1) as usize;
                    let clicked_index = self.list_state.offset() + local_index;

                    // Guard against clicks beyond the available data
                    if clicked_index >= self.total_items() {
                        return Action::None;
                    }

                    let selection = self.index_to_selection(clicked_index);
                    self.list_state.select(Some(clicked_index));
                    Action::NavigateToSidebar(selection)
                } else {
                    Action::None
                }
            }
            // Mouse wheel for navigation (move selection like task list)
            MouseEventKind::ScrollUp => {
                let current_index = self.list_state.selected().unwrap_or(0);
                // Find previous selectable item
                for offset in 1..=self.items.len() {
                    let prev_index = if current_index >= offset {
                        current_index - offset
                    } else {
                        self.items.len() + current_index - offset
                    };
                    if let Some(item) = self.items.get(prev_index) {
                        if item.is_selectable() {
                            if let Some(selection) = item.get_selection() {
                                self.list_state.select(Some(prev_index));
                                return Action::NavigateToSidebar(selection);
                            }
                        }
                    }
                }
                Action::None
            }
            MouseEventKind::ScrollDown => {
                let current_index = self.list_state.selected().unwrap_or(0);
                // Find next selectable item
                for offset in 1..=self.items.len() {
                    let next_index = (current_index + offset) % self.items.len();
                    if let Some(item) = self.items.get(next_index) {
                        if item.is_selectable() {
                            if let Some(selection) = item.get_selection() {
                                self.list_state.select(Some(next_index));
                                return Action::NavigateToSidebar(selection);
                            }
                        }
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }
}

impl Component for SidebarComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        use crossterm::event::KeyModifiers;

        match key.code {
            KeyCode::Char('H') => {
                // H key: collapse/fold folder if cursor is on a folder
                if let Some(current_index) = self.list_state.selected() {
                    if let Some(account_id) = self.get_folder_at_position(current_index) {
                        // Set folder to collapsed
                        self.folder_states.insert(account_id, false);
                        self.build_item_list();
                    }
                }
                Action::None
            }
            KeyCode::Char('L') => {
                // L key: expand/unfold folder if cursor is on a folder
                if let Some(current_index) = self.list_state.selected() {
                    if let Some(account_id) = self.get_folder_at_position(current_index) {
                        // Set folder to expanded
                        self.folder_states.insert(account_id, true);
                        self.build_item_list();
                    }
                }
                Action::None
            }
            KeyCode::Char('J') | KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                // Move to next selectable item, skipping non-selectable items (folders)
                let current_index = self.list_state.selected().unwrap_or(0);

                // Search forward for next selectable item
                for offset in 1..=self.items.len() {
                    let next_index = (current_index + offset) % self.items.len();
                    if let Some(item) = self.items.get(next_index) {
                        if item.is_selectable() {
                            if let Some(selection) = item.get_selection() {
                                self.list_state.select(Some(next_index));
                                return Action::NavigateToSidebar(selection);
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('K') | KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                // Move to previous selectable item, skipping non-selectable items (folders)
                let current_index = self.list_state.selected().unwrap_or(0);

                // Search backward for previous selectable item
                for offset in 1..=self.items.len() {
                    let prev_index = if current_index >= offset {
                        current_index - offset
                    } else {
                        self.items.len() + current_index - offset
                    };
                    if let Some(item) = self.items.get(prev_index) {
                        if item.is_selectable() {
                            if let Some(selection) = item.get_selection() {
                                self.list_state.select(Some(prev_index));
                                return Action::NavigateToSidebar(selection);
                            }
                        }
                    }
                }
                Action::None
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_up();
                Action::None
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn update(&mut self, action: Action) -> Action {
        match action {
            Action::NavigateToSidebar(selection) => {
                self.selection = selection.clone();
                self.update_list_state();
                // Pass the action through to AppComponent for further processing
                Action::NavigateToSidebar(selection)
            }
            _ => action,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        // Rebuild items list to ensure it's current
        self.build_item_list();

        // Ensure list state is synced with current selection (do this before borrowing items)
        self.update_list_state();

        // Render all items using their render() method
        let all_items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| item.render(&self.icons, &self.selection, false))
            .collect();

        // Calculate areas for list and scrollbar using helper
        let total_items = all_items.len();
        let (list_area, scrollbar_area) = ScrollbarHelper::calculate_areas(rect, total_items);

        // Update scrollbar state with current position and viewport info
        let available_height = rect.height.saturating_sub(2) as usize;
        let current_position = self.list_state.selected().unwrap_or(0);
        self.scrollbar_helper
            .update_state(total_items, current_position, Some(available_height));

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

        f.render_stateful_widget(list, list_area, &mut self.list_state);

        // Render scrollbar using helper
        self.scrollbar_helper.render(f, scrollbar_area);
    }
}
