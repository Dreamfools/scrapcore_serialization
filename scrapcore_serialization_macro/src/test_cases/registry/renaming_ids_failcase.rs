#[registry(error = ModelError)]
pub struct Model {
    #[model(singleton, id_name = ATestId)]
    test: A,
}