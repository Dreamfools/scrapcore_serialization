#[derive(Debug, DatabaseModel)]
enum Test {
    A(u32),
    B(String),
    C(Vec<Test>),
}
