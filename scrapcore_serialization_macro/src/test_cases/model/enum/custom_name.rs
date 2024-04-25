#[derive(Debug, DatabaseModel)]
#[model(name = CustomName)]
enum Test {
    A(u32),
    B(String),
    C(Vec<Test>),
}
