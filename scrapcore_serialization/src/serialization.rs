use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::{BuildHasher, Hash};

use ahash::AHashMap;

use slabmap::SlabMapId;

use crate::registry::entry::{RegistryEntry, RegistryEntrySerialized};
use crate::registry::{
    poison_on_err, CollectionItemId, MaybeRawItem, PartialCollectionHolder, PartialRegistry,
};
use crate::serialization::error::internal::InternalDeserializationError;
use crate::serialization::error::{
    DeserializationError, DeserializationErrorKind, DeserializationErrorStackItem,
};
use crate::{ItemId, ItemIdRef};

pub mod box_wrapper;
pub mod error;

pub mod min_max;

pub mod helpers;

pub mod primitives;
pub mod string;

pub mod migrate;

#[cfg(feature = "bevy")]
pub mod bevy;

pub use min_max::ApplyMax;
pub use min_max::ApplyMin;

pub trait SerializationFallback {
    type Fallback: Debug;
}

pub trait DeserializeModel<T, Registry: PartialRegistry> {
    fn deserialize(self, registry: &mut Registry) -> Result<T, DeserializationError<Registry>>;
}

impl<Registry: PartialRegistry, T: DeserializeModel<R, Registry>, R>
    DeserializeModel<Option<R>, Registry> for Option<T>
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<Option<R>, DeserializationError<Registry>> {
        self.map(|e| e.deserialize(registry)).transpose()
    }
}

impl<T: SerializationFallback> SerializationFallback for Option<T> {
    type Fallback = Option<T::Fallback>;
}

// region Vec

impl<Registry: PartialRegistry, T: DeserializeModel<R, Registry>, R>
    DeserializeModel<Vec<R>, Registry> for Vec<T>
{
    #[inline]
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<Vec<R>, DeserializationError<Registry>> {
        self.into_iter()
            .enumerate()
            .map(|(i, e)| {
                e.deserialize(registry)
                    .map_err(|e| e.context(DeserializationErrorStackItem::Index(i)))
            })
            .collect()
    }
}

impl<T: SerializationFallback> SerializationFallback for Vec<T> {
    type Fallback = Vec<T::Fallback>;
}

// endregion

// region HashMap

impl<
        Registry: PartialRegistry,
        RawKey: DeserializeModel<Key, Registry> + Eq + Hash + Display,
        Key: Eq + Hash,
        RawValue: DeserializeModel<Value, Registry>,
        Value,
        RawHasher: BuildHasher,
        Hasher: BuildHasher + Default,
    > DeserializeModel<HashMap<Key, Value, Hasher>, Registry>
    for HashMap<RawKey, RawValue, RawHasher>
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<HashMap<Key, Value, Hasher>, DeserializationError<Registry>> {
        self.into_iter()
            .map(|(k, v)| {
                let key_str = k.to_string();
                let v = v.deserialize(registry).map_err(|e| {
                    e.context(DeserializationErrorStackItem::MapEntry(key_str.clone()))
                })?;
                let k = k
                    .deserialize(registry)
                    .map_err(|e| e.context(DeserializationErrorStackItem::MapKey(key_str)))?;
                Ok((k, v))
            })
            .collect()
    }
}

impl<Key: SerializationFallback, Value: SerializationFallback, Hasher: BuildHasher>
    SerializationFallback for HashMap<Key, Value, Hasher>
{
    type Fallback = AHashMap<Key::Fallback, Value::Fallback>;
}

// endregion

impl<Item> SerializationFallback for SlabMapId<Item> {
    type Fallback = ItemId;
}

impl<Registry: PartialRegistry, T> DeserializeModel<T, Registry> for String
where
    for<'a> &'a str: DeserializeModel<T, Registry>,
{
    fn deserialize(self, registry: &mut Registry) -> Result<T, DeserializationError<Registry>> {
        self.as_str().deserialize(registry)
    }
}

