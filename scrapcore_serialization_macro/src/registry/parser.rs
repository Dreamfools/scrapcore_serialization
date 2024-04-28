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
use crate::registry::{AssetKind, ModelKind, RegistryDefinitions, Version};
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
#[darling(attributes(registry))]
#[darling(supports(enum_newtype, enum_tuple))]
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
    strip_attrs(data);
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

    for variant in body.data.take_enum().expect("") {
        let name2 = variant.ident.clone();
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
                    "Versioning is not supported fort asset variants"
                );
            }
            registry.assets.push(AssetKind {
                span: variant.span(),
                field_name: Ident::new(
                    &name2
                        .to_string()
                        .from_case(Case::Snake)
                        .to_case(Case::Pascal),
                    variant.ident.span(),
                ),
                variant_name: name2,
                ty: variant.fields.fields[0].ty.clone(),
            });
        } else {
            let variant_name = name2;
            let field_name = Ident::new(
                &variant_name
                    .to_string()
                    .from_case(Case::Snake)
                    .to_case(Case::Pascal),
                variant.ident.span(),
            );
            let (ty, ty_serialized, versions) = history(&variant_name, &variant)?;
            let model = ModelKind {
                span: variant.span(),
                id_name: variant
                    .id_name
                    .clone()
                    .unwrap_or_else(|| format_ident!("{}Id", variant_name)),
                variant_name,
                field_name,
                ty,
                ty_serialized2: ty_serialized,
                versioning: versions,
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

/// returns (type, serialized_type, history)
fn history(
    variant_name: &Ident,
    variant: &SpannedValue<ModelAttributeInput>,
) -> Result<(Type, Type, Vec<Version>), MacroError> {
    // if variant.fields.is_newtype() {
    //     let field = &variant.fields.fields[0];
    //     let ty = field.ty.clone();
    //     let ty_serialized = serialized_of(&field.ty);
    //     let version = version_data(field);
    //
    //     Ok((ty, ty_serialized, vec![version]))
    // } else {
    let versions = variant
        .fields
        .fields
        .iter()
        .map(version_data)
        .sorted_unstable_by(|a, b| a.version.cmp(&b.version).reverse())
        .collect_vec();
    let mut version_names = vec![&versions[0].version_name];
    for x in versions.windows(2) {
        let [first, second] = x else {
            unreachable!("Window size is 2")
        };
        if first.version == second.version {
            bail!(
                second.ty.span(),
                "Variant with this version is already declared"
            );
        } else if version_names.contains(&&second.version_name) {
            bail!(
                second.ty.span(),
                "Variant with this version name is already declared"
            );
        }
        version_names.push(&second.version_name)
    }
    let final_type = versions[0].ty.clone();

    let serialized_type: Type = syn::parse_str(&format!("Versioned{}", variant_name)).unwrap();

    Ok((final_type, serialized_type, versions))
    // }
}

fn version_data(field: &SpannedValue<ModelEnumFieldInput>) -> Version {
    let ty = field.ty.clone();
    let ty_serialized = serialized_of(&field.ty);
    let field_version = field.version.unwrap_or(1);
    let version_name = field
        .version_name
        .clone()
        .unwrap_or_else(|| field_version.to_string());

    Version {
        version: field_version,
        ty,
        ty_serialized,
        version_name,
        version_variant: Ident::new(&format!("V{}", field_version), field.span()),
    }
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
