#[registry(error = "ModelError", partial_registry_name = CustomPartialRegistry)]
pub enum Model {
    #[model(collection)]
    Test(A),
    #[model(singleton)]
    TestSingle(B),
    #[model(asset)]
    TestAsset(Option<A>),
}
