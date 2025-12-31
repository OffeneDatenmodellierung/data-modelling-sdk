//! Browser storage backend
//!
//! Implements StorageBackend for browser storage APIs (IndexedDB/localStorage).
//! Used by WASM apps for offline mode.

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
use super::{StorageBackend, StorageError};
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
use async_trait::async_trait;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
use wasm_bindgen::prelude::*;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
use wasm_bindgen_futures::JsFuture;
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
use web_sys::{IdbDatabase, IdbTransactionMode, Storage};

/// Browser storage backend using IndexedDB and localStorage
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub struct BrowserStorageBackend {
    db_name: String,
    store_name: String,
}

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
impl BrowserStorageBackend {
    /// Create a new browser storage backend
    ///
    /// # Arguments
    ///
    /// * `db_name` - IndexedDB database name
    /// * `store_name` - Object store name within the database
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::storage::browser::BrowserStorageBackend;
    ///
    /// let backend = BrowserStorageBackend::new("data_models", "tables");
    /// ```
    pub fn new(db_name: impl Into<String>, store_name: impl Into<String>) -> Self {
        Self {
            db_name: db_name.into(),
            store_name: store_name.into(),
        }
    }

    /// Get IndexedDB database instance
    async fn get_db(&self) -> Result<IdbDatabase, StorageError> {
        // Open IndexedDB database
        let window = web_sys::window()
            .ok_or_else(|| StorageError::BackendError("Window not available".to_string()))?;

        let idb_factory = window
            .indexed_db()
            .map_err(|e| StorageError::BackendError(format!("IndexedDB not available: {:?}", e)))?
            .ok_or_else(|| StorageError::BackendError("IndexedDB factory is None".to_string()))?;

        let open_request = idb_factory.open_with_u32(&self.db_name, 1).map_err(|e| {
            StorageError::BackendError(format!("Failed to open IndexedDB: {:?}", e))
        })?;

        // Set up onupgradeneeded handler
        let store_name_clone = self.store_name.clone();
        let onupgradeneeded = Closure::wrap(Box::new(
            move |event: &web_sys::IdbVersionChangeEvent| {
                if let Some(target) = event.target() {
                    if let Some(request) = target.dyn_ref::<web_sys::IdbOpenDbRequest>() {
                        if let Ok(result) = request.result() {
                            if let Some(db_result) = result.dyn_ref::<IdbDatabase>() {
                                // Create object store if it doesn't exist
                                // In onupgradeneeded, we can directly create the store
                                if let Err(e) = db_result.create_object_store(&store_name_clone) {
                                    web_sys::console::log_1(&format!("Failed to create object store (may already exist): {:?}", e).into());
                                }
                            }
                        }
                    }
                }
            },
        )
            as Box<dyn FnMut(&web_sys::IdbVersionChangeEvent)>);

        open_request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
        onupgradeneeded.forget();

        // Convert request to promise and await it
        let promise = js_sys::Promise::from(wasm_bindgen::JsValue::from(open_request));
        let result = JsFuture::from(promise).await.map_err(|e| {
            StorageError::BackendError(format!("Failed to open IndexedDB: {:?}", e))
        })?;

        // Extract database from result
        let request = result
            .dyn_ref::<web_sys::IdbOpenDbRequest>()
            .ok_or_else(|| {
                StorageError::BackendError("Result is not IdbOpenDbRequest".to_string())
            })?;

        let db_result = request.result().map_err(|e| {
            StorageError::BackendError(format!("Failed to get database result: {:?}", e))
        })?;

        db_result.dyn_into::<IdbDatabase>().map_err(|e| {
            StorageError::BackendError(format!("Failed to convert to IdbDatabase: {:?}", e))
        })
    }

    /// Get localStorage instance
    fn get_local_storage(&self) -> Result<Storage, StorageError> {
        let window = web_sys::window()
            .ok_or_else(|| StorageError::BackendError("Window not available".to_string()))?;

        window
            .local_storage()
            .map_err(|e| {
                StorageError::BackendError(format!("localStorage not available: {:?}", e))
            })?
            .ok_or_else(|| StorageError::BackendError("localStorage is None".to_string()))
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
#[async_trait(?Send)]
impl StorageBackend for BrowserStorageBackend {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        // Try IndexedDB first (for larger files)
        let db = self.get_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|e| {
                StorageError::BackendError(format!("Failed to create transaction: {:?}", e))
            })?;

        let request = {
            let store = transaction.object_store(&self.store_name).map_err(|e| {
                StorageError::BackendError(format!("Failed to get object store: {:?}", e))
            })?;

            store
                .get(&JsValue::from_str(path))
                .map_err(|e| StorageError::BackendError(format!("Failed to get value: {:?}", e)))?
            // store is dropped here
        };

        let result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(
            wasm_bindgen::JsValue::from(request),
        ))
        .await
        .map_err(|e| {
            StorageError::BackendError(format!("Failed to read from IndexedDB: {:?}", e))
        })?;

