//! Backend factory for creating backend instances from configuration.

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::{todoist::TodoistBackend, Backend};

/// Create a backend instance from backend type and credentials.
///
/// # Arguments
/// * `backend_type` - The type of backend (e.g., "todoist")
/// * `credentials` - JSON-encoded credentials string
///
/// # Returns
/// A boxed Backend trait object
///
/// # Errors
/// Returns error if:
/// - Backend type is unknown
/// - Credentials are invalid JSON
/// - Required credentials are missing
pub fn create_backend(backend_type: &str, credentials: &str) -> Result<Box<dyn Backend>> {
    let creds: Value =
        serde_json::from_str(credentials).map_err(|e| anyhow!("Failed to parse credentials JSON: {}", e))?;

    match backend_type {
        "todoist" => {
            let api_token = creds["api_token"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing 'api_token' in Todoist credentials"))?;
            Ok(Box::new(TodoistBackend::new(api_token.to_string())))
        }
        // Future backends can be added here:
        // "ticktick" => {
        //     let api_token = creds["api_token"].as_str().ok_or(...)?;
        //     Ok(Box::new(TickTickBackend::new(api_token.to_string())))
        // }
        // "github" => {
        //     let access_token = creds["access_token"].as_str().ok_or(...)?;
        //     Ok(Box::new(GitHubBackend::new(access_token.to_string())))
        // }
        _ => Err(anyhow!("Unknown backend type: {}", backend_type)),
    }
}
