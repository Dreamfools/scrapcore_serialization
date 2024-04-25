use crate::registry::PartialRegistry;
use crate::serialization::error::DeserializationError;
use crate::serialization::{DeserializeModel, SerializationFallback};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

/// Due to limitations of the trait system, we can't directly implement
/// [DeserializeModel] for String, so a wrapper is used instead
#[derive(Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SerializationStringWrapper(pub String);

impl Debug for SerializationStringWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl SerializationFallback for String {
    type Fallback = SerializationStringWrapper;
}

impl<Registry: PartialRegistry> DeserializeModel<String, Registry> for SerializationStringWrapper {
    #[inline(always)]
    fn deserialize(
        self,
        _registry: &mut Registry,
    ) -> Result<String, DeserializationError<Registry>> {
        Ok(self.0)
    }
}
