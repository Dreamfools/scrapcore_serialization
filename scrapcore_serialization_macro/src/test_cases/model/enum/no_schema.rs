#[derive(Debug, DatabaseModel)]
#[model(no_schema)]
enum Test {
    A(u32),
    B(String),
    C(Vec<Test>),
}
