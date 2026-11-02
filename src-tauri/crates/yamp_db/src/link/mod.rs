//! Keeps the library the same on every device of one user.
//!
//! Devices never talk to each other. They share a private folder on the user's own cloud
//! account through `cloud-kit` and the library converges through it.
//!
//! What is shared is the state of whole entities, keyed by kind and id: tracks, likes,
//! followed artists, albums and playlists. A playlist travels as one document with its
//! track list, not as a series of edits. Two devices that change the same entity
//! meanwhile do not merge, the later change wins. That is simple to reason about and
//! right for a library, where nobody edits the same playlist on two devices at once.
//!
//! Downloaded audio is not shared. A file exists on one device only.
//!
//! The folder has one subfolder per device that only this device writes, which is the
//! rule `cloud-kit` asks for since no backend has atomic writes:
//!
//! ```text
//! devices/<id>/meta.json      name, last seen, which entries of the others were read
//! devices/<id>/log/…          what this device changed, one entry per push
//! devices/<id>/snapshot.json  what the snapshot of this device covers
//! snapshots/<id>              the whole library of this device, as a blob
//! ```
//!
//! A device sends by appending to its own log and receives by reading the logs of the
//! others from a stored cursor. Logs would grow forever, so a device deletes entries that
//! everyone has read. A device that was away for longer, or a new one, starts from a
//! snapshot and reads the logs from where the snapshot ends, see `snapshot`.

#![allow(dead_code)]

mod doc;
mod export;
mod kind;
mod state;
