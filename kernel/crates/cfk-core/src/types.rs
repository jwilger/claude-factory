//! Semantic domain types for the factory kernel.
//!
//! Every value has a type that carries its meaning. No raw primitives escape
//! this module into domain code.

pub mod ids;
pub mod phase;
pub mod slice;
pub mod step;
pub mod routing;
pub mod gate;
pub mod lease;
pub mod evidence;
