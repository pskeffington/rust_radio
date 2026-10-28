//! Native, in-process playback engine, opt-in by depending on this crate.
//! No receiver, device or audio stream is opened at import or construction.
/// Transitional compatibility state machine preserving original Pip-Boy events.
/// Consolidate with rust-radio-core after equivalence tests.
pub mod compat_controller;
pub mod discovery;
mod media;
pub mod output;
pub mod playback;
/// Explicit, bounded station checks that decode PCM without opening an output.
pub mod validation;
