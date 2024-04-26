#[registry(error = ModelError)]
pub struct Model {
    #[model(singleton, collection, asset)]
    test: A,
}