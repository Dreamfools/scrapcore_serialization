use crate::load_database;

#[test]
fn load_test_db() {
    load_database("./test_db".as_ref()).unwrap();
}
