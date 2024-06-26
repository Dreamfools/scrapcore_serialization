use crate::registry::kind::ItemKindProvider;
use crate::registry::path_identifier::PathIdentifier;
use crate::registry::SerializationRegistry;
use crate::serialization::error::internal::InternalDeserializationError;
use crate::{AssetName, ItemId};
use slabmap::SlabMapDuplicateError;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[cfg(feature = "miette")]
mod diagnostic;

pub mod internal;

#[derive(Debug, Error, Clone)]
pub enum DeserializationErrorKind<Registry: SerializationRegistry> {
    /// Error at data loading stage
    #[error("Data loading error: {}", .0)]
    LoadingError(String),
    #[error("Item {}({}) is missing", .1, .0)]
    MissingItem(ItemId, Registry::ItemKind),
    #[error("Item {}({}) is declared twice, in `{}` and `{}`", .kind, .id, .path_a, .path_b)]
    DuplicateItem {
        id: ItemId,
        kind: Registry::ItemKind,
        path_a: PathIdentifier,
        path_b: PathIdentifier,
    },
    #[error("Item {}({}) is already declared", .1, .0)]
    DuplicateItemLowInfo(ItemId, Registry::ItemKind),
    #[error("Image `{}` is missing", .0)]
    MissingAsset(AssetName, Registry::AssetKind),
    #[error("Asset name `{}` is contested by `{}` and `{}`", .name, .path_a, .path_b)]
    DuplicateAsset {
        kind: Registry::AssetKind,
        name: AssetName,
        path_a: PathIdentifier,
        path_b: PathIdentifier,
    },
    #[error("Singleton item {} is declared twice, in `{}` and `{}`", .kind, .path_a, .path_b)]
    DuplicateSingleton {
        kind: Registry::ItemKind,
        path_a: PathIdentifier,
        path_b: PathIdentifier,
    },
    #[error("Singleton item {} is missing", .kind)]
    MissingSingleton { kind: Registry::ItemKind },
    #[error("File at `{}` doesn't have a name", .0)]
    MissingName(PathIdentifier),
    #[error("File path at `{}` is not UTF8", .0)]
    NonUtf8Path(PathIdentifier),
    #[error("Value is too large, got {} where at most {} is expected.", .got, .limit)]
    ValueTooLarge { limit: f64, got: f64 },
    #[error("Value is too small, got {} where at least {} is expected.", .got, .limit)]
    ValueTooSmall { limit: f64, got: f64 },
    #[error("Internal error. Please report this to the application author: {}", .0)]
    InternalError(InternalDeserializationError<Registry>),
    #[error("{}", .0)]
    Custom(Registry::Error),
}

impl<Registry: SerializationRegistry> DeserializationErrorKind<Registry> {
    pub fn into_err(self) -> DeserializationError<Registry> {
        self.into()
    }
}

#[derive(Debug, Clone)]
pub enum DeserializationErrorStackItem<Registry: SerializationRegistry> {
    File(PathIdentifier),
    ItemByPath(PathIdentifier, Registry::ItemKind),
    ItemById(ItemId, Registry::ItemKind),
    Field(&'static str),
    Variant(&'static str),
    Index(usize),
    MapKey(String),
    MapEntry(String),
    // all JSON keys are strings, so we expect deserialized value to be reasonably displayable
    ExprVariable(String),
}

impl<Registry: SerializationRegistry> Display for DeserializationErrorStackItem<Registry> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeserializationErrorStackItem::File(path) => {
                write!(f, "In file at `{}`", path)
            }
            DeserializationErrorStackItem::ItemByPath(path, kind) => {
                write!(f, "In item <{kind}> at `{}`", path)
            }
            DeserializationErrorStackItem::ItemById(id, kind) => {
                write!(f, "In item <{kind}>`{id}`")
            }
            DeserializationErrorStackItem::Field(name) => write!(f, "In field {name}"),
            DeserializationErrorStackItem::Variant(name) => write!(f, "In variant {name}"),
            DeserializationErrorStackItem::Index(i) => write!(f, "In item at position {i}"),
            DeserializationErrorStackItem::MapEntry(name) => {
                write!(f, "In map entry with key `{name}`")
            }
            DeserializationErrorStackItem::MapKey(name) => {
                write!(f, "In map key `{name}`")
            }
            DeserializationErrorStackItem::ExprVariable(name) => {
                write!(f, "In expression variable `{name}`")
            }
        }
    }
}

#[derive(Debug, Error, Clone)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub struct DeserializationError<Registry: SerializationRegistry> {
    pub kind: DeserializationErrorKind<Registry>,
    pub stack: Vec<DeserializationErrorStackItem<Registry>>,
}

impl<Registry: SerializationRegistry> DeserializationError<Registry> {
    /// Checks if this error is just a hot reload blocker, and can be solved by
    /// making a full reload, without any extra actions form the user
    pub fn is_hot_reload_blocker(&self) -> bool {
        matches!(
            self.kind,
            DeserializationErrorKind::InternalError(
                InternalDeserializationError::UnfilledHotReloadingSlot(..)
            )
        )
    }
}

impl<Registry: SerializationRegistry> Display for DeserializationError<Registry> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        for item in &self.stack {
            write!(f, "\n{}", item)?;
        }
        Ok(())
    }
}

impl<Registry: SerializationRegistry> DeserializationError<Registry> {
    pub fn context(mut self, item: DeserializationErrorStackItem<Registry>) -> Self {
        self.stack.push(item);
        self
    }
}

impl<Registry: SerializationRegistry> From<DeserializationErrorKind<Registry>>
    for DeserializationError<Registry>
{
    fn from(value: DeserializationErrorKind<Registry>) -> Self {
        DeserializationError {
            kind: value,
            stack: Default::default(),
        }
    }
}

impl<Registry: SerializationRegistry> From<Infallible> for DeserializationError<Registry> {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

// impl From<ExError> for DeserializationError {
//     fn from(value: ExError) -> Self {
//         DeserializationErrorKind::BadExpression(value).into()
//     }
// }

impl<T, Registry: SerializationRegistry> From<SlabMapDuplicateError<ItemId, T>>
    for DeserializationError<Registry>
where
    Registry: ItemKindProvider<T>,
{
    fn from(SlabMapDuplicateError(id, _): SlabMapDuplicateError<ItemId, T>) -> Self {
        DeserializationErrorKind::DuplicateItemLowInfo(id, Registry::kind()).into()
    }
}

/// Helper for wrapping a code block to help with contextualizing errors
/// Better editor support
#[inline(always)]
pub fn s_try<Args, T, Registry: SerializationRegistry>(
    args: Args,
    func: impl FnOnce(Args) -> Result<T, DeserializationError<Registry>>,
    context: Option<impl FnOnce() -> DeserializationErrorStackItem<Registry>>,
) -> Result<T, DeserializationError<Registry>> {
    let result = func(args);
    if let Some(context) = context {
        result.map_err(|e| e.context(context()))
    } else {
        result
    }
}
