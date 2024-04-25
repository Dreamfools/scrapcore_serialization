#[derive(Debug, DatabaseModel)]
enum Test {
    #[model(raw)]
    A(u32),
    #[model(with = custom_fn)]
    B(String),
    #[model(ty = "FxHashMap<ItemId, f64>")]
    C(IntMap<VariableId, f64>),
    #[model(ty = "f32", with = "stringify_fn")]
    D(String),
    #[model(from = "u32")]
    E(u64),
    #[model(raw, from = "u32")]
    E(u64),
}