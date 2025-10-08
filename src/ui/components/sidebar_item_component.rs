//! Sidebar item abstraction for hierarchical navigation.
//!
//! This module provides a trait-based abstraction for sidebar items,
//! enabling foldable account folders and hierarchical display of projects and labels.

use crate::entities::{label, project};
use crate::icons::IconService;
use crate::ui::core::SidebarSelection;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

/// Types of items that can appear in the sidebar
#[derive(Clone, Debug)]
pub enum SidebarItemType {
    /// Special views (Today, Tomorrow, Upcoming)
    SpecialView {
        name: String,
        selection: SidebarSelection,
    },
    /// Foldable account folder header
    AccountFolder {
        name: String,
        account_id: String,
        is_expanded: bool,
    },
    /// Project item (with account affiliation)
    Project {
        project: project::Model,
        account_id: String,
        original_index: usize,
        depth: usize,
        is_last_sibling: bool,
        has_children: bool,
        is_expanded: bool,
    },
    /// Label item (with account affiliation)
    Label {
        label: label::Model,
        account_id: String,
        original_index: usize,
    },
    /// Visual separator
    Separator { indent: usize },
}

/// Trait for sidebar items that can be rendered and navigated
pub trait SidebarItem {
    /// Render the item as a ListItem with appropriate styling
    fn render<'a>(
        &'a self,
        icons: &'a IconService,
        current_selection: &'a SidebarSelection,
        is_selected: bool,
    ) -> ListItem<'a>;

    /// Whether this item can be selected (navigated to)
    fn is_selectable(&self) -> bool;

    /// Get the indentation level for hierarchical display
    fn indent_level(&self) -> usize;

    /// Whether this item can be folded/unfolded
    fn is_foldable(&self) -> bool;

    /// Get the selection for this item (if selectable)
    fn get_selection(&self) -> Option<SidebarSelection>;
}

impl SidebarItem for SidebarItemType {
    fn render<'a>(
        &'a self,
        icons: &'a IconService,
        current_selection: &'a SidebarSelection,
        _is_selected: bool,
    ) -> ListItem<'a> {
        match self {
            SidebarItemType::SpecialView { name, selection } => {
                let is_selected = current_selection == selection;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let icon = match selection {
                    SidebarSelection::Today => icons.today(),
                    SidebarSelection::Tomorrow => icons.tomorrow(),
                    SidebarSelection::Upcoming => icons.upcoming(),
                    _ => "",
                };

                ListItem::new(Line::from(vec![
                    Span::styled(icon.to_string(), style),
                    Span::styled(name.clone(), style),
                ]))
            }

            SidebarItemType::AccountFolder {
                name,
                is_expanded,
                ..
            } => {
                let style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
                let arrow = if *is_expanded { "▼" } else { "▶" };
                let icon = "📦";

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", arrow), style),
                    Span::styled(format!("{} ", icon), style),
                    Span::styled(name.clone(), style),
                ]))
            }

            SidebarItemType::Project {
                project,
                original_index,
                depth,
                is_last_sibling,
                has_children,
                is_expanded,
                ..
            } => {
                let is_selected = matches!(
                    current_selection,
                    SidebarSelection::Project(idx) if idx == original_index
                );
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let tree_prefix = if *depth > 0 {
                    if *is_last_sibling {
                        "└─"
                    } else {
                        "├─"
                    }
                } else {
                    ""
                };

                let icon = if project.is_favorite {
                    icons.project_favorite()
                } else {
                    icons.project_regular()
                };

                let mut spans = vec![];

                // Add fold arrow if project has children
                if *has_children {
                    let arrow = if *is_expanded { "▼ " } else { "▶ " };
                    spans.push(Span::styled(arrow, style));
                }

                if !tree_prefix.is_empty() {
                    spans.push(Span::styled(
                        tree_prefix,
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                spans.push(Span::styled(icon.to_string(), style));
                spans.push(Span::styled(project.name.clone(), style));

                ListItem::new(Line::from(spans))
            }

            SidebarItemType::Label {
                label,
                original_index,
                ..
            } => {
                let is_selected = matches!(
                    current_selection,
                    SidebarSelection::Label(idx) if idx == original_index
                );
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(icons.label().to_string(), style),
                    Span::styled(label.name.clone(), style),
                ]))
            }

            SidebarItemType::Separator { indent } => {
                let spacing = " ".repeat(*indent);
                ListItem::new(Line::from(Span::raw(spacing)))
            }
        }
    }

    fn is_selectable(&self) -> bool {
        match self {
            SidebarItemType::SpecialView { .. } => true,
            SidebarItemType::AccountFolder { .. } => false, // Folders are not selectable, only foldable
            SidebarItemType::Project { .. } => true,
            SidebarItemType::Label { .. } => true,
            SidebarItemType::Separator { .. } => false,
        }
    }

    fn indent_level(&self) -> usize {
        match self {
            SidebarItemType::SpecialView { .. } => 0,
            SidebarItemType::AccountFolder { .. } => 0,
            SidebarItemType::Project { depth, .. } => *depth,
            SidebarItemType::Label { .. } => 0,
            SidebarItemType::Separator { indent } => *indent,
        }
    }

    fn is_foldable(&self) -> bool {
        match self {
            SidebarItemType::AccountFolder { .. } => true,
            SidebarItemType::Project { has_children, .. } => *has_children,
            _ => false,
        }
    }

    fn get_selection(&self) -> Option<SidebarSelection> {
        match self {
            SidebarItemType::SpecialView { selection, .. } => Some(selection.clone()),
            SidebarItemType::AccountFolder { .. } => None,
            SidebarItemType::Project { original_index, .. } => {
                Some(SidebarSelection::Project(*original_index))
            }
            SidebarItemType::Label { original_index, .. } => {
                Some(SidebarSelection::Label(*original_index))
            }
            SidebarItemType::Separator { .. } => None,
        }
    }
}
