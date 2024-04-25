use crate::error::{bail, MacroError};
use crate::model::attrs::{ModelAttributeConfig, SharedAttributeConfig};
use crate::model::enums::{process_enum, EnumVariantAttributeInput};
use crate::model::structs::{process_struct, FieldAttributeInput};
use crate::{serialized_of, MOD_ERRORS, MOD_REGISTRY, MOD_SERIALIZATION};
use darling::ast::Data;
use darling::FromDeriveInput;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use std::borrow::Cow;

use darling::util::SpannedValue;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{DeriveInput, Meta, Type, WhereClause};

mod enums;
mod structs;

mod attrs;

/// Derive attribute
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(my_crate),
    forward_attrs(model_attr, model_serde),
    supports(struct_named, enum_newtype, enum_unit)
)]
pub struct ModelAttributeInput {
    // Internal fields
    ident: Ident,

    generics: syn::Generics,

    attrs: Vec<syn::Attribute>,

    // Configurable fields
    #[darling(flatten)]
    config: ModelAttributeConfig,

    data: Data<SpannedValue<EnumVariantAttributeInput>, SpannedValue<FieldAttributeInput>>,
}

impl ModelAttributeConfig {
    fn schema_derive(&self) -> Option<TokenStream> {
        if self.no_schema.is_present() {
            None
        } else {
            Some(quote! {
                #[derive(schemars::JsonSchema)]
            })
        }
    }

    fn where_clauses(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.extra_conditions.iter().map(|clause| {
            let predicates = &clause.predicates;
            quote!(#predicates)
        })
    }
}

pub fn model_macro_impl(input: DeriveInput) -> Result<TokenStream, MacroError> {
    let data = ModelAttributeInput::from_derive_input(&input)?;

    match data.data {
        Data::Enum(variants) => {
            process_enum(data.ident, data.generics, data.attrs, data.config, variants)
        }
        Data::Struct(fields) => {
            process_struct(data.ident, data.generics, data.attrs, data.config, fields)
        }
    }
}

fn fallthrough(attrs: &[syn::Attribute]) -> Vec<TokenStream> {
    let mut inner_attrs = vec![];
    for attr in attrs {
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        let Some(name) = list.path.segments.last() else {
            continue;
        };
        let tokens = &list.tokens;
        match name.ident.to_string().as_str() {
            "model_attr" => {
                inner_attrs.push(quote_spanned!(attr.span()=>#[#tokens]));
            }
            "model_serde" => {
                inner_attrs.push(quote_spanned!(attr.span()=>#[serde(#tokens)]));
            }
            _ => continue,
        }
    }
    inner_attrs
}

fn edit_where_clause(
    clause: Option<&WhereClause>,
    fields_conditions: impl IntoIterator<Item = TokenStream>,
    extra_conditions: impl IntoIterator<Item = TokenStream>,
) -> TokenStream {
    let reg = MOD_REGISTRY.deref();
    let fields_conditions = fields_conditions.into_iter();
    let extra_conditions = extra_conditions.into_iter();
    if let Some(clause) = clause {
        quote! {
            #clause, #(#fields_conditions,)* #(#extra_conditions,)* Registry: #reg::PartialRegistry
        }
    } else {
        quote! {
            where #(#fields_conditions,)* #(#extra_conditions,)* Registry: #reg::PartialRegistry
        }
    }
}

impl SharedAttributeConfig {
    fn serialized_ty<'a>(&'a self, field_ty: &'a Type) -> Result<Cow<'a, Type>, MacroError> {
        // "raw" fields use their target type directly, otherwise lookup
        // `#[model(ty=T)]` or use `SerializationFallback`
        let field_ty = self.from.as_ref().unwrap_or(field_ty);
        let ty = if self.raw.is_present() {
            if self.custom_ty.is_some() || self.with.is_some() {
                bail!(self.raw.span(), "`raw` attribute field can not be used at the same time as `ty` or `with` attributes")
            }
            Cow::Borrowed(field_ty)
        } else if let Some(ty) = &self.custom_ty {
            Cow::Borrowed(ty)
        } else {
            Cow::Owned(serialized_of(field_ty)?)
        };

        Ok(ty)
    }

    fn where_condition(&self, original_type: &Type, serialized_type: &Type) -> Option<TokenStream> {
        let ser = MOD_SERIALIZATION.deref();
        let original_type = self.from.as_ref().unwrap_or(original_type);
        (self.with.is_none() && !self.raw.is_present()).then(|| {
            quote! {
                #serialized_type: #ser::DeserializeModel::<#original_type, Registry>
            }
        })
    }

    fn deserialization_code(
        &self,
        field_ty: &Type,
        varname: &Ident,
        context: Option<TokenStream>,
    ) -> Result<TokenStream, MacroError> {
        let ser = MOD_SERIALIZATION.deref();
        let err = MOD_ERRORS.deref();

        let target_type = self.from.as_ref().unwrap_or(field_ty);
        let mut blocks = vec![];
        if self.raw.is_present() {
            blocks.push(quote! {
                let #varname: #target_type = #varname;
            })
        } else if let Some(func) = &self.with {
            blocks.push(quote! {
                let #varname: #target_type = #func(#varname, registry)?;
            })
        } else {
            blocks.push(quote! {
                let #varname: #target_type = #ser::DeserializeModel::<#target_type, Registry>::deserialize(#varname, registry)?;
            })
        };

        if let Some(min) = &self.min {
            blocks.push(quote! {
                let #varname: #target_type = #ser::ApplyMin::apply(#varname, #min)?;
            })
        }

        if let Some(max) = &self.max {
            blocks.push(quote! {
                let #varname: #target_type = #ser::ApplyMax::apply(#varname, #max)?;
            })
        }

        if let Some(from) = &self.from {
            blocks.push(quote! {
                let #varname: #field_ty = <#field_ty as From<#from>>::from(#varname);
            });
        } else {
            assert_eq!(field_ty, target_type)
        }

        let context = match context {
            None => quote!(None),
            Some(ctx) => {
                quote! {
                    Some(||#ctx)
                }
            }
        };

        let code = quote! {
            #err::s_try(&mut *registry, |registry: &mut Registry| {
                #(#blocks)*
                Ok(#varname)
            }, #context)?
        };

        Ok(code)
    }
}
