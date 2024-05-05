use crate::registry::{AssetsHolder, PartialRegistry};
use crate::serialization::error::{DeserializationError, DeserializationErrorKind};
use crate::serialization::{DeserializeModel, SerializationFallback};
use crate::{AssetName, AssetNameRef};
use assets_manager::Handle;

/// Deserialization for `assets_manager` handler fields
///
/// Only static handles are supported
impl<'a, Registry: PartialRegistry, A> DeserializeModel<&'static Handle<A>, Registry>
    for AssetNameRef<'a>
where
    Registry: AssetsHolder<&'static Handle<A>>,
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<&'static Handle<A>, DeserializationError<Registry>> {
        let name = self.to_ascii_lowercase();
        if let Some(handle) = registry.get_assets().get(&name) {
            Ok(handle.0)
        } else {
            Err(DeserializationErrorKind::MissingAsset(name, Registry::asset_kind()).into())
        }
    }
}

impl<A> SerializationFallback for &'static Handle<A> {
    type Fallback = AssetName;
}
