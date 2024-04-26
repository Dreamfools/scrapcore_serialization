#[registry(error = ModelError, item_name = CustomItem)]
pub struct Model {
    #[model(collection)]
    test: A,
    #[model(singleton)]
    test_single: B,
    #[model(asset)]
    test_asset: Option<A>,
}