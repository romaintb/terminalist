use terminalist::todoist::*;

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
    assert_eq!(display.priority, 3);
    assert!(!display.is_completed);
    assert!(display.is_recurring);
    assert_eq!(display.labels.len(), 2);
    assert_eq!(display.duration, Some("30m".to_string()));
}
