#[derive(Debug, DatabaseModel)]
struct Test {
    a: u32,
    b: String,
    c: Vec<Test>,
}
