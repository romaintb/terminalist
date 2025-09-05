pub mod debug_logger;
pub mod icons;
pub mod storage;
pub mod sync;
pub mod todoist;
pub mod ui;
pub mod utils;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let show_help = args.iter().any(|arg| arg == "--help" || arg == "-h");

    if show_help {
        println!("Terminalist - A TUI for Todoist");
        println!();
        println!("USAGE:");
        println!("    terminalist [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help    Show this help message");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    TODOIST_API_TOKEN    Your Todoist API token (required)");
        println!();
        println!("NAVIGATION:");
        println!("    J/K           Navigate sidebar items (projects/labels/views)");
        println!("    j/k           Navigate task items");
        println!("    ?/h           Toggle help dialog");
        println!();
        return Ok(());
    }

    // Check if API token is set
    if std::env::var("TODOIST_API_TOKEN").is_err() {
        eprintln!("❌ Error: TODOIST_API_TOKEN environment variable not set");
        eprintln!("\n💡 To use this app:");
        eprintln!("1. Get your API token from https://todoist.com/prefs/integrations");
        eprintln!("2. Set it as environment variable: export TODOIST_API_TOKEN=your_token_here");
        eprintln!("3. Run the app again to see your actual data!");
        eprintln!("\n💡 Use --help for more options");
        return Ok(());
    }

    // Create sync service with timeout
    let api_token = std::env::var("TODOIST_API_TOKEN")?;

    match tokio::time::timeout(tokio::time::Duration::from_secs(10), sync::SyncService::new(api_token)).await {
        Ok(Ok(sync_service)) => {
            ui::run_new_app(sync_service).await?;
        }
        Ok(Err(e)) => {
            return Err(e);
        }
        Err(_) => {
            return Err(anyhow::anyhow!("Sync service creation timed out"));
        }
    }

    Ok(())
}
