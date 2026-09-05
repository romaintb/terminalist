//! Sidebar item abstraction for hierarchical navigation.
//!
//! This module provides a trait-based abstraction for sidebar items,
//! enabling foldable account folders and hierarchical display of projects and labels.

use crate::entities::{label, project};
use crate::icons::IconService;
use crate::theme::Theme;
use crate::ui::core::SidebarSelection;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

/// Types of items that can appear in the sidebar
#[derive(Clone, Debug)]
pub enum SidebarItemType {
    /// Special views (Today, Tomorrow, Upcoming)
    SpecialView { name: String, selection: SidebarSelection },
    /// Project item
    Project {
        project: project::Model,
        original_index: usize,
        depth: usize,
        is_last_sibling: bool,
        has_children: bool,
        is_expanded: bool,
    },
    /// Label item
    Label { label: label::Model, original_index: usize },
}

/// Trait for sidebar items that can be rendered and navigated
pub trait SidebarItem {
    /// Render the item as a ListItem with appropriate styling
    fn render<'a>(
        &'a self,
        icons: &'a IconService,
        current_selection: &'a SidebarSelection,
        is_selected: bool,
        theme: &'a Theme,
    ) -> ListItem<'a>;

    /// Whether this item can be selected (navigated to)
    fn is_selectable(&self) -> bool;

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
        theme: &'a Theme,
    ) -> ListItem<'a> {
        match self {
            SidebarItemType::SpecialView { name, selection } => {
                let is_selected = current_selection == selection;
                let style = if is_selected {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
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
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
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
                    spans.push(Span::styled(tree_prefix, Style::default().fg(theme.text_muted)));
                }
                spans.push(Span::styled(icon.to_string(), style));
                spans.push(Span::styled(project.name.clone(), style));

                ListItem::new(Line::from(spans))
            }

            SidebarItemType::Label {
                label, original_index, ..
            } => {
                let is_selected = matches!(
                    current_selection,
                    SidebarSelection::Label(idx) if idx == original_index
                );
                let style = if is_selected {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(icons.label().to_string(), style),
                    Span::styled(label.name.clone(), style),
                ]))
            }
        }
    }

    fn is_selectable(&self) -> bool {
        true
    }

    fn is_foldable(&self) -> bool {
        matches!(self, SidebarItemType::Project { has_children, .. } if *has_children)
    }

    fn get_selection(&self) -> Option<SidebarSelection> {
        match self {
            SidebarItemType::SpecialView { selection, .. } => Some(selection.clone()),
            SidebarItemType::Project { original_index, .. } => Some(SidebarSelection::Project(*original_index)),
            SidebarItemType::Label { original_index, .. } => Some(SidebarSelection::Label(*original_index)),
        }
    }
}
