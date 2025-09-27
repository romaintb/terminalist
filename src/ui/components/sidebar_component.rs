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
    scroll_offset: usize, // Track viewport scroll position
}

impl Default for SidebarComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarComponent {
    /// Creates a new SidebarComponent with default, ready-to-use state.
    ///
    /// The component is initialized with the "Today" selection, empty project and label lists,
    /// a default IconService, the internal list selection set to the first item, and a scroll offset of 0.
    ///
    /// # Examples
    ///
    /// ```
    /// let _sidebar = SidebarComponent::new();
    /// ```
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0)); // Start with Today selected
        Self {
            selection: SidebarSelection::Today,
            projects: Vec::new(),
            labels: Vec::new(),
            icons: IconService::default(),
            list_state,
            scroll_offset: 0,
        }
    }

    /// Replace the sidebar's project and label data, reset the viewport scroll, and resynchronize UI state.
    ///
    /// After calling this, the component will display the provided `projects` and `labels`,
    /// its internal scroll offset is set to zero, and the list selection/offset are updated to match
    /// the current selection.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sidebar = SidebarComponent::new();
    /// // Replace existing data with empty lists and reset scroll/state.
    /// sidebar.update_data(vec![], vec![]);
    /// assert_eq!(sidebar.total_items(), 3); // Today, Tomorrow, Upcoming
    /// ```
    pub fn update_data(&mut self, projects: Vec<ProjectDisplay>, labels: Vec<LabelDisplay>) {
        self.projects = projects;
        self.labels = labels;
        // Reset scroll when data changes
        self.scroll_offset = 0;
        self.update_list_state();
    }

    /// Returns the total number of selectable entries in the sidebar.
    ///
    /// This counts the three fixed navigation entries (Today, Tomorrow, Upcoming)
    /// plus the current number of labels and projects.
    ///
    /// # Examples
    ///
    /// ```
    /// let sb = SidebarComponent::new();
    /// // with no labels or projects, only Today/Tomorrow/Upcoming are present
    /// assert_eq!(sb.total_items(), 3);
    /// ```
    fn total_items(&self) -> usize {
        3 + self.labels.len() + self.projects.len() // Today, Tomorrow, Upcoming + labels + projects
    }

    /// Decrements the sidebar viewport scroll offset by one if the viewport is not already at the top.
    ///
    /// This moves the visible window toward earlier items; if `scroll_offset` is zero the method does nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = SidebarComponent::new();
    /// c.scroll_offset = 2;
    /// c.scroll_up();
    /// assert_eq!(c.scroll_offset, 1);
    /// ```
    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Advances the sidebar viewport downward by one item, if additional items exist.
    ///
    /// Does nothing when there are no items or the viewport is already positioned at the last item.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sidebar = SidebarComponent::new();
    /// // simulate data so there are multiple items
    /// sidebar.update_data(vec![/* one project */], vec![/* one label */]);
    /// let before = sidebar.scroll_offset;
    /// sidebar.scroll_down();
    /// assert!(sidebar.scroll_offset == before || sidebar.scroll_offset == before + 1);
    /// ```
    fn scroll_down(&mut self) {
        let total_items = self.total_items();
        if total_items > 0 && self.scroll_offset < total_items.saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    /// Synchronizes the internal `ListState` with the component's current selection and scroll offset.
    ///
    /// Updates the selected index shown by the list and applies `scroll_offset` so the List widget will render the correct viewport.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sidebar = SidebarComponent::new();
    /// sidebar.update_list_state();
    /// ```
    fn update_list_state(&mut self) {
        // Find the index of the current selection
        let selection_index = self.selection_to_index(&self.selection);
        self.list_state.select(Some(selection_index));

        // Set the scroll offset for the List widget
        *self.list_state.offset_mut() = self.scroll_offset;
    }

    /// Computes the next logical sidebar selection in forward order.
    ///
    /// Navigation advances through the fixed items (Today → Tomorrow → Upcoming), then through labels (in index order), then through projects (in sorted order). From Upcoming it moves to the first label if any, otherwise to the first project if any, otherwise wraps to Today. From a Label it moves to the next label if present, otherwise to the first project if any, otherwise to Today. From a Project it moves to the next project in sorted order, otherwise wraps to Today.
    ///
    /// # Examples
    ///
    /// ```
    /// let comp = SidebarComponent::default();
    /// // Default selection is Today, so the next selection is Tomorrow.
    /// assert_eq!(comp.get_next_selection(), SidebarSelection::Tomorrow);
    /// ```
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

    /// Map a sidebar list index into the corresponding `SidebarSelection`.
    ///
    /// The `index` is the zero-based position in the rendered sidebar list:
    /// - 0 => Today, 1 => Tomorrow, 2 => Upcoming
    /// - 3..(3 + labels.len()) => `Label(index - 3)`
    /// - remaining indices map to `Project(original_index)` according to the component's sorted project order
    ///
    /// # Returns
    ///
    /// The `SidebarSelection` for the given list index. If the index does not correspond to any project (out of range),
    /// `SidebarSelection::Today` is returned as a safe default.
    ///
    /// # Examples
    ///
    /// ```
    /// // assuming `comp` is a SidebarComponent with at least one label and one project:
    /// let sel0 = comp.index_to_selection(0);
    /// assert_eq!(sel0, SidebarSelection::Today);
    ///
    /// let sel_label0 = comp.index_to_selection(3);
    /// matches!(sel_label0, SidebarSelection::Label(0));
    /// ```
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

    /// Map a `SidebarSelection` to its corresponding list index in the sidebar view.
    ///
    /// The mapping is:
    /// - `Today` -> 0
    /// - `Tomorrow` -> 1
    /// - `Upcoming` -> 2
    /// - `Label(i)` -> `3 + i`
    /// - `Project(original_index)` -> `3 + labels.len() + position_in_sorted_projects`
    ///
    /// If a `Project` selection's original index is not found in the current sorted projects, the function returns `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sidebar = SidebarComponent::new();
    /// // Default selection mapping for Today
    /// assert_eq!(sidebar.selection_to_index(&SidebarSelection::Today), 0);
    /// ```
    fn selection_to_index(&self, selection: &SidebarSelection) -> usize {
        match selection {
            SidebarSelection::Today => 0,
            SidebarSelection::Tomorrow => 1,
            SidebarSelection::Upcoming => 2,
            SidebarSelection::Label(index) => 3 + index,
            SidebarSelection::Project(original_index) => {
                // Find the position of this project in the sorted list
                let sorted_projects = self.get_sorted_projects();
                for (sorted_index, (orig_idx, _)) in sorted_projects.iter().enumerate() {
                    if orig_idx == original_index {
                        return 3 + self.labels.len() + sorted_index;
                    }
                }
                // If not found, default to Today
                0
            }
        }
    }

    /// Handle a left-button mouse click inside the sidebar area and navigate to the clicked item.
    ///
    /// If the mouse event is a left-button down within the given rectangular `area`, the function
    /// computes which list item was clicked (based on the row offset), updates the internal list
    /// selection state to that index, and returns `Action::NavigateToSidebar` with the corresponding
    /// `SidebarSelection`. If the event is outside the clickable bounds or not a left-button down,
    /// the function returns `Action::None`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut sidebar = SidebarComponent::new();
    /// let area = Rect { x: 0, y: 0, width: 10, height: 5 };
    /// let mouse = MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 1, row: 2 };
    /// let action = sidebar.handle_mouse(mouse, area);
    /// let expected = Action::NavigateToSidebar(sidebar.index_to_selection((2 - area.y - 1) as usize));
    /// assert_eq!(action, expected);
    /// ```
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
    /// Handles keyboard input for sidebar navigation and scrolling.
    ///
    /// Maps specific key events to navigation or viewport scroll actions:
    /// - `J`, `Shift+Down` => move selection forward and return `Action::NavigateToSidebar` with the new selection.
    /// - `K`, `Shift+Up` => move selection backward and return `Action::NavigateToSidebar` with the new selection.
    /// - `Ctrl+Up` => scroll the sidebar viewport up and return `Action::None`.
    /// - `Ctrl+Down` => scroll the sidebar viewport down and return `Action::None`.
    /// - any other key => no action (`Action::None`).
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    ///
    /// let mut sidebar = SidebarComponent::default();
    /// let action = sidebar.handle_key_events(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE));
    /// match action {
    ///     Action::NavigateToSidebar(_) => {}
    ///     _ => panic!("expected navigation action"),
    /// }
    /// ```
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

    /// Updates the component in response to an action and synchronizes the sidebar selection state.
    ///
    /// When given `Action::NavigateToSidebar(selection)`, sets the component's selection to `selection`,
    /// refreshes the internal list state (including the scroll offset) to reflect that selection, and
    /// returns `Action::NavigateToSidebar(selection)`. All other actions are returned unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut s = SidebarComponent::new();
    /// let action = Action::NavigateToSidebar(SidebarSelection::Tomorrow);
    /// let out = s.update(action.clone());
    /// assert_eq!(out, action);
    /// ```
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

    /// Renders the sidebar navigation list into the provided frame rectangle.
    ///
    /// This draws the static sections (Today, Tomorrow, Upcoming), label entries, and
    /// the project list (hierarchically sorted and indented), applying iconography
    /// and a visual selection style for the current `SidebarSelection`. The component's
    /// internal `list_state` is synchronized with the current selection and scroll
    /// offset before the list widget is rendered inside a bordered block titled
    /// "Navigation".
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use terminalist_ui::{SidebarComponent, Frame, Rect};
    /// # fn get_frame_and_rect() -> (Frame<'static>, Rect) { unimplemented!() }
    /// let mut sidebar = SidebarComponent::new();
    /// // populate sidebar.labels / sidebar.projects as needed...
    /// let (mut frame, rect) = get_frame_and_rect();
    /// sidebar.render(&mut frame, rect);
    /// ```
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

        // Ensure list state is synced with current selection
        self.update_list_state();

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
