use crate::error::tokens;
use crate::model::model_macro_impl;

use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};

use std::path::Path;
use crate::registry::registry_impl_inner;

fn check_model(path: &Path) -> String {
    let data = std::fs::read_to_string(path).expect("Should be able to read the file");
    let mut stream: TokenStream = data
        .parse()
        .expect("Test case data should be a valid rust file");
    let tokens = tokens(model_macro_impl(
        syn::parse2(stream.clone()).expect("Test input should be a valid derive input"),
    ));

    stream.append_all(tokens);

    let file: syn::File = syn::parse2(stream).expect("Should generate valid rust code");

    prettyplease::unparse(&file)
}

fn check_registry(path: &Path) -> String {
    let data = std::fs::read_to_string(path).expect("Should be able to read the file");
    let (attribute, structure) = data.split_once("\n").expect("Test case data should have an attribute-marked input");

    let attribute = attribute.trim_start_matches("#[registry(").trim_end_matches(")]");

    let mut attribute: TokenStream = attribute
        .parse()
        .expect("Test case data should be a valid rust file");
    let mut structure: TokenStream = structure
        .parse()
        .expect("Test case data should be a valid rust file");
    let tokens = tokens(registry_impl_inner(
        attribute.clone(),
        syn::parse2(structure.clone()).expect("Test input should be a struct"),
    ));

    let mut stream = TokenStream::new();

    stream.append_all(quote! {#[registry(#attribute)]});
    stream.append_all(structure);
    stream.append_all(tokens);

    let file: syn::File = syn::parse2(stream).expect("Should generate valid rust code");

    prettyplease::unparse(&file)

}

#[test]
fn struct_tests() {
    insta::glob!("test_cases/model/struct/*.rs", |path| {
        insta::assert_snapshot!(check_model(path))
    });
}


#[test]
fn enum_tests() {
    insta::glob!("test_cases/model/enum/*.rs", |path| {
        insta::assert_snapshot!(check_model(path))
    });
}

#[test]
fn registry_tests() {
    insta::glob!("test_cases/registry/*.rs", |path| {
        insta::assert_snapshot!(check_registry(path))
    });
}
