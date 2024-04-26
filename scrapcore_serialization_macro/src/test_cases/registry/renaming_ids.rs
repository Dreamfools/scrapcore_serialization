#[registry(error = ModelError)]
pub struct Model {
    #[model(collection, id_name = ATestId)]
    test: A,
    #[model(collection, id_name = SomethingElse)]
    test2: B,
    #[model(collection)]
    test3: C1,
}