use crate::registry::path_identifier::PathIdentifier;
use crate::registry::{CollectionHolder, CollectionItemId, MaybeRawItem, PartialCollectionHolder};
use crate::serialization::error::internal::InternalDeserializationError;
use crate::serialization::error::DeserializationError;
use crate::ItemId;

/// Reserves IDs in the partial registry based on IDs from the main registry
pub fn reserve_ids<T, PartialRegistry: PartialCollectionHolder<T>>(
    registry: &PartialRegistry::Registry,
    partial: &mut PartialRegistry,
) -> Result<(), DeserializationError<PartialRegistry>>
where
    PartialRegistry::Registry: CollectionHolder<T>,
{
    let source = registry.get_collection();

    let target = partial.get_collection();

    for (k, id) in source.keys_ids() {
        let k: &ItemId = k;
        let id: CollectionItemId<T> = id;

        let (inserted_id, _) = target.insert(
            k.clone(),
            (
                PathIdentifier::from_components([]),
                MaybeRawItem::HotReloading,
            ),
        );
        let inserted_id = inserted_id.raw();
        if inserted_id != id.raw() {
            return Err(InternalDeserializationError::ConversionIdsDiverge {
                key: k.clone(),
                expected: id.raw(),
                got: inserted_id,
                kind: PartialRegistry::kind(),
            }
            .into_err());
        }
    }

    Ok(())
}
