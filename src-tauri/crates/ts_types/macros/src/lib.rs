//! Derive macro for `ts-types`. Used through the `ts_types` crate,
//! which re-exports everything here.

use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned};

mod ts_derive;

/// Derive a TypeScript equivalent for a struct / newtype / unit-only enum.
/// Emitted as an `export interface` / `export type` into the generated file.
/// Honors `#[serde(rename_all, rename, skip)]`. Per-field override with
/// `#[ts(type = "string")]`, and `#[ts(skip)]`.
#[proc_macro_derive(TsType, attributes(ts))]
pub fn derive_ts_type(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  ts_derive::expand(input)
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

pub(crate) fn err<T>(span: impl Spanned, msg: &str) -> syn::Result<T> {
  Err(syn::Error::new(span.span(), msg))
}
