use crate::error::{bail, MacroError};
use crate::model::attrs::{
    ModelAttributeConfig, SharedAttributeConfig, StructFieldAttributeConfig,
};
use crate::model::{edit_where_clause, fallthrough};
use crate::{MOD_ERRORS, MOD_REGISTRY, MOD_SERIALIZATION};
use convert_case::{Case, Casing};
use darling::ast::Fields;
use darling::util::SpannedValue;
use darling::{FromField, FromMeta};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use std::borrow::Cow;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{GenericParam, Ident, Type, TypeParam};

#[derive(Debug, FromField)]
#[darling(attributes(model), forward_attrs(model_attr, model_serde))]
pub struct FieldAttributeInput {
    ident: Option<syn::Ident>,
    ty: Type,
    attrs: Vec<syn::Attribute>,
    #[darling(flatten)]
    config: StructFieldAttributeConfig,
}

#[derive(Debug)]
struct FieldData<'a> {
    name: Ident,
    original_type: &'a Type,
    serialized_type: Cow<'a, Type>,
    definition: TokenStream,
    config: &'a SharedAttributeConfig,
}

pub fn process_struct(
    model_name: Ident,
    mut generics: syn::Generics,
    attributes: Vec<syn::Attribute>,
    config: ModelAttributeConfig,
    struct_fields: Fields<SpannedValue<FieldAttributeInput>>,
) -> Result<TokenStream, MacroError> {
    let mut fields = Vec::new();
    let model_fallthrough_attrs = fallthrough(&attributes);

    let serialized_name = config
        .name
        .as_ref()
        .map(|e| format_ident!("{}", e))
        .unwrap_or_else(|| format_ident!("{}Serialized", model_name));

    let mut as_refs = vec![];

    for field in &struct_fields.fields {
        let Some(name) = &field.ident else {
            bail!(field.ty.span(), "All model fields must be named")
        };
        let ty = &field.ty;

        if field.config.as_ref.is_present() {
            as_refs.push(quote_spanned! {name.span()=>
                #[automatically_derived]
                impl AsRef<#ty> for #model_name {
                    fn as_ref(&self) -> &#ty {
                        &self.#name
                    }
                }
            })
        }

        let serialized_type = field.config.config.serialized_ty(ty)?;
        let fallthrough_attrs = fallthrough(&field.attrs);
        let definition = quote_spanned!(name.span()=>
            #(#fallthrough_attrs)*
            #name: #serialized_type
        );

        let field_data = FieldData {
            name: name.clone(),
            definition,
            original_type: ty,
            serialized_type,
            config: &field.config.config,
        };

        fields.push(field_data)
    }

    let tokens = fields.iter().map(|e| &e.definition);
    let schema_derive = config.schema_derive();

    let _reg = MOD_REGISTRY.deref();
    let ser = MOD_SERIALIZATION.deref();
    let err = MOD_ERRORS.deref();

    let (gen_imp, gen_ty, gen_wher) = generics.split_for_impl();

    let serialized_struct = quote!(
        #(#model_fallthrough_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #schema_derive
        #[serde(rename_all = "camelCase")]
        pub struct #serialized_name #generics {
            #(#tokens),*
        }

        #[automatically_derived]
        impl #gen_imp #ser::SerializationFallback for #model_name #gen_ty #gen_wher {
            type Fallback = #serialized_name;
        }

        #(#as_refs)*

        #[automatically_derived]
        impl #gen_imp  AsRef<#model_name #gen_ty> for #model_name #gen_ty #gen_wher {
            fn as_ref(&self) -> &#model_name {
                &self
            }
        }
    );

    let map_name = config.name.clone().unwrap_or_else(|| {
        format_ident!(
            "{}",
            model_name
                .to_string()
                .from_case(Case::Pascal)
                .to_case(Case::Snake)
        )
    });
    let _kind_name = format_ident!(
        "{}",
        map_name
            .to_string()
            .from_case(Case::Snake)
            .to_case(Case::Pascal)
    );
    // let _map_name = format_ident!("{}", map_name);
    // let _kind_name = format_ident!("{}", kind_name);

    let _names = fields.iter().map(|e| &e.name);

    let serialized_object_name = format_ident!("serialized");

    let modifiers: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let data = &Ident::new("item", name.span());
            let original_type = &f.original_type;
            let deser_code = f.config.deserialization_code(
                original_type,
                data,
                Some(quote!(#err::DeserializationErrorStackItem::Field(stringify!(#name)))),
            )?;
            Result::<TokenStream, MacroError>::Ok(quote_spanned! { original_type.span()=>
                #name: {
                    let #data = #serialized_object_name.#name;
                    #deser_code
                },
            })
        })
        .try_collect()?;

    let field_where_conditions = fields.iter().map(
        |FieldData {
             original_type,
             serialized_type,
             config,
             ..
         }| { config.where_condition(original_type, serialized_type) },
    );

    let field_where_conditions =
        itertools::process_results(field_where_conditions, |i| i.flatten().collect_vec())?;

    let (_, gen_ty, _) = generics.split_for_impl();
    let gen_ty = quote!(#gen_ty);

    generics
        .params
        .push(GenericParam::Type(TypeParam::from_string("Registry")?));

    let (gen_imp, _, gen_wher) = generics.split_for_impl();

    let where_condition =
        edit_where_clause(gen_wher, field_where_conditions, config.where_clauses());

    let deserialization_impl = quote! {
        #[automatically_derived]
        impl #gen_imp #ser::DeserializeModel<#model_name, Registry> for #serialized_name #gen_ty #where_condition {
            fn deserialize(self, registry: &mut Registry) -> Result<#model_name, #err::DeserializationError<Registry>> {
                let #serialized_object_name = self;

                Ok(#model_name {
                    #(#modifiers)*
                })
            }
        }
    };

    let all_together = quote! {
        #serialized_struct

        #deserialization_impl
    };

    Ok(all_together)
}
