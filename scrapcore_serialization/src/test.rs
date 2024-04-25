use std::convert::Infallible;
use std::fmt::Debug;
use std::path::PathBuf;
use thiserror::Error;

use scrapcore_serialization_macro::{registry, DatabaseModel};

#[derive(Debug, DatabaseModel)]
pub struct A {
    c: String,
}

#[derive(Debug, DatabaseModel)]
pub struct B {
    #[model(raw, min = 5, from = "u16")]
    a: u32,
    #[model(ty="f32", with=test_a)]
    b: u32,
    #[model(from = "String")]
    c: PathBuf,
}

#[derive(Debug, DatabaseModel)]
pub enum C {
    Var1(f32),
    Var2(A),
    Var3,
}

fn test_a<R>(_i: f32, _: R) -> Result<u32, Infallible> {
    todo!()
}

#[registry(error = ModelError)]
pub struct Model {
    #[model(collection)]
    test: A,
    #[model(singleton)]
    test_single: B,
    #[model(collection)]
    test_enum: C,
    #[model(asset)]
    test_asset: Option<A>,
}

#[derive(Debug, Clone, Error)]
pub enum ModelError {}

#[test]
fn run_test() {}
