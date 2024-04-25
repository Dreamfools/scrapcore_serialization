use ahash::AHashSet;
use attribute_derive::Attribute;
use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::format_ident;
use syn::spanned::Spanned;
use syn::{ItemStruct, Type, Visibility};

use crate::error::{bail, MacroError};
use crate::registry::{AssetKind, ModelKind, RegistryDefinitions};
use crate::serialized_of;

#[derive(Debug, Attribute)]
struct RegistryAttributeInput {
    /// Whether to emit schemars derives for the model
    #[attribute(default = true)]
    schema: bool,
    /// Overrides the name of the registry items. Defaults to the struct name
    /// with "Item" appended
    item_name: Option<Ident>,
    /// Overrides the name of the serialized model. Defaults to the [item_name]
    /// with "Serialized" appended
    serialized_item_name: Option<Ident>,
    /// Overrides the name of the item kind. Defaults to the [item_name] with
    /// "Kind" appended
    item_kind_name: Option<Ident>,
    /// Overrides the name of the assets kind. Defaults to the struct name with
    /// "AssetKind" appended
    assets_kind_name: Option<Ident>,
    /// Overrides the name of the registry. Defaults to the struct name with
    /// "Registry" appended
    registry_name: Option<Ident>,
    /// Overrides the name of the partial registry. Defaults to [registry_name]
    /// with "Partial" prepended
    partial_registry_name: Option<Ident>,
    /// Visibility for all of generated items
    vis: Option<Visibility>,
    /// Custom registry error type
    error: Type,
}

#[derive(Debug, Attribute)]
#[attribute(ident = model)]
struct ModelAttributeInput {
    #[attribute(conflicts=[collection, singleton])]
    asset: bool,
    #[attribute(conflicts=[asset, singleton])]
    collection: bool,
    #[attribute(conflicts=[asset, collection])]
    singleton: bool,
}
pub(super) fn parse_struct_defs(
    attr: proc_macro::TokenStream,
    data: &mut ItemStruct,
) -> Result<RegistryDefinitions, MacroError> {
    let mut used_types = AHashSet::default();
    let input = RegistryAttributeInput::from_args(attr.into())?;
    let registry_item_name = input
        .item_name
        .unwrap_or_else(|| format_ident!("{}Item", data.ident));
    let registry_name = input
        .registry_name
        .unwrap_or_else(|| format_ident!("{}Registry", data.ident));
    let partial_registry_name = input
        .partial_registry_name
        .unwrap_or_else(|| format_ident!("Partial{}", registry_name));
    let serialized_model_name = input
        .serialized_item_name
        .unwrap_or_else(|| format_ident!("{}Serialized", registry_item_name));
    let kind_name = input
        .item_kind_name
        .unwrap_or_else(|| format_ident!("{}Kind", registry_item_name));
    let assets_kind_name = input
        .assets_kind_name
        .unwrap_or_else(|| format_ident!("{}AssetKind", data.ident));

    let visibility = input.vis.unwrap_or_else(|| data.vis.clone());

    let mut registry = RegistryDefinitions {
        serialized_model_name,
        registry_name,
        partial_registry_name,
        kind_name,
        assets_kind_name,
        model_name: registry_item_name,
        error: input.error,
        schema: input.schema,
        singletons: Default::default(),
        collections: Default::default(),
        assets: Default::default(),
        visibility,
    };

    for field in &mut data.fields {
        if !used_types.insert(&field.ty) {
            bail!(field.ty.span(), "This type is already defined in the model")
        }
        let attribute = ModelAttributeInput::remove_attributes(&mut field.attrs)?;
        let name = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new(field.span(), "Tuple enums are not supported"))?;

        if attribute.asset {
            registry.assets.push(AssetKind {
                span: field.span(),
                variant_name: Ident::new(
                    &name
                        .to_string()
                        .from_case(Case::Snake)
                        .to_case(Case::Pascal),
                    field.span(),
                ),
                field_name: name,
                ty: field.ty.clone(),
            });
        } else {
            let model = ModelKind {
                span: field.span(),
                variant_name: Ident::new(
                    &name
                        .to_string()
                        .from_case(Case::Snake)
                        .to_case(Case::Pascal),
                    field.span(),
                ),
                field_name: name,
                ty: field.ty.clone(),
                ty_serialized: serialized_of(&field.ty)?,
            };
            if attribute.collection {
                registry.collections.0.push(model);
            } else if attribute.singleton {
                registry.singletons.0.push(model);
            } else {
                bail!(field.span(), "All fields must be annotated with #[model(asset)], #[model(collection)], or #[model(singleton)]")
            }
        }
    }
    Ok(registry)
}
