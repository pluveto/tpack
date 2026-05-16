pub use tpack_core::*;

#[cfg(feature = "derive")]
pub use tpack_macros::{TpackDeserialize, TpackSerialize};

#[cfg(feature = "std")]
mod std_registry {
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    use tpack_core::{Schema, SchemaRegistry};

    #[derive(Debug, Clone, Default)]
    pub struct StdSchemaRegistry {
        inner: Arc<RwLock<HashMap<Vec<u8>, Arc<Schema>>>>,
    }

    impl StdSchemaRegistry {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn insert(&self, schema_id: impl Into<Vec<u8>>, schema: Schema) {
            if let Ok(mut schemas) = self.inner.write() {
                schemas.insert(schema_id.into(), Arc::new(schema));
            }
        }

        pub fn insert_shared(&self, schema_id: impl Into<Vec<u8>>, schema: Arc<Schema>) {
            if let Ok(mut schemas) = self.inner.write() {
                schemas.insert(schema_id.into(), schema);
            }
        }

        pub fn remove(&self, schema_id: &[u8]) -> Option<Arc<Schema>> {
            self.inner.write().ok()?.remove(schema_id)
        }

        pub fn len(&self) -> usize {
            self.inner.read().map(|schemas| schemas.len()).unwrap_or(0)
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }

    impl SchemaRegistry for StdSchemaRegistry {
        fn get(&self, schema_id: &[u8]) -> Option<Arc<Schema>> {
            self.inner.read().ok()?.get(schema_id).cloned()
        }
    }
}

#[cfg(feature = "std")]
pub use std_registry::StdSchemaRegistry;

#[cfg(feature = "serde_support")]
pub mod serde_support;
