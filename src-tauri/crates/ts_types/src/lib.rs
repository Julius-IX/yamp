//! Generated TypeScript declarations for Rust types.
//!
//! - `#[derive(TsType)]` registers a data type (DB tables etc) in a link-time registry.
//! - [`TsType`] maps a Rust type to its TS type expression.
//! - [`render_types_ts`] builds the `types.ts` content from the registry.

mod registry;
mod ts_type;

pub use registry::{render_types_ts, type_names, TypeEntry, HEADER};
pub use ts_type::TsType;
pub use ts_types_macros::TsType;

#[doc(hidden)]
pub mod __private {
  pub use inventory;
}
