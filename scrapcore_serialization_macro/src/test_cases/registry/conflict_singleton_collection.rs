#[registry(error = "ModelError")]
pub enum Model {
    #[model(singleton, collection)]
    Test(A),
}
