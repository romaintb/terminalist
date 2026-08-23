//! Terminalist - A Terminal User Interface (TUI) for Todoist
//!
//! This is the main entry point for the Terminalist application.
//! It handles command-line arguments, configuration loading, and
//! initializes the synchronization service before launching the UI.
//!
//! # Command Line Options
//!
//! * `-h, --help` - Show help message
//! * `-V, --version` - Show version information
//! * `-d, --debug` - Skip the startup sync and work from the cached data already on disk
//! * `--generate-config` - Generate a default configuration file
//!
//! # Environment Variables
//!
//! * `TODOIST_API_TOKEN` - Your Todoist API token (required)

use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;
use terminalist::{backend_registry, config, logger, storage, sync, ui};
use tokio::sync::Mutex;

/// Main entry point for the Terminalist application.
///
/// This function:
/// 1. Parses command-line arguments
/// 2. Loads configuration
/// 3. Validates the Todoist API token
/// 4. Initializes the sync service
/// 5. Launches the TUI application
///
/// # Errors
///
/// Returns an error if:
/// * Configuration cannot be loaded
/// * API token is not set
/// * Sync service fails to initialize
/// * UI fails to run
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let show_help = args.iter().any(|arg| arg == "--help" || arg == "-h");
    let show_version = args.iter().any(|arg| arg == "--version" || arg == "-V");
    let debug_mode = args.iter().any(|arg| arg == "--debug" || arg == "-d");
    let generate_config = args.iter().any(|arg| arg == "--generate-config");

    if show_version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if show_help {
        println!("Terminalist - A TUI for Todoist");
        println!();
        println!("USAGE:");
        println!("    terminalist [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help           Show this help message");
        println!("    -V, --version        Show version information");
        println!("    -d, --debug          Debug mode: skip the startup sync and use the cached data as-is");
        println!("    --generate-config    Generate a default configuration file");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    TODOIST_API_TOKEN    Your Todoist API token (required)");
        println!();
        return Ok(());
    }

    // Handle config generation
    if generate_config {
        let config_path = config::Config::get_default_config_path()?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        config::Config::generate_default_config(&config_path)?;

        return Ok(());
    }

    // Load configuration
    let (config, theme_warnings) = config::Config::load()?;

    // Initialize logger
    logger::init_logger(config.logging.enabled)?;

    for warning in &theme_warnings {
        log::warn!("{warning}");
    }

    // Check if API token is set
    if std::env::var("TODOIST_API_TOKEN").is_err() {
        eprintln!("❌ Error: TODOIST_API_TOKEN environment variable not set");
        eprintln!("\n💡 To use this app:");
        eprintln!("1. Go to https://app.todoist.com/app/settings/integrations/developer and copy your API token");
        eprintln!("2. Set it as environment variable: export TODOIST_API_TOKEN=your_token_here");
        eprintln!("3. Run the app again to see your actual data!");
        eprintln!("\n💡 Use --help for more options");
        return Ok(());
    }

    // Initialize storage
    let data_dir = storage::resolve_data_dir(config.storage.data_dir.as_deref())?;
    let local_storage = Arc::new(Mutex::new(storage::LocalStorage::new_at(&data_dir).await?));

    // Initialize backend registry
    let backend_registry = Arc::new(backend_registry::BackendRegistry::new(local_storage.clone()));
    backend_registry.load_backends().await?;

    // Create/refresh the Todoist backend. This adopts an existing row's UUID when one is already
    // there (including the random one older versions wrote) and derives a stable UUID otherwise,
    // so a relaunch just refreshes the credentials and keeps the cache attached.
    let api_token = std::env::var("TODOIST_API_TOKEN")?;
    let credentials = serde_json::json!({ "api_token": api_token }).to_string();

    let backend_uuid = backend_registry
        .add_backend(
            "todoist".to_string(),
            "My Todoist".to_string(),
            credentials,
            "{}".to_string(),
        )
        .await?;

    // Create sync service with timeout
    let timeout = tokio::time::Duration::from_secs(10);
    match tokio::time::timeout(
        timeout,
        sync::SyncService::new(backend_registry.clone(), backend_uuid, debug_mode),
    )
    .await
    {
        Ok(Ok(sync_service)) => {
            ui::run_app(sync_service, config, theme_warnings).await?;
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
