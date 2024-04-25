use crate::registry::PartialRegistry;
use crate::serialization::error::DeserializationError;
use crate::serialization::DeserializeModel;

/// Basically just an extension method to conveniently deserialize item from its
/// serialized form
pub trait DeserializeFrom<Registry: PartialRegistry>: Sized {
    fn deserialize_from<U>(
        data: U,
        registry: &mut Registry,
    ) -> Result<Self, DeserializationError<Registry>>
    where
        U: DeserializeModel<Self, Registry>,
    {
        data.deserialize(registry)
    }
}

impl<Registry: PartialRegistry, T> DeserializeFrom<Registry> for T {}
