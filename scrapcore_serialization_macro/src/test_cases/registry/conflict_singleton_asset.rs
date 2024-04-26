#[registry(error = ModelError)]
pub struct Model {
    #[model(singleton, asset)]
    test: A,
}