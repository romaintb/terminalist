pub mod debug_logger;
pub mod icons;
pub mod storage;
pub mod sync;
pub mod todoist;
pub mod ui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Check if API token is set
    if std::env::var("TODOIST_API_TOKEN").is_err() {
        eprintln!("❌ Error: TODOIST_API_TOKEN environment variable not set");
        eprintln!("\n💡 To use this app:");
        eprintln!("1. Get your API token from https://todoist.com/prefs/integrations");
        eprintln!("2. Set it as environment variable: export TODOIST_API_TOKEN=your_token_here");
        eprintln!("3. Run the app again to see your actual data!");
        return Ok(());
    }

    // Run the TUI application
    ui::run_app().await?;

    Ok(())
}
