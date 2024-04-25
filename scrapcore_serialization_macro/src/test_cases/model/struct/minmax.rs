#[derive(Debug, DatabaseModel)]
struct Test {
    #[model(min = 5)]
    with_min: u32,
    #[model(max = 15)]
    with_max: u32,
    #[model(min = -5, max = 15)]
    with_min_max: u32,
}
