#[registry(error = "ModelError", item_name = CustomItem, serialized_item_name = CustomSerializedItemName, item_kind_name = CustomKind, assets_kind_name = CustomAsset, registry_name = CustomRegistry, partial_registry_name = CustomPartialRegistry)]
pub enum Model {
    #[model(collection)]
    Test(A),
    #[model(singleton)]
    TestSingle(B),
    #[model(asset)]
    TestAsset(Option<A>),
}
