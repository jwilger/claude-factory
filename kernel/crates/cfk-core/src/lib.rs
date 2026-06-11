//! cfk-core: Claude-Factory kernel pure functional core.
//!
//! No I/O of any kind. All public functions are pure: given the same inputs,
//! they produce the same outputs. I/O is the responsibility of cfk-engine.

pub mod types;
pub mod routing;
pub mod state_machine;
