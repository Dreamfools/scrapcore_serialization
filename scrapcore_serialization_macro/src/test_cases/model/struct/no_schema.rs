#[derive(Debug, DatabaseModel)]
#[model(no_schema)]
struct Test {
    a: u32,
    b: String,
    c: Vec<Test>,
}
