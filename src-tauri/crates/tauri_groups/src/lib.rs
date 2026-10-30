//! Grouped Tauri commands + generated TypeScript bindings.
//!
//! - `#[command(group)]` / `#[group]` register commands in a link-time registry.
//! - [`handler`] builds the `invoke_handler` from the registry.
//! - [`export_ts`] writes `commands.ts` (the invoke wrappers) and `types.ts`
//!   (the `TsType` declarations from `ts-types`) from the registries.
//!
//! `TsType` and its derive are re-exported from `ts-types`.

mod registry;

pub use registry::{export_ts, handler, render_commands_ts, Arg, CommandEntry, Sig};
pub use tauri_groups_macros::{command, group};
pub use ts_types::{render_types_ts, TsType, TypeEntry};

#[doc(hidden)]
pub mod __private {
  pub use inventory;
}
