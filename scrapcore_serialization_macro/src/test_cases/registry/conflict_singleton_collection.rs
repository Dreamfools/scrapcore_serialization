#[registry(error = ModelError)]
pub struct Model {
    #[model(singleton, collection)]
    test: A,
}