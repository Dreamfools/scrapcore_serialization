#[registry(error = ModelError, no_schema)]
pub struct Model {
    #[model(collection)]
    test1: A,
    #[model(singleton)]
    test2: A,
}