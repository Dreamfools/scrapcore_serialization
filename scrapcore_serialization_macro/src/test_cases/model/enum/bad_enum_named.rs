#[derive(Debug, DatabaseModel)]
enum Test {
    #[model(with = "convert", raw)]
    A {
        name: u32
    }
}
