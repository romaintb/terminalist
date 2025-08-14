use std::env;
use terminalist::todoist::TodoistWrapper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get API token from environment variable
    let api_token = env::var("TODOIST_API_TOKEN").expect("Please set TODOIST_API_TOKEN environment variable");

    let todoist = TodoistWrapper::new(api_token);

    // Example: Create a new task
    println!("Creating a new task...");
    let new_task = todoist.create_task("Test task from Rust", None).await?;
    println!("Created task: {} (ID: {})", new_task.content, new_task.id);

    // Example: Get all tasks and find our new task
    println!("\nFetching all tasks...");
    let tasks = todoist.get_tasks().await?;
    println!("Found {} tasks", tasks.len());

    // Find our newly created task
    if let Some(task) = tasks.iter().find(|t| t.id == new_task.id) {
        println!("Found our task: {}", task.content);
    }

    // Example: Complete the task
    println!("\nCompleting the task...");
    todoist.complete_task(&new_task.id).await?;
    println!("Task completed successfully!");

    Ok(())
}