impl<'a, Registry: PartialCollectionHolder<Data>, Data: SerializationFallback>
    DeserializeModel<CollectionItemId<Data>, Registry> for ItemIdRef<'a>
where
    Data::Fallback: DeserializeModel<Data, Registry>,
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<CollectionItemId<Data>, DeserializationError<Registry>> {
        poison_on_err(registry, |registry| {
            let items = registry.get_collection();

            let id = items.key_to_id(self).ok_or_else(|| {
                DeserializationErrorKind::<Registry>::MissingItem(
                    self.to_string(),
                    Registry::kind(),
                )
            })?;
            let (_, item) = &mut items[id];

            let other = match item {
                MaybeRawItem::Raw(_) => {
                    let id = id.as_untyped().as_typed_unchecked();
                    let MaybeRawItem::Raw(other) =
                        std::mem::replace(item, MaybeRawItem::Reserved(id))
                    else {
                        unreachable!("Should ")
                    };
                    other
                }
                MaybeRawItem::Reserved(id) => return Ok(*id),
                MaybeRawItem::Deserialized(item) => return Ok(item.id),
            };

            other.deserialize(registry).map_err(|e| {
                e.context(DeserializationErrorStackItem::ItemByPath(
                    registry.get_collection()[id].0.clone(),
                    Registry::kind(),
                ))
            })
        })
    }
}

impl<Registry: PartialCollectionHolder<Data>, Data: SerializationFallback>
    DeserializeModel<CollectionItemId<Data>, Registry> for RegistryEntrySerialized<Data::Fallback>
where
    Data::Fallback: DeserializeModel<Data, Registry>,
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<CollectionItemId<Data>, DeserializationError<Registry>> {
        poison_on_err(registry, |registry| {
            let items = registry.get_collection();

            let id = items
                .key_to_id(&self.id)
                .ok_or_else(|| InternalDeserializationError::EntryNotRegistered)?;

            let (path, item) = &mut items[id];

            let (model_id, id) = match item {
                MaybeRawItem::Raw(_) => {
                    return Err(InternalDeserializationError::ConflictingRawEntry(
                        path.clone(),
                        Registry::kind(),
                    )
                    .into_err())
                }
                MaybeRawItem::Reserved(model_id) => (*model_id, id),
                MaybeRawItem::Deserialized(_) => {
                    return Err(InternalDeserializationError::ConflictingDeserializedEntry(
                        path.clone(),
                        Registry::kind(),
                    )
                    .into_err())
                }
            };
            let data = DeserializeModel::<Data, Registry>::deserialize(self.data, registry)?;
            let model = RegistryEntry { id: model_id, data };

            let items = registry.get_collection();
            let (path, item) = items
                .get_by_id_mut(id)
                .ok_or_else(|| InternalDeserializationError::EntryGoneDuringDeserialization)?;
            match item {
                MaybeRawItem::Raw(_) => {
                    return Err(InternalDeserializationError::EntryBecameRaw(
                        path.clone(),
                        Registry::kind(),
                    )
                    .into_err())
                }
                MaybeRawItem::Reserved(reserved_id) => {
                    if model_id != *reserved_id {
                        let reserved_id = reserved_id.as_untyped();
                        let id = registry
                            .get_collection()
                            .untyped_to_key(reserved_id)
                            .expect("Should have key for the reserved id");
                        return Err(InternalDeserializationError::EntryChangedId(
                            id.clone(),
                            Registry::kind(),
                        )
                        .into_err());
                    }
                    *item = MaybeRawItem::Deserialized(model);
                }
                MaybeRawItem::Deserialized(_) => {
                    return Err(InternalDeserializationError::EntryBecameDeserialized(
                        path.clone(),
                        Registry::kind(),
                    )
                    .into_err())
                }
            };

            Ok(model_id)
        })
        .map_err(|e| {
            e.context(DeserializationErrorStackItem::ItemById(
                self.id,
                Registry::kind(),
            ))
        })
    }
}
