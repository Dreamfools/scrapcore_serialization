#[registry(error = "ModelError")]
pub enum Model {
    #[model(singleton, asset)]
    Test(A),
}
