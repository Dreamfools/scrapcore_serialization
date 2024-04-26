#[registry(error = ModelError, vis = )]
pub struct Model {
    #[model(collection)]
    test: A,
    #[model(singleton)]
    test_single: B,
    #[model(asset)]
    test_asset: Option<A>,
}