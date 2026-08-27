//! Backend registry for managing multiple backend instances.
//!
//! This module provides the `BackendRegistry` which manages the lifecycle of
//! backend instances, including loading from database, creating instances,
//! and coordinating sync operations across multiple backends.

use anyhow::Result;
use log::{error, info};
use sea_orm::{ActiveValue, EntityTrait, IntoActiveModel};
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
/// - Loading backend configurations from the database
/// - Creating and caching backend instances
/// - Managing backend lifecycle (add/remove/enable/disable)
/// - Coordinating sync operations across multiple backends
pub struct BackendRegistry {
    storage: Arc<Mutex<LocalStorage>>,
    backends: Arc<Mutex<BackendMap>>,
}

/// Namespace for deriving stable backend UUIDs (a fixed, arbitrary v4 UUID).
const BACKEND_NAMESPACE: Uuid = Uuid::from_u128(0x6b1c_2f0e_9d47_4a35_8f21_5c7e_3a9b_0d64);

/// Derive the UUID for a backend from its type and name.
///
/// Deterministic: the same pair yields the same UUID on every launch. The local cache keys
/// every project, task, label, and section to this value, so a random UUID per launch would
/// silently duplicate the entire dataset against a persistent database.
#[must_use]
pub fn derive_backend_uuid(backend_type: &str, name: &str) -> Uuid {
    Uuid::new_v5(&BACKEND_NAMESPACE, format!("{backend_type}:{name}").as_bytes())
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

    /// Load all backends from the database and create their instances.
    ///
    /// This should be called once during application initialization.
    ///
    /// # Errors
    /// Returns error if database access fails or backend creation fails
    pub async fn load_backends(&self) -> Result<()> {
        let storage = self.storage.lock().await;
        let backend_models = BackendRepository::get_all(&storage.conn).await?;

        info!("Loading {} backend(s) from database", backend_models.len());

        let mut backends = self.backends.lock().await;

        for backend_model in backend_models {
            match Self::create_backend_instance(&backend_model) {
                Ok(backend_instance) => {
                    info!(
                        "✅ Loaded backend: {} ({})",
                        backend_model.name, backend_model.backend_type
                    );
                    backends.insert(backend_model.uuid, Arc::new(backend_instance));
                }
                Err(e) => {
                    error!(
                        "❌ Failed to load backend {} ({}): {}",
                        backend_model.name, backend_model.backend_type, e
                    );
                    // Continue loading other backends
                }
            }
        }

        info!("Loaded {} backend instance(s)", backends.len());
        Ok(())
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

    /// Get all backend instances.
    ///
    /// # Returns
    /// Vector of all backend instances
    pub async fn get_all_backends(&self) -> Vec<Arc<Box<dyn Backend>>> {
        let backends = self.backends.lock().await;
        backends.values().cloned().collect()
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

    /// List all enabled backend configurations from the database.
    ///
    /// # Returns
    /// Vector of enabled backend models
    ///
    /// # Errors
    /// Returns error if database access fails
    pub async fn list_enabled_backends(&self) -> Result<Vec<backend::Model>> {
        let storage = self.storage.lock().await;
        BackendRepository::get_enabled(&storage.conn).await
    }

    /// Add a new backend.
    ///
    /// # Arguments
    /// * `backend_type` - Backend type (e.g., "todoist")
    /// * `name` - Human-readable name
    /// * `credentials` - JSON-encoded credentials
    /// * `settings` - JSON-encoded settings
    ///
    /// # Returns
    /// UUID of the created backend
    ///
    /// # Errors
    /// Returns error if backend creation fails or database insert fails
    pub async fn add_backend(
        &self,
        backend_type: String,
        name: String,
        credentials: String,
        settings: String,
    ) -> Result<Uuid> {
        use sea_orm::sea_query::OnConflict;

        // Validate by creating instance first
        let backend_instance = factory::create_backend(&backend_type, &credentials)?;

        let storage = self.storage.lock().await;

        // Reuse the row a previous version wrote, whatever UUID it has, so the cache that hangs
        // off that `backend_uuid` stays attached to this backend. Releases predating the derived
        // UUID wrote a random v4 here; inserting the derived v5 instead would both violate
        // `idx_backends_type_name` (the app would fail to start) and, if that constraint were
        // widened rather than respected, strand every cached project/task under a dead
        // `backend_uuid` — which the repositories' unfiltered `get_all` would then render as a
        // fully duplicated task list.
        let uuid = match BackendRepository::get_by_type_and_name(&storage.conn, &backend_type, &name).await? {
            Some(existing) => existing.uuid,
            None => derive_backend_uuid(&backend_type, &name),
        };

        let backend_model = backend::ActiveModel {
            uuid: ActiveValue::Set(uuid),
            backend_type: ActiveValue::Set(backend_type.clone()),
            name: ActiveValue::Set(name.clone()),
            is_enabled: ActiveValue::Set(true),
            credentials: ActiveValue::Set(credentials),
            settings: ActiveValue::Set(settings),
        };

        backend::Entity::insert(backend_model)
            .on_conflict(
                OnConflict::column(backend::Column::Uuid)
                    .update_columns([backend::Column::Name, backend::Column::Credentials, backend::Column::Settings])
                    .to_owned(),
            )
            .exec(&storage.conn)
            .await?;

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
    /// * `settings` - Optional new settings
    ///
    /// # Errors
    /// Returns error if backend not found or update fails
    pub async fn update_backend(
        &self,
        uuid: &Uuid,
        name: Option<String>,
        credentials: Option<String>,
        settings: Option<String>,
    ) -> Result<()> {
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

        if let Some(settings) = settings {
            active_model.settings = ActiveValue::Set(settings);
        }

        BackendRepository::update(&storage.conn, active_model).await?;

        info!("✅ Updated backend: {}", uuid);
        Ok(())
    }

    /// Remove a backend.
    ///
    /// # Arguments
    /// * `uuid` - Backend UUID
    ///
    /// # Errors
    /// Returns error if deletion fails
    pub async fn remove_backend(&self, uuid: &Uuid) -> Result<()> {
        let storage = self.storage.lock().await;
        BackendRepository::delete(&storage.conn, uuid).await?;

        // Remove from in-memory cache
        let mut backends = self.backends.lock().await;
        backends.remove(uuid);

        info!("✅ Removed backend: {}", uuid);
        Ok(())
    }

    /// Enable a backend.
    ///
    /// # Arguments
    /// * `uuid` - Backend UUID
    ///
    /// # Errors
    /// Returns error if backend not found or update fails
    pub async fn enable_backend(&self, uuid: &Uuid) -> Result<()> {
        self.set_enabled_status(uuid, true).await
    }

    /// Disable a backend.
    ///
    /// # Arguments
    /// * `uuid` - Backend UUID
    ///
    /// # Errors
    /// Returns error if backend not found or update fails
    pub async fn disable_backend(&self, uuid: &Uuid) -> Result<()> {
        self.set_enabled_status(uuid, false).await
    }

    /// Helper to set enabled status.
    async fn set_enabled_status(&self, uuid: &Uuid, enabled: bool) -> Result<()> {
        let storage = self.storage.lock().await;

        let backend_model = BackendRepository::get_by_uuid(&storage.conn, uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Backend not found: {}", uuid))?;

        let mut active_model = backend_model.into_active_model();
        active_model.is_enabled = ActiveValue::Set(enabled);

        BackendRepository::update(&storage.conn, active_model).await?;

        let status = if enabled { "enabled" } else { "disabled" };
        info!("✅ Backend {} {}", uuid, status);
        Ok(())
    }

    /// Create a backend instance from a backend model.
    ///
    /// # Arguments
    /// * `model` - Backend model from database
    ///
    /// # Returns
    /// Boxed backend instance
    ///
    /// # Errors
    /// Returns error if backend creation fails
    fn create_backend_instance(model: &backend::Model) -> Result<Box<dyn Backend>> {
        factory::create_backend(&model.backend_type, &model.credentials)
    }

    /// Get the storage instance (for creating SyncService instances).
    pub fn storage(&self) -> Arc<Mutex<LocalStorage>> {
        self.storage.clone()
    }
}
