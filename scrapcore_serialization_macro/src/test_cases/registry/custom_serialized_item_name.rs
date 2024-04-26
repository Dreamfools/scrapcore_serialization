#[registry(error = ModelError, serialized_item_name = CustomSerializedItemName)]
pub struct Model {
    #[model(collection)]
    test: A,
    #[model(singleton)]
    test_single: B,
    #[model(asset)]
    test_asset: Option<A>,
}