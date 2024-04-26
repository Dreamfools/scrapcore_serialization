use crate::registry::index::RegistryIndex;
use crate::registry::{CollectionHolder, CollectionItemId, PartialRegistry, SerializationRegistry};
use crate::serialization::error::DeserializationError;
use crate::serialization::{DeserializeModel, SerializationFallback};
use crate::ItemId;
use serde::{Deserialize, Serialize};

/// Item that can either be referenced by ID or have [Data] inline
#[derive(Debug, Clone)]
pub enum InlineOrId<Data> {
    Id(CollectionItemId<Data>),
    Inline(Data),
}

/// Serialized form of [InlineOrId]
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum InlineOrIdSerialized<DataSerialized> {
    Id(ItemId),
    Inline(DataSerialized),
}

impl<Data: SerializationFallback> SerializationFallback for InlineOrId<Data> {
    type Fallback = InlineOrIdSerialized<Data::Fallback>;
}

impl<Registry: PartialRegistry, Data, DataSerialized: DeserializeModel<Data, Registry>>
    DeserializeModel<InlineOrId<Data>, Registry> for InlineOrIdSerialized<DataSerialized>
where
    for<'a> &'a str: DeserializeModel<CollectionItemId<Data>, Registry>,
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<InlineOrId<Data>, DeserializationError<Registry>> {
        Ok(match self {
            InlineOrIdSerialized::Id(id) => InlineOrId::Id(id.deserialize(registry)?),
            InlineOrIdSerialized::Inline(data) => InlineOrId::Inline(data.deserialize(registry)?),
        })
    }
}

impl<Data> RegistryIndex<Data> for InlineOrId<Data> {
    fn get<'a, Registry: SerializationRegistry + CollectionHolder<Data>>(
        &'a self,
        registry: &'a Registry,
    ) -> &'a Data {
        match self {
            InlineOrId::Id(id) => &registry.get_collection()[*id].data,
            InlineOrId::Inline(data) => data,
        }
    }
}
