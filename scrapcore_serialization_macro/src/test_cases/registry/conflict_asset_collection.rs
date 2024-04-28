#[registry(error = "ModelError")]
pub enum Model {
    #[model(asset, collection)]
    Test(A),
}
