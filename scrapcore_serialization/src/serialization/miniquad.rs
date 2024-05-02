use crate::registry::{AssetsHolder, PartialRegistry};
use crate::serialization::error::{DeserializationError, DeserializationErrorKind};
use crate::serialization::{DeserializeModel, SerializationFallback};
use crate::{AssetName, AssetNameRef};
use miniquad::TextureId;

/// Deserialization for bevy asset handler fields
///
/// Fields are populated with WEAK handles to the asset
/// Currently there is no way to request a strong handle
impl<'a, Registry: PartialRegistry> DeserializeModel<TextureId, Registry> for AssetNameRef<'a>
where
    Registry: AssetsHolder<TextureId>,
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<TextureId, DeserializationError<Registry>> {
        let name = self.to_ascii_lowercase();
        if let Some(handle) = registry.get_assets().get(&name) {
            Ok(handle.0)
        } else {
            Err(DeserializationErrorKind::MissingAsset(name, Registry::asset_kind()).into())
        }
    }
}

impl SerializationFallback for TextureId {
    type Fallback = AssetName;
}
