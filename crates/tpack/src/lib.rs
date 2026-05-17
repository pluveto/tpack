#![doc = include_str!("../README.md")]

use sha2::{Digest, Sha256};

pub use tpack_core::*;

#[cfg(feature = "derive")]
pub use tpack_macros::{TpackDeserialize, TpackSerialize};

#[cfg(feature = "std")]
mod std_registry {
    use std::{
        collections::HashMap,
        fmt,
        sync::{Arc, RwLock},
    };

    use tpack_core::{Schema, SchemaRegistry};

    /// Returned when a `SchemaId` is already bound to a different schema.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SchemaBindingConflict {
        schema_id: Vec<u8>,
    }

    impl SchemaBindingConflict {
        pub fn schema_id(&self) -> &[u8] {
            &self.schema_id
        }
    }

    impl fmt::Display for SchemaBindingConflict {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "schema id already bound to a different schema: {:?}",
                self.schema_id
            )
        }
    }

    impl std::error::Error for SchemaBindingConflict {}

    /// Default in-memory schema registry for `std` deployments.
    ///
    /// `insert` and `insert_shared` are fail-closed: rebinding the same
    /// `SchemaId` to a different schema returns [`SchemaBindingConflict`] and
    /// keeps the original binding in place. Use `replace` or `replace_shared`
    /// only when the caller intentionally wants to override that binding after
    /// applying its own scope or freshness checks.
    #[derive(Debug, Clone, Default)]
    pub struct StdSchemaRegistry {
        inner: Arc<RwLock<HashMap<Vec<u8>, Arc<Schema>>>>,
    }

    impl StdSchemaRegistry {
        pub fn new() -> Self {
            Self::default()
        }

        /// Insert a schema binding when the `SchemaId` is unbound or already
        /// bound to an equivalent schema.
        ///
        /// Conflicting inserts leave the existing binding unchanged and return
        /// [`SchemaBindingConflict`].
        pub fn insert(
            &self,
            schema_id: impl Into<Vec<u8>>,
            schema: Schema,
        ) -> Result<(), SchemaBindingConflict> {
            self.insert_shared(schema_id, Arc::new(schema))
        }

        /// Insert a shared schema binding when the `SchemaId` is unbound or
        /// already bound to an equivalent schema.
        ///
        /// Conflicting inserts leave the existing binding unchanged and return
        /// [`SchemaBindingConflict`].
        pub fn insert_shared(
            &self,
            schema_id: impl Into<Vec<u8>>,
            schema: Arc<Schema>,
        ) -> Result<(), SchemaBindingConflict> {
            let schema_id = schema_id.into();
            let mut schemas = self.inner.write().expect("StdSchemaRegistry lock poisoned");
            match schemas.get(&schema_id) {
                Some(existing) if existing.as_ref() == schema.as_ref() => Ok(()),
                Some(_) => Err(SchemaBindingConflict { schema_id }),
                None => {
                    schemas.insert(schema_id, schema);
                    Ok(())
                }
            }
        }

        /// Replace the binding for a `SchemaId`, returning the previous schema
        /// when one existed.
        pub fn replace(
            &self,
            schema_id: impl Into<Vec<u8>>,
            schema: Schema,
        ) -> Option<Arc<Schema>> {
            self.replace_shared(schema_id, Arc::new(schema))
        }

        /// Replace the binding for a `SchemaId` with a shared schema, returning
        /// the previous schema when one existed.
        pub fn replace_shared(
            &self,
            schema_id: impl Into<Vec<u8>>,
            schema: Arc<Schema>,
        ) -> Option<Arc<Schema>> {
            self.inner
                .write()
                .expect("StdSchemaRegistry lock poisoned")
                .insert(schema_id.into(), schema)
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
pub use std_registry::{SchemaBindingConflict, StdSchemaRegistry};

#[cfg(feature = "serde_support")]
pub mod serde_support;

/// Derive the recommended SHA-256-based SchemaId bytes for a schema.
///
/// This facade helper keeps the stronger digest available for deployments that
/// want the draft's SHA-256 convention while leaving `tpack-core` free of the
/// heavier dependency. The hash input is the canonical schema descriptor bytes
/// returned by [`encode_schema`], excluding the header, envelope fields,
/// `SchemaLen`, and data bytes. The helper returns the bare 32-byte digest.
pub fn recommended_schema_id_sha256(schema: &Schema) -> Result<[u8; 32]> {
    let schema_bytes = encode_schema(schema)?;
    let digest = Sha256::digest(schema_bytes);
    let mut output = [0u8; 32];
    output.copy_from_slice(digest.as_slice());
    Ok(output)
}
