use alloc::sync::Arc;

use crate::Schema;

pub trait SchemaRegistry {
    fn get(&self, schema_id: &[u8]) -> Option<Arc<Schema>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyRegistry;

impl SchemaRegistry for EmptyRegistry {
    fn get(&self, _schema_id: &[u8]) -> Option<Arc<Schema>> {
        None
    }
}

pub fn empty_registry() -> EmptyRegistry {
    EmptyRegistry
}

impl<T: SchemaRegistry + ?Sized> SchemaRegistry for &T {
    fn get(&self, schema_id: &[u8]) -> Option<Arc<Schema>> {
        (**self).get(schema_id)
    }
}
