use scrapcore_serialization::derive::{registry, DatabaseModel};
use scrapcore_serialization::serialization::error::{
    DeserializationError, DeserializationErrorKind,
};
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[cfg(test)]
mod tests;

use id::PersonId;

#[derive(Debug, DatabaseModel)]
pub struct Person {
    pub name: String,
    #[model(id)]
    pub mom: Option<PersonId>,
    #[model(id)]
    pub dad: Option<PersonId>,
}

#[derive(Debug, DatabaseModel)]
pub struct House {
    pub residents: Vec<Person>,
}

#[derive(Debug, DatabaseModel)]
pub struct Theater {
    pub name: String,
    pub seats: u32,
}

#[derive(Debug, DatabaseModel)]
pub enum Plot {
    Empty,
    House(House),
    Theater(Theater),
}

#[derive(Debug, DatabaseModel)]
pub struct Mayor {
    pub person: PersonId,
}

#[registry(error = "ModelError")]
pub enum City {
    #[model(collection)]
    Person(Person),
    #[model(collection)]
    Plot(Plot),
    #[model(singleton)]
    Mayor(Mayor),
}

#[derive(Debug, Clone, Error)]
pub enum ModelError {}

fn load_database(path: &Path) -> Result<CityRegistry, DeserializationError<PartialCityRegistry>> {
    let mut registry = PartialCityRegistry::default();
    for entry in WalkDir::new(path).into_iter() {
        let entry = entry.unwrap();
        if !entry.path().is_file() {
            continue;
        }

        let data = std::fs::read(entry.path()).unwrap();

        let data: CityItemSerialized = serde_json::from_slice(&data).map_err(|err| {
            DeserializationErrorKind::<PartialCityRegistry>::LoadingError(err.to_string())
                .into_err()
        })?;
        registry.insert(entry.path(), data)?;
    }

    registry.into_registry()
}
