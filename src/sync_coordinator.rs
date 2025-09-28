use std::collections::HashMap;
use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};

use crate::backends::registry::BackendRegistry;
use crate::backends::todoist::TodoistBackend;
use crate::storage::LocalStorage;
use crate::config::Config;
use std::sync::Arc;

/// Coordinates synchronization across multiple backends
pub struct SyncCoordinator {
    storage: LocalStorage,
    backend_registry: BackendRegistry,
}

impl SyncCoordinator {
    /// Create a new sync coordinator
    pub fn new(storage: LocalStorage, backend_registry: BackendRegistry) -> Self {
        Self {
            storage,
            backend_registry,
        }
    }

    /// Initialize the sync coordinator with backends from config
    pub async fn initialize_from_config(&mut self, config: &Config) -> Result<()> {
        info!("Initializing sync coordinator with {} backend instances",
              config.backends.instances.len());

        // Register all backend instances in the database
        for (backend_id, instance_config) in &config.backends.instances {
            if !instance_config.enabled {
                info!("Backend '{}' is disabled, skipping", backend_id);
                continue;
            }

            info!("Registering backend '{}' ({})", backend_id, instance_config.name);

            // Register in database
            self.storage
                .register_backend(
                    backend_id,
                    &instance_config.backend_type,
                    &instance_config.name,
                    instance_config.enabled,
                    None, // We don't store sensitive config in DB
                )
                .await?;

            // Add to registry
            match instance_config.backend_type.as_str() {
                "todoist" => {
                    // Get API token from environment
                    let token_env_var = instance_config.config
                        .get("api_token_env")
                        .map(|v| v.as_str())
                        .unwrap_or("TODOIST_API_TOKEN");

                    match std::env::var(token_env_var) {
                        Ok(token) => {
                            let backend = Arc::new(TodoistBackend::new(
                                token,
                                Some(backend_id.clone()),
                                Some(instance_config.name.clone()),
                            ));
                            self.backend_registry.add_backend(backend)?;
                            info!("Successfully added Todoist backend '{}'", backend_id);
                        }
                        Err(_) => {
                            warn!("API token not found in environment variable '{}' for backend '{}'",
                                  token_env_var, backend_id);

                            // Update backend status in database
                            self.storage
                                .update_backend_sync_status(
                                    backend_id,
                                    None,
                                    Some(&format!("API token not found in env var '{}'", token_env_var)),
                                )
                                .await?;
                        }
                    }
                }
                backend_type => {
                    warn!("Unknown backend type '{}' for backend '{}'", backend_type, backend_id);

                    // Update backend status in database
                    self.storage
                        .update_backend_sync_status(
                            backend_id,
                            None,
                            Some(&format!("Unknown backend type: {}", backend_type)),
                        )
                        .await?;
                }
            }
        }

        info!("Sync coordinator initialization complete");
        Ok(())
    }

    /// Sync all enabled backends
    pub async fn sync_all_backends(&self) -> Result<SyncResults> {
        info!("Starting sync across all backends");

        let registered_backends = self.storage.get_registered_backends().await?;
        let mut results = SyncResults::new();

        for backend in registered_backends {
            if !backend.enabled {
                info!("Backend '{}' is disabled, skipping sync", backend.backend_id);
                continue;
            }

            info!("Syncing backend '{}'", backend.backend_id);

            match self.sync_single_backend(&backend.backend_id).await {
                Ok(stats) => {
                    info!("Backend '{}' synced successfully: {:?}", backend.backend_id, stats);
                    results.successful.insert(backend.backend_id.clone(), stats);

                    // Update sync status
                    let _ = self.storage
                        .update_backend_sync_status(
                            &backend.backend_id,
                            Some(&Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()),
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    error!("Backend '{}' sync failed: {}", backend.backend_id, e);
                    results.failed.insert(backend.backend_id.clone(), e.to_string());

                    // Update error status
                    let _ = self.storage
                        .update_backend_sync_status(
                            &backend.backend_id,
                            None,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        }

        info!("Sync complete. Success: {}, Failed: {}",
              results.successful.len(), results.failed.len());

        Ok(results)
    }

    /// Sync a single backend by ID
    pub async fn sync_single_backend(&self, backend_id: &str) -> Result<SyncStats> {
        let backend = self.backend_registry.get_backend(backend_id)
            .map_err(|_| anyhow::anyhow!("Backend '{}' not found in registry", backend_id))?;

        let mut stats = SyncStats::default();

        info!("Syncing backend '{}' - fetching data from API", backend_id);

        // Fetch raw API objects from backend
        let projects = backend.get_projects().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch projects from backend '{}': {}", backend_id, e))?;

        let labels = backend.get_labels().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch labels from backend '{}': {}", backend_id, e))?;

        let sections = backend.get_sections().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch sections from backend '{}': {}", backend_id, e))?;

        let tasks = backend.get_tasks().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch tasks from backend '{}': {}", backend_id, e))?;

        info!("Fetched {} projects, {} labels, {} sections, {} tasks from backend '{}'",
              projects.len(), labels.len(), sections.len(), tasks.len(), backend_id);

        // Convert and store projects
        for project in projects {
            self.storage.sync_project_from_backend(backend_id, &project).await?;
            stats.projects_synced += 1;
        }

        // Convert and store labels
        for label in labels {
            self.storage.sync_label_from_backend(backend_id, &label).await?;
            stats.labels_synced += 1;
        }

        // Convert and store sections
        for section in sections {
            self.storage.sync_section_from_backend(backend_id, &section).await?;
        }

        // Convert and store tasks
        for task in tasks {
            self.storage.sync_task_from_backend(backend_id, &task).await?;
            stats.tasks_synced += 1;
        }

        info!("Backend '{}' sync completed: {} projects, {} labels, {} tasks stored",
              backend_id, stats.projects_synced, stats.labels_synced, stats.tasks_synced);

        Ok(stats)
    }

    /// Get sync status for all backends
    pub async fn get_sync_status(&self) -> Result<Vec<BackendSyncStatus>> {
        let registered_backends = self.storage.get_registered_backends().await?;

        let mut statuses = Vec::new();
        for backend in registered_backends {
            let is_available = self.backend_registry.get_backend(&backend.backend_id).is_ok();

            statuses.push(BackendSyncStatus {
                backend_id: backend.backend_id,
                backend_type: backend.backend_type,
                name: backend.name,
                enabled: backend.enabled,
                available: is_available,
                last_sync: backend.last_sync,
                sync_error: backend.sync_error,
            });
        }

        Ok(statuses)
    }
}

/// Results of a multi-backend sync operation
#[derive(Debug, Default)]
pub struct SyncResults {
    pub successful: HashMap<String, SyncStats>,
    pub failed: HashMap<String, String>,
}

impl SyncResults {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_synced(&self) -> usize {
        self.successful.len()
    }

    pub fn total_failed(&self) -> usize {
        self.failed.len()
    }

    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Statistics for a single backend sync
#[derive(Debug, Default, Clone)]
pub struct SyncStats {
    pub projects_synced: usize,
    pub labels_synced: usize,
    pub tasks_synced: usize,
}

impl SyncStats {
    pub fn total_items(&self) -> usize {
        self.projects_synced + self.labels_synced + self.tasks_synced
    }
}

/// Status information for a backend
#[derive(Debug, Clone)]
pub struct BackendSyncStatus {
    pub backend_id: String,
    pub backend_type: String,
    pub name: String,
    pub enabled: bool,
    pub available: bool, // Whether the backend is properly configured and available
    pub last_sync: Option<String>,
    pub sync_error: Option<String>,
}