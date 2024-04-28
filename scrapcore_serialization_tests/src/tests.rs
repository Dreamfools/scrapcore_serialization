use crate::{load_database, CityItemSerialized};
use std::fs;

#[test]
fn save_schema() {
    let schema = schemars::schema_for!(CityItemSerialized);
    let data = serde_json::to_vec(&schema).unwrap();
    fs::write("./schema.json", data).unwrap()
}

#[test]
fn load_test_db() {
    if let Err(err) = load_database("./test_db".as_ref()) {
        panic!("{}", err)
    }
}
