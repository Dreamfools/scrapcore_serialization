#[registry(error = ModelError, item_name = CustomItem, serialized_item_name = CustomSerializedItemName, item_kind_name = CustomKind, assets_kind_name = CustomAsset, registry_name = CustomRegistry, partial_registry_name = CustomPartialRegistry)]
pub struct Model {
    #[model(collection)]
    test: A,
    #[model(singleton)]
    test_single: B,
    #[model(asset)]
    test_asset: Option<A>,
}