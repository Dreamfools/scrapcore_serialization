use crate::error::tokens;
use crate::model::model_macro_impl;

use proc_macro2::TokenStream;
use quote::TokenStreamExt;

use std::path::Path;

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

#[test]
fn struct_tests() {
    insta::glob!("test_cases/model/struct/*.rs", |path| {
        insta::assert_snapshot!(check_model(path))
    });
}
