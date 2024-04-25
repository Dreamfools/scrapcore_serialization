#[derive(Debug, DatabaseModel)]
enum Test {
    #[model(raw, min = 5)]
    WithMin(u32),
    #[model(raw, max = 15)]
    WithMax(u32),
    #[model(raw, min = -5, max = 15)]
    WithMinMax(u32),
}
