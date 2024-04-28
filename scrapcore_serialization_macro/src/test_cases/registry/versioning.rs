#[registry(error = "ModelError")]
pub enum Model {
    #[model(singleton)]
    TestVersioning {
        #[model(version = 4)]
        some_name: B,
        #[model(version_name = "D")]
        v1: D,
        v2: C,
        #[model(version = 3, version_name = "Test2")]
        random: E,
    },
}
