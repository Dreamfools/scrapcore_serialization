#[derive(Debug, DatabaseModel)]
#[model(name = CustomName)]
struct Test {
    a: u32,
    b: String,
    c: Vec<Test>,
}
