use crate::error::{bail, MacroError};
use crate::model::attrs::{EnumVariantAttributeConfig, ModelAttributeConfig};
use crate::model::{edit_where_clause, fallthrough};
use crate::{MOD_ERRORS, MOD_REGISTRY, MOD_SERIALIZATION};
use darling::ast::Fields;
use darling::util::SpannedValue;
use darling::{FromField, FromMeta, FromVariant};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use std::ops::Deref;
use syn::{GenericParam, TypeParam};

#[derive(Debug, FromVariant)]
#[darling(attributes(model))]
pub struct EnumVariantAttributeInput {
    ident: Ident,
    fields: Fields<EnumVariantField>,
    attrs: Vec<syn::Attribute>,

    #[darling(flatten)]
    config: EnumVariantAttributeConfig,
}

#[derive(Debug, FromField)]
struct EnumVariantField {
    ident: Option<Ident>,
    ty: syn::Type,
}

pub fn process_enum(
    model_name: Ident,
    mut generics: syn::Generics,
    attributes: Vec<syn::Attribute>,
    config: ModelAttributeConfig,
    variants: Vec<SpannedValue<EnumVariantAttributeInput>>,
) -> Result<TokenStream, MacroError> {
    let model_fallthrough_attrs = fallthrough(&attributes);
    let serialized_name = config
        .name
        .as_ref()
        .map(|e| format_ident!("{}", e))
        .unwrap_or_else(|| format_ident!("{}Serialized", model_name));

    let _reg = MOD_REGISTRY.deref();
    let ser = MOD_SERIALIZATION.deref();
    let err = MOD_ERRORS.deref();

    let variants = variants.iter().map(|variant| {
        let field = match variant.fields.iter().at_most_one() {
            Ok(field) => field,
            Err(_) => {
                bail!(
                    variant.span(),
                    "Enum variant must have exactly zero or one field"
                );
            }
        };

        let fallthrough_attrs = fallthrough(&variant.attrs);
        let variant_name = variant.config.rename.as_ref().unwrap_or(&variant.ident);

        let (serialized_variant, deserialization_match, where_condition) =
            if let Some(field) = field {
                if field.ident.is_some() {
                    bail!(variant.span(), "Only newtype enums are supported");
                }

                let serialized_ty = variant.config.config.serialized_ty(&field.ty)?;

                let serialized_variant = quote_spanned! {variant.span()=>
                    #(#fallthrough_attrs)*
                    #variant_name(#serialized_ty),
                };

                let item_var = Ident::new("item", variant.span());

                let deser_code = variant.config.config.deserialization_code(
                &field.ty,
                &item_var,
                Some(
                    quote!(#err::DeserializationErrorStackItem::Variant(stringify!(#variant_name))),
                ),
            )?;

                let deserialization_match = quote_spanned! {variant.span()=>
                    Self::#variant_name(#item_var) => #model_name::#variant_name(#deser_code),
                };

                let where_condition = variant
                    .config
                    .config
                    .where_condition(&field.ty, &serialized_ty)?;

                (serialized_variant, deserialization_match, where_condition)
            } else {
                let serialized_variant = quote_spanned! {variant.span()=>
                    #(#fallthrough_attrs)*
                    #variant_name,
                };
                let deserialization_match = quote_spanned! {variant.span()=>
                    Self::#variant_name => #model_name::#variant_name,
                };
                let where_condition = None;
                (serialized_variant, deserialization_match, where_condition)
            };

        Result::<(TokenStream, TokenStream, Option<TokenStream>), MacroError>::Ok((
            serialized_variant,
            deserialization_match,
            where_condition,
        ))
    });

    let (members, deser, where_conditions) = itertools::process_results(variants, |iter| {
        iter.multiunzip::<(Vec<_>, Vec<_>, Vec<_>)>()
    })?;

    let where_conditions = where_conditions.into_iter().flatten();

    let schema_derive = config.schema_derive();
    let (gen_imp, gen_ty, gen_wher) = generics.split_for_impl();
    let defs = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #schema_derive
        #(#model_fallthrough_attrs)*
        pub enum #serialized_name {
            #(#members)*
        }

        #[automatically_derived]
        impl #gen_imp #ser::SerializationFallback for #model_name #gen_ty #gen_wher {
            type Fallback = #serialized_name;
        }

        #[automatically_derived]
        impl #gen_imp  AsRef<#model_name #gen_ty> for #model_name #gen_ty #gen_wher {
            fn as_ref(&self) -> &#model_name {
                &self
            }
        }

        // #[automatically_derived]
        // impl #serialization_mod::ModelDeserializable<#model_name> for #serialized_name {
        //     fn deserialize(self, registry: &mut #model_mod::PartialModRegistry) -> Result<#model_name, #serialization_mod::DeserializationError> {
        //         Ok(match self {
        //             #(#deserialization)*
        //         })
        //     }
        // }
    };

    let gen_ty = quote!(#gen_ty);

    generics
        .params
        .push(GenericParam::Type(TypeParam::from_string("Registry")?));

    let (gen_imp, _, gen_wher) = generics.split_for_impl();

    let where_condition = edit_where_clause(gen_wher, where_conditions, config.where_clauses());

    let deser_code = quote! {
        #[automatically_derived]
        impl #gen_imp #ser::DeserializeModel<#model_name, Registry> for #serialized_name #gen_ty #where_condition {
            fn deserialize(self, registry: &mut Registry) -> Result<#model_name, #err::DeserializationError<Registry>> {
                Ok(match self {
                    #(#deser)*
                })
            }
        }
    };

    Ok(quote! {
        #defs
        #deser_code
    })
}
