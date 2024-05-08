use convert_case::{Case, Casing};
use darling::ast::{Data, Fields, NestedMeta};
use darling::util::{Flag, SpannedValue};
use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use syn::spanned::Spanned;
use syn::{DeriveInput, Generics, Type, Visibility};

use crate::error::{bail, MacroError};
use crate::registry::{AssetKind, ModelKind, RegistryDefinitions};
use crate::serialized_of;

#[derive(Debug, FromMeta)]
struct RegistryAttributeInput {
    /// Whether to skip emitting `schemars` derives for the model
    no_schema: Flag,
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
    #[darling(rename = "vis")]
    visibility: Option<Visibility>,
    /// Custom registry error type
    error: Type,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(model))]
struct ModelAttributeInput {
    asset: Flag,
    collection: Flag,
    singleton: Flag,

    /// Name of the ID type for this model (only applies to collections)
    id_name: Option<Ident>,

    // <========================>
    // <=== Technical Fields ===>
    // <========================>
    ident: Ident,
    fields: Fields<SpannedValue<ModelEnumFieldInput>>,
}

#[derive(Debug, FromField)]
#[darling(attributes(model))]
struct ModelEnumFieldInput {
    /// Version of the data entry. The default version is `1`
    ///
    /// Versioning on only supposed to be used for breaking format changes, so
    /// only the "major" version can be specified here
    ///
    /// Please open a github issue if you believe you have a valid use case
    /// for more gradual versioning
    version: Option<usize>,

    /// Custom name of the version, used for serialization. Defaults to string
    /// representation of `version` field
    version_name: Option<String>,

    // <========================>
    // <=== Technical Fields ===>
    // <========================>
    ident: Option<Ident>,
    ty: Type,
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(enum_newtype))]
struct RegistryAttributeDeriveInput {
    // <========================>
    // <=== Technical Fields ===>
    // <========================>
    ident: Ident,
    vis: Visibility,
    generics: Generics,

    data: Data<SpannedValue<ModelAttributeInput>, darling::util::Ignored>,
}

pub(super) fn parse_struct_defs(
    attr: TokenStream,
    data: &mut DeriveInput,
) -> Result<RegistryDefinitions, MacroError> {
    let input = RegistryAttributeInput::from_list(&NestedMeta::parse_meta_list(attr)?)?;
    let body = RegistryAttributeDeriveInput::from_derive_input(data)?;

    if !body.generics.params.is_empty() {
        bail!(body.generics.span(), "Generic registries are not supported")
    }

    strip_attrs(data);

    let registry_item_name = input
        .item_name
        .unwrap_or_else(|| format_ident!("{}Item", body.ident));
    let registry_name = input
        .registry_name
        .unwrap_or_else(|| format_ident!("{}Registry", body.ident));
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
        .unwrap_or_else(|| format_ident!("{}AssetKind", body.ident));

    let visibility = input.visibility.unwrap_or_else(|| body.vis.clone());

    let mut registry = RegistryDefinitions {
        serialized_model_name,
        registry_name,
        partial_registry_name,
        kind_name,
        assets_kind_name,
        model_name: registry_item_name,
        error: input.error,
        schema: !input.no_schema.is_present(),
        singletons: Default::default(),
        collections: Default::default(),
        assets: Default::default(),
        visibility,
    };

    let mut used_types = vec![];
    for variant in body.data.take_enum().expect("") {
        let name = variant.ident.clone();
        if [&variant.asset, &variant.collection, &variant.singleton]
            .iter()
            .filter_map(|f| f.is_present().then_some(()))
            .count()
            > 1
        {
            bail!(
                variant.span(),
                "Only one of `asset`, `collection`, or `singleton` markers can be present at once"
            )
        }

        if let Some(name) = &variant.id_name {
            if !variant.collection.is_present() {
                bail!(
                    name.span(),
                    "`id_name` attribute can only be used with `collection` marker"
                )
            }
        }

        if variant.asset.is_present() {
            if !variant.fields.is_newtype() {
                bail!(
                    variant.span(),
                    "Versioning is not supported for asset variants"
                );
            }
            registry.assets.push(AssetKind {
                span: variant.span(),
                field_name: Ident::new(
                    &name
                        .to_string()
                        .from_case(Case::Pascal)
                        .to_case(Case::Snake),
                    variant.ident.span(),
                ),
                variant_name: name,
                ty: variant.fields.fields[0].ty.clone(),
            });
        } else {
            let variant_name = name;
            let field_name = Ident::new(
                &variant_name
                    .to_string()
                    .from_case(Case::Pascal)
                    .to_case(Case::Snake),
                variant.ident.span(),
            );

            let [field] = variant.fields.fields.as_slice() else {
                unreachable!("Should only have newtype enums")
            };

            let ty = field.ty.clone();
            let ty_serialized = serialized_of(&field.ty);
            if used_types.contains(&ty) {
                bail!(ty.span(), "This type is already defined in the model")
            }
            used_types.push(ty.clone());
            let model = ModelKind {
                span: variant.span(),
                id_name: variant
                    .id_name
                    .clone()
                    .unwrap_or_else(|| format_ident!("{}Id", variant_name)),
                variant_name,
                field_name,
                ty,
                ty_versioned: ty_serialized,
            };
            if variant.collection.is_present() {
                registry.collections.0.push(model);
            } else if variant.singleton.is_present() {
                registry.singletons.0.push(model);
            } else {
                bail!(variant.span(), "All fields must be annotated with #[model(asset)], #[model(collection)], or #[model(singleton)]")
            }
        }
    }

    Ok(registry)
}

fn strip_attrs(input: &mut DeriveInput) {
    input.attrs.retain(|attr| !attr.path().is_ident("registry"));
    if let syn::Data::Enum(data) = &mut input.data {
        for variant in &mut data.variants {
            variant.attrs.retain(|attr| !attr.path().is_ident("model"));
            for field in &mut variant.fields {
                field.attrs.retain(|attr| !attr.path().is_ident("model"));
            }
        }
    }
}
