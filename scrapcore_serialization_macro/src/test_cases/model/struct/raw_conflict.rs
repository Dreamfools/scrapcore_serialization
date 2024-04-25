#[derive(Debug, DatabaseModel)]
struct Test {
    #[model(ty = "f64", raw)]
    a: u32
}
