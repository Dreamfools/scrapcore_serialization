#[registry(error = "ModelError", no_schema)]
pub enum Model {
    #[model(collection)]
    Test1(A),
    #[model(singleton)]
    Test2(A),
}
