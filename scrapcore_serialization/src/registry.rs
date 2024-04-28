use ahash::AHashMap;
use slabmap::{SlabMap, SlabMapId};
use std::error::Error;
use std::fmt::{Debug, Display};

use crate::registry::entry::{RegistryEntry, RegistryEntrySerialized};
use crate::registry::kind::{AssetKindProvider, ItemKindProvider};
use crate::registry::path_identifier::PathIdentifier;
use crate::serialization::error::internal::InternalDeserializationError;
use crate::serialization::error::DeserializationError;
use crate::serialization::{DeserializeModel, SerializationFallback};
use crate::{AssetName, ItemId};

pub mod entry;
pub mod finalize;
pub mod index;
pub mod inline;
pub mod insert;
pub mod kind;
pub mod path_identifier;
pub mod reverse_id;

/// Collection of items in a registry
pub type ItemCollection<T> = SlabMap<ItemId, RegistryEntry<T>>;

/// Id of an item in the collection
pub type CollectionItemId<T> = SlabMapId<RegistryEntry<T>>;

/// Storage type for items in partial collection
#[derive(Debug)]
pub enum MaybeRawItem<T, Serialized> {
    /// Raw serialized item
    Raw(RegistryEntrySerialized<Serialized>),
    /// Item that is currently in process of being deserialized
    Reserved(CollectionItemId<T>),
    /// Fully deserialized item
    Deserialized(RegistryEntry<T>),
}

/// Collection of items in a "partial" registry, with some items having IDs
/// assigned but not yet filled with an actual data
pub type PartialItemCollection<T, Serialized> =
    SlabMap<ItemId, (PathIdentifier, MaybeRawItem<T, Serialized>)>;

/// Singleton item in a registry
pub type Singleton<T> = T;

#[derive(Debug)]
pub enum MaybeRawSingleton<T: SerializationFallback> {
    Raw(T::Fallback),
    Deserialized(Singleton<T>),
}
/// Singleton item in a "partial" registry
pub type PartialSingleton<T> = Option<(PathIdentifier, MaybeRawSingleton<T>)>;

/// Collection of assets in a registry
pub type AssetsCollection<T> = AHashMap<AssetName, (T, PathIdentifier)>;

/// Registry trait for collections of items
pub trait CollectionHolder<Value>: SerializationRegistry + ItemKindProvider<Value> {
    fn get_collection(&self) -> &ItemCollection<Value>;
    fn get_collection_mut(&mut self) -> &mut ItemCollection<Value>;
}

/// Registry trait for singletons
pub trait SingletonHolder<Value>: SerializationRegistry + ItemKindProvider<Value> {
    fn get_singleton(&self) -> &Singleton<Value>;
    fn get_singleton_mut(&mut self) -> &mut Singleton<Value>;
}

/// Trait for "partial" registries used during deserialization
pub trait PartialCollectionHolder<Value>:
    PartialRegistry + ItemKindProvider<Value> + Sized
{
    type Serialized: DeserializeModel<Value, Self>;
    fn get_collection(&mut self) -> &mut PartialItemCollection<Value, Self::Serialized>;
}

/// Trait for "partial" registries used during deserialization
pub trait PartialSingletonHolder<Value: SerializationFallback>:
    PartialRegistry + ItemKindProvider<Value>
{
    // &mut Option usage is intentional so None can be replaced with Some
    fn get_singleton(&mut self) -> &mut PartialSingleton<Value>;
}

/// Registry trait for assets
pub trait AssetsHolder<Value>: SerializationRegistry + AssetKindProvider<Value> {
    fn get_assets(&self) -> &AssetsCollection<Value>;
    fn get_assets_mut(&mut self) -> &mut AssetsCollection<Value>;
}

/// Base trait for all registry-related types, providing common types
pub trait SerializationRegistry: Debug {
    /// Type indicating kind of registry or singleton items
    type ItemKind: Debug + Clone + Display;

    /// Type indicating kind of assets
    type AssetKind: Debug + Clone + Display;

    /// Custom error kind emitted during deserialization
    type Error: Error + Clone;
}

pub trait PartialRegistry: SerializationRegistry {
    /// Marks registry as poisoned, preventing further deserialization
    fn poison(&mut self);

    /// Determines whenever registry is poisoned
    fn is_poisoned(&self) -> bool;
}

/// Runs the closure on the partial registry, returning the error if registry
/// is already poisoned, and poisoning the registry is error happens during
/// evaluation
#[inline(always)]
pub fn poison_on_err<Registry: PartialRegistry, T>(
    registry: &mut Registry,
    func: impl FnOnce(&mut Registry) -> Result<T, DeserializationError<Registry>>,
) -> Result<T, DeserializationError<Registry>> {
    if registry.is_poisoned() {
        return Err(InternalDeserializationError::PoisonedRegistry.into());
    }
    let result = func(registry);
    if result.is_err() {
        registry.poison();
    }

    result
}
