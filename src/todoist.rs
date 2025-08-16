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
}

#[derive(Debug, Clone)]
pub struct LabelDisplay {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct TaskDisplay {
    pub id: String,
    pub content: String,
    pub project_id: String,
    pub is_completed: bool,
    pub is_deleted: bool,
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
            is_completed: task.is_completed,
            is_deleted: false, // New tasks are not deleted
            priority: task.priority,
            due: task.due.as_ref().map(|d| d.date.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false),
            deadline: task.deadline.as_ref().map(|d| d.date.clone()),
            duration: duration_string,
            labels,
            description: task.description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_conversion() {
        let project = Project {
            id: "123".to_string(),
            name: "Test Project".to_string(),
            comment_count: 0,
            order: 1,
            color: "blue".to_string(),
            is_shared: false,
            is_favorite: true,
            is_inbox_project: false,
            is_team_inbox: false,
            view_style: "list".to_string(),
            url: "https://todoist.com".to_string(),
            parent_id: None,
        };

        let display: ProjectDisplay = project.into();
        assert_eq!(display.id, "123");
        assert_eq!(display.name, "Test Project");
        assert_eq!(display.color, "blue");
        assert!(display.is_favorite);
    }

    #[test]
    fn test_task_conversion() {
        let task = Task {
            id: "456".to_string(),
            content: "Test Task".to_string(),
            description: "Test Description".to_string(),
            project_id: "123".to_string(),
            section_id: None,
            parent_id: None,
            order: 1,
            priority: 3,
            is_completed: false,
            labels: vec!["label1".to_string(), "label2".to_string()],
            created_at: "2023-01-01T00:00:00Z".to_string(),
            due: Some(Due {
                string: "tomorrow".to_string(),
                date: "2023-01-02".to_string(),
                is_recurring: true,
                datetime: None,
                timezone: None,
            }),
            deadline: None,
            duration: Some(Duration {
                amount: 30,
                unit: "minute".to_string(),
            }),
            assignee_id: None,
            url: "https://todoist.com".to_string(),
            comment_count: 0,
        };

        let display: TaskDisplay = task.into();
        assert_eq!(display.id, "456");
        assert_eq!(display.content, "Test Task");
        assert_eq!(display.project_id, "123");
        assert!(!display.is_completed);
        assert_eq!(display.priority, 3);
        assert_eq!(display.due, Some("2023-01-02".to_string()));
        assert!(display.is_recurring);
        assert_eq!(display.duration, Some("30m".to_string()));
        assert_eq!(display.labels.len(), 2);
        assert_eq!(display.labels[0].name, "label1");
        assert_eq!(display.labels[1].name, "label2");
    }
}
