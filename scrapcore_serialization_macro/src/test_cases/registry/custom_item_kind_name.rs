#[registry(error = "ModelError", item_kind_name = CustomKind)]
pub enum Model {
    #[model(collection)]
    Test(A),
    #[model(singleton)]
    TestSingle(B),
    #[model(asset)]
    TestAsset(Option<A>),
}
