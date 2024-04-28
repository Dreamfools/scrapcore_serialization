#[registry(error = "ModelError", serialized_item_name = CustomSerializedItemName)]
pub enum Model {
    #[model(collection)]
    Test(A),
    #[model(singleton)]
    TestSingle(B),
    #[model(asset)]
    TestAsset(Option<A>),
}
