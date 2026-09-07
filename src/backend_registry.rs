//! Backend registry for managing backend instances.
//!
//! This module provides the `BackendRegistry`, which holds the app's configured
//! backends: it persists their configuration in the database and keeps the
//! in-memory backend instance the sync service operates on.

use anyhow::Result;
use log::info;
use sea_orm::{ActiveValue, IntoActiveModel};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::backend::{factory, Backend};
use crate::entities::backend;
use crate::repositories::BackendRepository;
use crate::storage::LocalStorage;

/// Type alias for the backend instances map.
type BackendMap = HashMap<Uuid, Arc<Box<dyn Backend>>>;

/// Registry for managing backend instances and their configurations.
///
/// The `BackendRegistry` is responsible for:
/// - Creating and caching backend instances
/// - Persisting backend configuration to the database
pub struct BackendRegistry {
    storage: Arc<Mutex<LocalStorage>>,
    backends: Arc<Mutex<BackendMap>>,
}

impl BackendRegistry {
    /// Create a new backend registry.
    ///
    /// # Arguments
    /// * `storage` - Shared storage instance
    ///
    /// # Returns
    /// A new `BackendRegistry` instance
    pub fn new(storage: Arc<Mutex<LocalStorage>>) -> Self {
        Self {
            storage,
            backends: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a backend instance by UUID.
    ///
    /// # Arguments
    /// * `uuid` - Backend UUID
    ///
    /// # Returns
    /// Arc to the backend instance
    ///
    /// # Errors
    /// Returns error if backend is not found
    pub async fn get_backend(&self, uuid: &Uuid) -> Result<Arc<Box<dyn Backend>>> {
        let backends = self.backends.lock().await;
        backends
            .get(uuid)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Backend not found: {}", uuid))
    }

    /// List all backend configurations from the database.
    ///
    /// # Returns
    /// Vector of backend models
    ///
    /// # Errors
    /// Returns error if database access fails
    pub async fn list_backends(&self) -> Result<Vec<backend::Model>> {
        let storage = self.storage.lock().await;
        BackendRepository::get_all(&storage.conn).await
    }

    /// Add a new backend.
    ///
    /// # Arguments
    /// * `backend_type` - Backend type (e.g., "todoist")
    /// * `name` - Human-readable name
    /// * `credentials` - JSON-encoded credentials
    ///
    /// # Returns
    /// UUID of the created backend
    ///
    /// # Errors
    /// Returns error if backend creation fails or database insert fails
    pub async fn add_backend(&self, backend_type: String, name: String, credentials: String) -> Result<Uuid> {
        // Validate by creating instance first
        let backend_instance = factory::create_backend(&backend_type, &credentials)?;

        let uuid = Uuid::new_v4();

        let backend_model = backend::ActiveModel {
            uuid: ActiveValue::Set(uuid),
            backend_type: ActiveValue::Set(backend_type.clone()),
            name: ActiveValue::Set(name.clone()),
            credentials: ActiveValue::Set(credentials),
        };

        let storage = self.storage.lock().await;
        BackendRepository::create(&storage.conn, backend_model).await?;

        // Add to in-memory cache
        let mut backends = self.backends.lock().await;
        backends.insert(uuid, Arc::new(backend_instance));

        info!("✅ Added backend: {} ({})", name, backend_type);
        Ok(uuid)
    }

    /// Update an existing backend.
    ///
    /// # Arguments
    /// * `uuid` - Backend UUID
    /// * `name` - Optional new name
    /// * `credentials` - Optional new credentials
    ///
    /// # Errors
    /// Returns error if backend not found or update fails
    pub async fn update_backend(&self, uuid: &Uuid, name: Option<String>, credentials: Option<String>) -> Result<()> {
        let storage = self.storage.lock().await;

        let backend_model = BackendRepository::get_by_uuid(&storage.conn, uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Backend not found: {}", uuid))?;

        let backend_type = backend_model.backend_type.clone();
        let mut active_model = backend_model.into_active_model();

        if let Some(name) = name {
            active_model.name = ActiveValue::Set(name);
        }

        // If credentials changed, validate and recreate instance
        if let Some(ref new_credentials) = credentials {
            let backend_instance = factory::create_backend(&backend_type, new_credentials)?;
            active_model.credentials = ActiveValue::Set(new_credentials.clone());

            // Update in-memory cache
            let mut backends = self.backends.lock().await;
            backends.insert(*uuid, Arc::new(backend_instance));
        }

        BackendRepository::update(&storage.conn, active_model).await?;

        info!("✅ Updated backend: {}", uuid);
        Ok(())
    }

    /// Get the storage instance (for creating SyncService instances).
    pub fn storage(&self) -> Arc<Mutex<LocalStorage>> {
        self.storage.clone()
    }
}
