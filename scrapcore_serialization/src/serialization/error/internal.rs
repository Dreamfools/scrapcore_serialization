use crate::registry::path_identifier::PathIdentifier;
use crate::registry::SerializationRegistry;
use crate::serialization::error::{DeserializationError, DeserializationErrorKind};
use crate::ItemId;
use thiserror::Error;

/// Internal errors that are results of bad application code organization,
/// rather than loaded mod issues
#[derive(Debug, Error, Clone)]
pub enum InternalDeserializationError<Registry: SerializationRegistry> {
    #[error(
        "Partial registry is poisoned. This is likely caused by ignoring some previous errors."
    )]
    PoisonedRegistry,
    #[error("Collection item was not registered before deserializing")]
    EntryNotRegistered,
    #[error("Reserved collection entry was deleted during item deserialization")]
    EntryGoneDuringDeserialization,
    #[error("Reserved {} collection entry was turned into Raw entry during deserialization: {}", .1, .0)]
    EntryBecameRaw(PathIdentifier, Registry::ItemKind),
    #[error("Reserved {} collection entry was turned into a different Deserialized entry during deserialization: {}", .1, .0)]
    EntryBecameDeserialized(PathIdentifier, Registry::ItemKind),
    #[error("Reserved {} collection entry has different ID after deserialization: {}", .1, .0,)]
    EntryChangedId(ItemId, Registry::ItemKind),
    #[error("Attempted to deserialize {} collection item while having a conflicting raw entry: {}", .1, .0)]
    ConflictingRawEntry(PathIdentifier, Registry::ItemKind),
    #[error("Attempted to deserialize {} collection item while having a conflicting deserialized entry: {}", .1, .0)]
    ConflictingDeserializedEntry(PathIdentifier, Registry::ItemKind),
    #[error("{} collection item was not deserialized before conversion: {}, was `process_raw_singleton` not invoked?", .1, .0)]
    ConversionEntryNotDeserialized(ItemId, Registry::ItemKind),
    #[error("{} collection item was left in reserved state before conversion: {}, was registry poisoned?", .1, .0)]
    ConversionEntryReserved(ItemId, Registry::ItemKind),
    #[error("Got divergence in collection IDs for item {}({}): expected to be saved at ID {} but instead got {}", .kind, .key, .expected, .got)]
    ConversionIdsDiverge {
        key: ItemId,
        expected: usize,
        got: usize,
        kind: Registry::ItemKind,
    },
    #[error("Singleton {} is missing during conversion, this should have been caught during `process_raw_singleton` invocation", .0)]
    ConversionMissingSingleton(Registry::ItemKind),
    #[error("Singleton {}({}) is not processed, was `process_raw_singleton` not invoked?", .1, .0)]
    ConversionUnprocessedSingleton(PathIdentifier, Registry::ItemKind),
}

impl<Registry: SerializationRegistry> InternalDeserializationError<Registry> {
    pub fn into_err(self) -> DeserializationError<Registry> {
        self.into()
    }
}

impl<Registry: SerializationRegistry> From<InternalDeserializationError<Registry>>
    for DeserializationError<Registry>
{
    fn from(value: InternalDeserializationError<Registry>) -> Self {
        DeserializationErrorKind::InternalError(value).into()
    }
}
