//! Native, in-process playback engine, opt-in by depending on this crate.
//! No receiver, device or audio stream is opened at import or construction.
pub mod output;
pub mod playback;
/// Transitional compatibility state machine preserving original Pip-Boy events.
/// Consolidate with rust-radio-core after equivalence tests.
pub mod compat_controller;
