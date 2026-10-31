//! Command groups. Thin wrappers that give the frontend typed access to `YtApi`, which owns all
//! provider handling. Nothing here knows which backend runs.
mod engine;
mod media;
mod queue;

/// Commands report failures as plain strings, the detail is already logged by `yt_api`.
fn err(e: impl ToString) -> String {
  e.to_string()
}
