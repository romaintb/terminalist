//! Backend registry for managing multiple backend instances
//!
//! This module provides a centralized registry for managing multiple backend instances,
//! allowing the application to work with different task management systems simultaneously.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;

use super::{BackendError, BackendStatus, FullBackend};

/// Registry for managing multiple backend instances
///
/// The BackendRegistry provides a centralized way to register, access, and manage
/// multiple backend instances. It supports operations like adding/removing backends,
/// getting backend status, and routing operations to specific backends.
pub struct BackendRegistry {
    /// Map of backend_id -> backend instance
    backends: HashMap<String, Arc<dyn FullBackend>>,
    /// Default backend ID to use when none is specified
    default_backend_id: Option<String>,
}

impl BackendRegistry {
    /// Create a new empty backend registry
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
            default_backend_id: None,
        }
    }

    /// Add a backend to the registry
    ///
    /// # Arguments
    /// * `backend` - The backend instance to add
    ///
    /// # Returns
    /// Returns an error if a backend with the same ID already exists
    pub fn add_backend(&mut self, backend: Arc<dyn FullBackend>) -> Result<()> {
        let backend_id = backend.backend_id().to_string();

        if self.backends.contains_key(&backend_id) {
            return Err(anyhow!(BackendError::InvalidConfiguration(format!(
                "Backend with ID '{}' already exists",
                backend_id
            ))));
        }

        // If this is the first backend, make it the default
        if self.backends.is_empty() {
            self.default_backend_id = Some(backend_id.clone());
        }

        self.backends.insert(backend_id, backend);
        Ok(())
    }

    /// Remove a backend from the registry
    ///
    /// # Arguments
    /// * `backend_id` - The ID of the backend to remove
    ///
    /// # Returns
    /// Returns the removed backend instance, or an error if not found
    pub fn remove_backend(&mut self, backend_id: &str) -> Result<Arc<dyn FullBackend>> {
        let backend = self.backends.remove(backend_id).ok_or_else(|| {
            anyhow!(BackendError::BackendNotFound(backend_id.to_string()))
        })?;

        // If we removed the default backend, pick a new default
        if self.default_backend_id.as_deref() == Some(backend_id) {
            self.default_backend_id = self.backends.keys().next().cloned();
        }

        Ok(backend)
    }

    /// Get a backend by ID
    ///
    /// # Arguments
    /// * `backend_id` - The ID of the backend to retrieve
    ///
    /// # Returns
    /// Returns the backend instance or an error if not found
    pub fn get_backend(&self, backend_id: &str) -> Result<Arc<dyn FullBackend>> {
        self.backends
            .get(backend_id)
            .cloned()
            .ok_or_else(|| anyhow!(BackendError::BackendNotFound(backend_id.to_string())))
    }

    /// Get the default backend
    ///
    /// # Returns
    /// Returns the default backend instance or an error if no backends are registered
    pub fn get_default_backend(&self) -> Result<Arc<dyn FullBackend>> {
        let backend_id = self.default_backend_id.as_ref().ok_or_else(|| {
            anyhow!(BackendError::BackendNotFound("No default backend set".to_string()))
        })?;

        self.get_backend(backend_id)
    }

    /// Set the default backend
    ///
    /// # Arguments
    /// * `backend_id` - The ID of the backend to set as default
    ///
    /// # Returns
    /// Returns an error if the backend ID doesn't exist
    pub fn set_default_backend(&mut self, backend_id: &str) -> Result<()> {
        if !self.backends.contains_key(backend_id) {
            return Err(anyhow!(BackendError::BackendNotFound(backend_id.to_string())));
        }

        self.default_backend_id = Some(backend_id.to_string());
        Ok(())
    }

    /// Get the default backend ID
    pub fn get_default_backend_id(&self) -> Option<&str> {
        self.default_backend_id.as_deref()
    }

    /// Get all registered backend IDs
    pub fn get_backend_ids(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    /// Get all registered backends
    pub fn get_backends(&self) -> Vec<Arc<dyn FullBackend>> {
        self.backends.values().cloned().collect()
    }

    /// Check if a backend exists
    pub fn has_backend(&self, backend_id: &str) -> bool {
        self.backends.contains_key(backend_id)
    }

    /// Get the number of registered backends
    pub fn len(&self) -> usize {
        self.backends.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }

    /// Get status for all backends
    ///
    /// # Returns
    /// Returns a map of backend_id -> backend status
    pub async fn get_all_backend_statuses(&self) -> HashMap<String, Result<BackendStatus>> {
        let mut statuses = HashMap::new();

        for (backend_id, backend) in &self.backends {
            let status = backend.get_status().await;
            statuses.insert(backend_id.clone(), status);
        }

        statuses
    }

    /// Get status for a specific backend
    ///
    /// # Arguments
    /// * `backend_id` - The ID of the backend to get status for
    ///
    /// # Returns
    /// Returns the backend status or an error if the backend doesn't exist
    pub async fn get_backend_status(&self, backend_id: &str) -> Result<BackendStatus> {
        let backend = self.get_backend(backend_id)?;
        backend.get_status().await
    }

    /// Test connection for all backends
    ///
    /// # Returns
    /// Returns a map of backend_id -> connection test result
    pub async fn test_all_connections(&self) -> HashMap<String, Result<()>> {
        let mut results = HashMap::new();

        for (backend_id, backend) in &self.backends {
            let result = backend.test_connection().await;
            results.insert(backend_id.clone(), result);
        }

        results
    }

    /// Test connection for a specific backend
    ///
    /// # Arguments
    /// * `backend_id` - The ID of the backend to test
    ///
    /// # Returns
    /// Returns Ok(()) if connection is successful, or an error
    pub async fn test_backend_connection(&self, backend_id: &str) -> Result<()> {
        let backend = self.get_backend(backend_id)?;
        backend.test_connection().await
    }

    /// Find backend by item ID
    ///
    /// This is a utility method for finding which backend an item belongs to
    /// based on some identifier pattern. For now, it returns the default backend,
    /// but this can be enhanced to support backend-specific ID patterns.
    ///
    /// # Arguments
    /// * `_item_id` - The item ID to find the backend for
    ///
    /// # Returns
    /// Returns the backend that should handle this item
    pub fn find_backend_for_item(&self, _item_id: &str) -> Result<Arc<dyn FullBackend>> {
        // For now, just return the default backend
        // In the future, this could analyze the item_id to determine the source backend
        // For example: "todoist:12345" -> todoist backend, "local:abc123" -> local backend
        self.get_default_backend()
    }

    /// Clear all backends from the registry
    pub fn clear(&mut self) {
        self.backends.clear();
        self.default_backend_id = None;
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::todoist::TodoistBackend;

    #[tokio::test]
    async fn test_backend_registry() {
        let mut registry = BackendRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        // Create a mock backend
        let backend = Arc::new(TodoistBackend::new(
            "test_token".to_string(),
            Some("test_backend".to_string()),
            Some("Test Backend".to_string()),
        ));

        // Add backend
        registry.add_backend(backend.clone()).unwrap();
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.has_backend("test_backend"));
        assert_eq!(registry.get_default_backend_id(), Some("test_backend"));

        // Get backend
        let retrieved = registry.get_backend("test_backend").unwrap();
        assert_eq!(retrieved.backend_id(), "test_backend");

        // Test default backend
        let default = registry.get_default_backend().unwrap();
        assert_eq!(default.backend_id(), "test_backend");

        // Remove backend
        let removed = registry.remove_backend("test_backend").unwrap();
        assert_eq!(removed.backend_id(), "test_backend");
        assert!(registry.is_empty());
        assert_eq!(registry.get_default_backend_id(), None);
    }

    #[tokio::test]
    async fn test_backend_registry_errors() {
        let mut registry = BackendRegistry::new();

        // Test getting non-existent backend
        let result = registry.get_backend("nonexistent");
        assert!(result.is_err());

        // Test getting default when empty
        let result = registry.get_default_backend();
        assert!(result.is_err());

        // Test removing non-existent backend
        let result = registry.remove_backend("nonexistent");
        assert!(result.is_err());
    }
}