#[registry(error = ModelError)]
pub struct Model {
    #[model(asset, collection)]
    test: A,
}