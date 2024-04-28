pub mod registry;

pub mod serialization;

#[cfg(feature = "derive")]
pub mod derive {
    pub use scrapcore_serialization_macro::*;
}

pub type ItemId = String;
pub type ItemIdRef<'a> = &'a str;
pub type AssetName = String;
pub type AssetNameRef<'a> = &'a str;
