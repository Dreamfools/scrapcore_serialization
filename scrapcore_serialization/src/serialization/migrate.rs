use crate::registry::SerializationRegistry;
use crate::serialization::error::DeserializationError;

/// Trait for migrating serialized data
pub trait Migrate<Latest, Registry: SerializationRegistry> {
    /// Performs migration
    fn migrate(self, registry: &mut Registry) -> Result<Latest, DeserializationError<Registry>>;
}

// impl DeserializeModel<>