        if result.is_undefined() {
            // Fall back to localStorage
            let storage = self.get_local_storage()?;
            let value = storage.get_item(path).map_err(|e| {
                StorageError::BackendError(format!("Failed to read from localStorage: {:?}", e))
            })?;

            if let Some(value) = value {
                // Convert string to bytes
                Ok(value.as_bytes().to_vec())
            } else {
                Err(StorageError::FileNotFound(path.to_string()))
            }
        } else {
            // Convert JsValue to Vec<u8>
            let array = js_sys::Uint8Array::from(result);
            Ok(array.to_vec())
        }
    }

    async fn write_file(&self, path: &str, content: &[u8]) -> Result<(), StorageError> {
        // Use IndexedDB for larger files, localStorage for smaller ones
        if content.len() > 5 * 1024 * 1024 {
            // Use IndexedDB for files > 5MB
            let db = self.get_db().await?;
            let transaction = db
                .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
                .map_err(|e| {
                    StorageError::BackendError(format!("Failed to create transaction: {:?}", e))
                })?;

            {
                let store = transaction.object_store(&self.store_name).map_err(|e| {
                    StorageError::BackendError(format!("Failed to get object store: {:?}", e))
                })?;

                let array = js_sys::Uint8Array::new_with_length(content.len() as u32);
                array.copy_from(content);

                store
                    .put_with_key(&array.into(), &JsValue::from_str(path))
                    .map_err(|e| {
                        StorageError::BackendError(format!("Failed to write to IndexedDB: {:?}", e))
                    })?;
                // store is dropped here
            }

            // Now await transaction completion without holding store reference
            JsFuture::from(js_sys::Promise::from(wasm_bindgen::JsValue::from(
                transaction,
            )))
            .await
            .map_err(|e| {
                StorageError::BackendError(format!("Failed to commit transaction: {:?}", e))
            })?;
        } else {
            // Use localStorage for smaller files
            let storage = self.get_local_storage()?;
            let value = String::from_utf8(content.to_vec())
                .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;

            storage.set_item(path, &value).map_err(|e| {
                StorageError::BackendError(format!("Failed to write to localStorage: {:?}", e))
            })?;
        }

        Ok(())
    }

    async fn list_files(&self, dir: &str) -> Result<Vec<String>, StorageError> {
        let db = self.get_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|e| {
                StorageError::BackendError(format!("Failed to create transaction: {:?}", e))
            })?;

        let request = {
            let store = transaction.object_store(&self.store_name).map_err(|e| {
                StorageError::BackendError(format!("Failed to get object store: {:?}", e))
            })?;

            store
                .get_all()
                .map_err(|e| StorageError::BackendError(format!("Failed to get all: {:?}", e)))?
            // store is dropped here
        };

        let result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(
            wasm_bindgen::JsValue::from(request),
        ))
        .await
        .map_err(|e| StorageError::BackendError(format!("Failed to list files: {:?}", e)))?;

        let array = js_sys::Array::from(&result);
        let mut files = Vec::new();

        for i in 0..array.length() {
            if let Some(key) = array.get(i).as_string() {
                if key.starts_with(dir) {
                    files.push(key);
                }
            }
        }

        Ok(files)
    }

    async fn file_exists(&self, path: &str) -> Result<bool, StorageError> {
        // Check IndexedDB first
        match self.read_file(path).await {
            Ok(_) => Ok(true),
            Err(StorageError::FileNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        // Try IndexedDB first
        let db = self.get_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| {
                StorageError::BackendError(format!("Failed to create transaction: {:?}", e))
            })?;

        {
            let store = transaction.object_store(&self.store_name).map_err(|e| {
                StorageError::BackendError(format!("Failed to get object store: {:?}", e))
            })?;

            store.delete(&JsValue::from_str(path)).map_err(|e| {
                StorageError::BackendError(format!("Failed to delete from IndexedDB: {:?}", e))
            })?;
            // store is dropped here
        }

        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(wasm_bindgen::JsValue::from(
            transaction,
        )))
        .await
        .map_err(|e| {
            StorageError::BackendError(format!("Failed to commit transaction: {:?}", e))
        })?;

        // Also try localStorage
        if let Ok(storage) = self.get_local_storage() {
            let _ = storage.remove_item(path);
        }

        Ok(())
    }

    async fn create_dir(&self, _path: &str) -> Result<(), StorageError> {
        // Browser storage doesn't have directories, but we can use path prefixes
        // This is a no-op for browser storage
        Ok(())
    }

    async fn dir_exists(&self, path: &str) -> Result<bool, StorageError> {
        // Check if any files exist with this prefix
        let files = self.list_files(path).await?;
        Ok(!files.is_empty())
    }
}
