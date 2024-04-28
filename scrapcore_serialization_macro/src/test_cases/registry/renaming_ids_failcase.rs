#[registry(error = "ModelError")]
pub enum Model {
    #[model(singleton, id_name = ATestId)]
    test(A),
}
