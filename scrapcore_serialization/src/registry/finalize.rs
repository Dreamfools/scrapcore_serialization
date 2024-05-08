//! Utilities for converting raw & partial registry into a full registry. End
//! user code is unlikely to use these methods directly
//!
//! If you implement registry manually (why?), the intended process is to
//! first use [process_raw_collection] to convert all raw items into partial items,
//! and then use [convert_partial_collection] on each collection to construct the full
//! registry, then the same for singletons, with [process_raw_singleton] and
//! [convert_partial_singleton]
use itertools::Itertools;

use crate::registry::kind::ItemKindProvider;
use crate::registry::{
    poison_on_err, ItemCollection, MaybeRawItem, MaybeRawSingleton, PartialCollectionHolder,
    PartialItemCollection, PartialSingleton, PartialSingletonHolder, Singleton,
};
use crate::serialization::error::internal::InternalDeserializationError;
use crate::serialization::error::{
    DeserializationError, DeserializationErrorKind, DeserializationErrorStackItem,
};
use crate::serialization::{DeserializeModel, SerializationFallback};

/// Processes all raw items into partial collection
pub fn process_raw_collection<T: SerializationFallback, Registry: PartialCollectionHolder<T>>(
    registry: &mut Registry,
) -> Result<(), DeserializationError<Registry>>
where
    T::Fallback: DeserializeModel<T, Registry>,
{
    poison_on_err(registry, |registry| {
        let items = registry.get_collection();
        let ids = items.keys().cloned().collect_vec();
        for id in ids {
            id.deserialize(registry)?;
        }

        Ok(())
    })
}

/// Convert partial collection into item collection
pub fn convert_partial_collection<T, Registry: ItemKindProvider<T> + PartialCollectionHolder<T>>(
    raw: PartialItemCollection<T, Registry::Serialized>,
) -> Result<ItemCollection<T>, InternalDeserializationError<Registry>> {
    let mut out: ItemCollection<T> = Default::default();
    for (key, id, (_, value)) in raw.into_iter().sorted_by_key(|(_, id, _)| *id) {
        let value = match value {
            MaybeRawItem::HotReloading => {
                return Err(InternalDeserializationError::UnfilledHotReloadingSlot(
                    key,
                    Registry::kind(),
                ))
            }
            MaybeRawItem::Raw(item) => {
                return Err(
                    InternalDeserializationError::ConversionEntryNotDeserialized(
                        item.id,
                        Registry::kind(),
                    ),
                )
            }
            MaybeRawItem::Reserved(_) => {
                return Err(InternalDeserializationError::ConversionEntryReserved(
                    key,
                    Registry::kind(),
                ))
            }
            MaybeRawItem::Deserialized(item) => item,
        };
        let (inserted_id, _) = out.insert(key.clone(), value);
        let inserted_id = inserted_id.raw();
        if inserted_id != id {
            return Err(InternalDeserializationError::ConversionIdsDiverge {
                key,
                expected: id,
                got: inserted_id,
                kind: Registry::kind(),
            });
        }
    }

    Ok(out)
}

/// Processes all raw items into partial collection
pub fn process_raw_singleton<T: SerializationFallback, Registry: PartialSingletonHolder<T>>(
    registry: &mut Registry,
) -> Result<(), DeserializationError<Registry>>
where
    T::Fallback: DeserializeModel<T, Registry>,
{
    poison_on_err(registry, |registry| {
        let Some((path, singleton)) = std::mem::take(registry.get_singleton()) else {
            return Err(DeserializationErrorKind::MissingSingleton {
                kind: Registry::kind(),
            }
            .into_err());
        };

        let MaybeRawSingleton::Raw(item) = singleton else {
            // Already deserialized, skip
            return Ok(());
        };

        let deserialized = item.deserialize(registry).map_err(|e| {
            e.context(DeserializationErrorStackItem::ItemByPath(
                path.to_owned(),
                Registry::kind(),
            ))
        })?;

        *registry.get_singleton() = Some((path, MaybeRawSingleton::Deserialized(deserialized)));

        Ok(())
    })
}

/// Convert partial collection into item collection
pub fn convert_partial_singleton<T, Registry: ItemKindProvider<T> + PartialSingletonHolder<T>>(
    raw: PartialSingleton<T, Registry::Serialized>,
) -> Result<Singleton<T>, InternalDeserializationError<Registry>> {
    match raw {
        None => Err(InternalDeserializationError::ConversionMissingSingleton(
            Registry::kind(),
        )),
        Some((data, item)) => match item {
            MaybeRawSingleton::Raw(_) => Err(
                InternalDeserializationError::ConversionUnprocessedSingleton(
                    data,
                    Registry::kind(),
                ),
            ),
            MaybeRawSingleton::Deserialized(item) => Ok(item),
        },
    }
}
