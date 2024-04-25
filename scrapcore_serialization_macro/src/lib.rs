use proc_macro::TokenStream;
use std::fmt::Display;
use std::ops::Deref;

use crate::error::{tokens, MacroError};
use lazy_static::lazy_static;
use quote::{quote, ToTokens};

use syn::{parse_macro_input, parse_str, DeriveInput, Type};

use crate::model::model_macro_impl;
use crate::registry::registry_impl;

mod error;
mod model;
mod registry;

#[cfg(test)]
mod tests;

#[derive(Debug)]
struct IdentSync(String);

impl IdentSync {
    fn join(&self, path: &str) -> IdentSync {
        IdentSync(format!("{}::{}", self.0, path))
    }
}

impl Display for IdentSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ToTokens for IdentSync {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty: Type = parse_str(&self.0).unwrap();
        // proc_macro2::Ident::new(&self.0, Span::call_site()).to_tokens(tokens)
        ty.to_tokens(tokens)
    }
}

lazy_static! {
    static ref SERIALIZATION_CRATE: IdentSync = crate_name("scrapcore_serialization");
    static ref MOD_REGISTRY: IdentSync = SERIALIZATION_CRATE.join("registry");
    static ref MOD_SERIALIZATION: IdentSync = SERIALIZATION_CRATE.join("serialization");
    static ref MOD_ERRORS: IdentSync = MOD_SERIALIZATION.join("error");
}

fn crate_name(name: &str) -> IdentSync {
    match proc_macro_crate::crate_name(name) {
        Ok(data) => IdentSync(match data {
            proc_macro_crate::FoundCrate::Itself => "crate".to_string(),
            proc_macro_crate::FoundCrate::Name(name) => name,
        }),
        // Crate not found, so just use the name and let user get the compiler error
        Err(_) => IdentSync(name.to_string()),
    }
}

fn serialized_of(ty: &Type) -> Result<Type, MacroError> {
    let ser = MOD_SERIALIZATION.deref();
    Ok(syn::parse2(quote! {
        <#ty as #ser::SerializationFallback>::Fallback
    })?)
}

#[proc_macro_derive(DatabaseModel, attributes(model, model_attr, model_serde))]
pub fn database_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    tokens(model_macro_impl(input)).into()
}

#[proc_macro_attribute]
pub fn registry(attr: TokenStream, input: TokenStream) -> TokenStream {
    registry_impl(attr, input)
}
