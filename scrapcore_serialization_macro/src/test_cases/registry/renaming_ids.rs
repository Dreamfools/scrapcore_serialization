#[registry(error = "ModelError")]
pub enum Model {
    #[model(collection, id_name = ATestId)]
    Test(A),
    #[model(collection, id_name = SomethingElse)]
    TestSingle(B),
    #[model(asset)]
    TestAsset(Option<A>),
}
