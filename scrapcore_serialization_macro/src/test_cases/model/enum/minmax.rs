#[derive(Debug, DatabaseModel)]
enum Test {
    #[model(min = 5)]
    WithMin(u32),
    #[model(max = 15)]
    WithMax(u32),
    #[model(min = -5, max = 15)]
    WithMinMax(u32),
}
