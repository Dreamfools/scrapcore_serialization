use proc_macro2::TokenStream;
use thiserror::Error;

pub fn tokens(result: Result<TokenStream, MacroError>) -> TokenStream {
    result.unwrap_or_else(|err| err.into_tokens())
}

#[derive(Debug, Error)]
#[error("{}", .0)]
pub enum MacroError {
    Syn(syn::Error),
    Darling(darling::Error),
}

impl MacroError {
    pub fn into_tokens(self) -> TokenStream {
        match self {
            MacroError::Syn(err) => err.to_compile_error(),
            MacroError::Darling(err) => err.write_errors(),
        }
    }
}

impl From<syn::Error> for MacroError {
    fn from(err: syn::Error) -> Self {
        MacroError::Syn(err)
    }
}

impl From<darling::Error> for MacroError {
    fn from(err: darling::Error) -> Self {
        MacroError::Darling(err)
    }
}

macro_rules! error {
    ($span:expr, $($t:tt)*) => {
        syn::Error::new($span, format!($($t)*))
    };
}

pub(crate) use error;

macro_rules! bail {
    ($span:expr, $($t:tt)*) => {
        return Err($crate::error::error!($span, $($t)*).into())
    };
}

pub(crate) use bail;
