use crate::registry::{AssetsHolder, PartialRegistry};
use crate::serialization::error::{DeserializationError, DeserializationErrorKind};
use crate::serialization::{DeserializeModel, SerializationFallback};
use crate::{AssetName, AssetNameRef};
use macroquad::texture::Texture2D;

/// Deserialization for Macroquad Texture2D
///
/// Fields are populated with WEAK handles to the asset
/// Currently there is no way to request a strong handle
impl<'a, Registry: PartialRegistry> DeserializeModel<Texture2D, Registry> for AssetNameRef<'a>
where
    Registry: AssetsHolder<Texture2D>,
{
    fn deserialize(
        self,
        registry: &mut Registry,
    ) -> Result<Texture2D, DeserializationError<Registry>> {
        let name = self.to_ascii_lowercase();
        if let Some(handle) = registry.get_assets().get(&name) {
            Ok(handle.0.weak_clone())
        } else {
            Err(DeserializationErrorKind::MissingAsset(name, Registry::asset_kind()).into())
        }
    }
}

impl SerializationFallback for Texture2D {
    type Fallback = AssetName;
}
