use slabmap::SlabMapId;

use crate::registry::entry::RegistryEntry;
use crate::registry::{CollectionHolder, SerializationRegistry};

/// Trait for things that can be used to fetch [Data] from the registry
///
/// Be default, this is implemented for:
/// - [SlabMapId] for fetching items by their ID
/// - [Data] to just return a reference to itself
/// - [crate::registry::inline::InlineOrId] to pick one of the two above
pub trait RegistryIndex<Data> {
    fn get<'a, Registry: SerializationRegistry + CollectionHolder<Data>>(
        &'a self,
        registry: &'a Registry,
    ) -> &'a Data;
}

impl<Data> RegistryIndex<Data> for SlabMapId<RegistryEntry<Data>> {
    fn get<'a, Registry: SerializationRegistry + CollectionHolder<Data>>(
        &'a self,
        registry: &'a Registry,
    ) -> &'a Data {
        &registry.get_collection()[*self].data
    }
}

impl<Data> RegistryIndex<Data> for Data {
    fn get<'a, Registry: SerializationRegistry + CollectionHolder<Data>>(
        &'a self,
        _registry: &'a Registry,
    ) -> &'a Data {
        self
    }
}
