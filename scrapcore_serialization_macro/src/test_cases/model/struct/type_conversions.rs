#[derive(Debug, DatabaseModel)]
struct Test {
    #[model(raw)]
    a: u32,
    #[model(with = custom_fn)]
    b: String,
    #[model(ty = "FxHashMap<ItemId, f64>")]
    c: IntMap<VariableId, f64>,
    #[model(ty = "f32", with = "stringify_fn")]
    d: String,
}
