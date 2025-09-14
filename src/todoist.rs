// Re-export the Todoist API library
pub use todoist_api::*;

// Display models for UI rendering
#[derive(Debug, Clone)]
pub struct ProjectDisplay {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub parent_id: Option<String>,
    pub is_inbox_project: bool,
}

#[derive(Debug, Clone)]
pub struct LabelDisplay {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct SectionDisplay {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub order: i32,
}

#[derive(Debug, Clone)]
pub struct TaskDisplay {
    pub id: String,
    pub content: String,
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub priority: i32,
    pub due: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub labels: Vec<LabelDisplay>,
    pub description: String,
}

// Conversion implementations
impl From<Project> for ProjectDisplay {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            color: project.color,
            is_favorite: project.is_favorite,
            parent_id: project.parent_id,
            is_inbox_project: project.is_inbox_project,
        }
    }
}

impl From<Section> for SectionDisplay {
    fn from(section: Section) -> Self {
        Self {
            id: section.id,
            name: section.name,
            project_id: section.project_id,
            order: section.order,
        }
    }
}

impl From<Task> for TaskDisplay {
    fn from(task: Task) -> Self {
        let duration_string = task.duration.map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        // Convert label names to LabelDisplay objects (colors will be filled in later)
        let labels = task
            .labels
            .into_iter()
            .map(|name| LabelDisplay {
                id: name.clone(), // Use name as ID for now
                name,
                color: "blue".to_string(), // Default color, will be updated from storage
            })
            .collect();

        Self {
            id: task.id,
            content: task.content,
            project_id: task.project_id,
            section_id: task.section_id,
            parent_id: task.parent_id,
            priority: task.priority,
            due: task.due.as_ref().map(|d| d.date.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().is_some_and(|d| d.is_recurring),
            deadline: task.deadline.as_ref().map(|d| d.date.clone()),
            duration: duration_string,
            labels,
            description: task.description,
        }
    }
}
