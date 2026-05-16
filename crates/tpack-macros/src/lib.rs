mod ast;
mod emit;
mod parse;
mod util;

use proc_macro::TokenStream;
use util::compile_error;

#[proc_macro_derive(TpackSerialize, attributes(tpack))]
pub fn derive_tpack_serialize(input: TokenStream) -> TokenStream {
    match parse::parse_item(input).and_then(|item| emit::impl_serialize(&item)) {
        Ok(tokens) => tokens,
        Err(err) => compile_error(&err),
    }
}

#[proc_macro_derive(TpackDeserialize, attributes(tpack))]
pub fn derive_tpack_deserialize(input: TokenStream) -> TokenStream {
    match parse::parse_item(input).and_then(|item| emit::impl_deserialize(&item)) {
        Ok(tokens) => tokens,
        Err(err) => compile_error(&err),
    }
}
