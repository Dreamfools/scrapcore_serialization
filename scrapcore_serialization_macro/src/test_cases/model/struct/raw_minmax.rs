#[derive(Debug, DatabaseModel)]
struct Test {
    #[model(raw, min = 5)]
    with_min: u32,
    #[model(raw, max = 15)]
    with_max: u32,
    #[model(raw, min = -5, max = 15)]
    with_min_max: u32,
}
