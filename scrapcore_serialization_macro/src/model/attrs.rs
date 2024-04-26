use darling::util::Flag;
use darling::FromMeta;
use proc_macro2::Ident;
use syn::{Expr, Path, Type};

/// Attribute fields only usable on the struct/enum-level #\[model] attribute
#[derive(Debug, FromMeta)]
pub struct ModelAttributeConfig {
    /// Custom name for the serialized type
    pub name: Option<Ident>,
    /// Skips deriving `schemars` schema
    pub no_schema: Flag,
    /// Extra "where" conditions for the `DeserializeModel` implementation
    #[darling(multiple)]
    #[darling(rename = "condition")]
    pub extra_conditions: Vec<syn::WhereClause>,
}

/// Attribute fields for both struct and enum fields
#[derive(Debug, FromMeta)]
pub struct SharedAttributeConfig {
    /// Custom serialized field type
    ///
    /// This flag is **conflicting** with `raw` flag
    #[darling(rename = "ty")]
    pub custom_ty: Option<Type>,
    /// Path to the custom deserialization function for the field
    ///
    /// Deserialization function must have signature
    /// `fn(S, Registry) -> Result<T, impl DeserializationError<Registry>>`,
    /// where `S` is a serialized field type (or type specified in `ty`
    /// attribute), and `T` is the resulting field type (or type specified in
    /// `from` attribute)
    ///
    /// This flag is **conflicting** with `raw` flag
    pub with: Option<Path>,
    /// Completely skips deserialization for the given field
    ///
    /// This flag is identical to using`#[model(ty=T, with=identity_function)]`,
    /// where `T` is the field type, and so this flag is **conflicting** with
    /// `ty` and `with` flags
    ///
    /// Useful for types from external crates that do not implement `DeserializeModel`
    pub raw: Flag,

    /// Forces the field to be deserialized as a given type,
    /// and then converted into the field type using the `From` impl
    pub from: Option<Type>,

    /// Applies min validator to the field
    pub min: Option<Expr>,
    /// Applies max validator to the field
    pub max: Option<Expr>,

    /// Marks the field as "Id" field, emitting a different `where` condition.
    ///
    /// Often useful for avoiding recursive trait bounds.
    ///
    /// Relies on `ReverseId` trait:
    /// `where Registry: PartialCollectionHolder<<T as ReverseId>::Item>`
    /// where `T` is the field type
    pub id: Flag,

    /// Same as [id], but uses the provided type instead of relying on a trait
    ///
    /// Often useful for avoiding recursive trait bounds.
    ///
    /// Example generated condition:
    /// `where Registry: PartialCollectionHolder<T>` where `T` is the provided type
    pub id_of: Option<Type>,

    /// Skips generating `where` condition for this field
    ///
    /// Useful for avoiding recursion issues
    pub no_condition: Flag,
}

/// Attribute fields for struct fields
#[derive(Debug, FromMeta)]
pub struct StructFieldAttributeConfig {
    /// Generated AsRef implementation for marked struct to value of this field
    pub as_ref: Flag,
    /// Custom name for the serialized field
    pub rename: Option<Ident>,

    /// Shared config, see [SharedAttributeConfig] for available attributes
    #[darling(flatten)]
    pub config: SharedAttributeConfig,
}

/// Attribute fields for enum fields
#[derive(Debug, FromMeta)]
pub struct EnumVariantAttributeConfig {
    /// Custom name for the serialized variant
    pub rename: Option<Ident>,
    /// Shared config, see [SharedAttributeConfig] for available attributes
    #[darling(flatten)]
    pub config: SharedAttributeConfig,
}
