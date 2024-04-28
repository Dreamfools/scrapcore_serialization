use scrapcore_serialization::derive::{registry, DatabaseModel};
use scrapcore_serialization::serialization::error::{
    DeserializationError, DeserializationErrorKind,
};
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[cfg(test)]
mod tests;

#[derive(Debug, DatabaseModel)]
struct Person {
    name: String,
    #[model(id)]
    mom: Option<PersonId>,
    #[model(id)]
    dad: Option<PersonId>,
}

#[derive(Debug, DatabaseModel)]
struct House {
    residents: Vec<Person>,
}

#[derive(Debug, DatabaseModel)]
struct Theater {
    name: String,
    seats: u32,
}

#[derive(Debug, DatabaseModel)]
enum Plot {
    Empty,
    House(House),
    Theater(Theater),
}

#[registry(error = "ModelError")]
enum City {
    #[model(collection)]
    Person(Person),
}

#[derive(Debug, Clone, Error)]
enum ModelError {}

fn load_database(path: &Path) -> Result<CityRegistry, DeserializationError<PartialCityRegistry>> {
    let mut registry = PartialCityRegistry::default();
    for entry in WalkDir::new(path).into_iter() {
        let entry = entry.unwrap();
        if !entry.path().is_file() {
            continue;
        }

        let data = std::fs::read(entry.path()).unwrap();

        let data: CityItemSerialized = serde_json::from_slice(&data).map_err(|err| {
            DeserializationErrorKind::<PartialCityRegistry>::ParsingError(err.to_string())
                .into_err()
        })?;
        registry.insert(entry.path(), data)?;
    }

    registry.into_registry()
}
