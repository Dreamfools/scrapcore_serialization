#[registry(error = "ModelError", vis = "")]
pub enum Model {
    #[model(collection)]
    Test(A),
    #[model(singleton)]
    TestSingle(B),
    #[model(asset)]
    TestAsset(Option<A>),
}
