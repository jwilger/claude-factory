//! cfk-engine: Claude-Factory kernel imperative shell.
//!
//! This crate owns all I/O: the event store, routing table loading, and
//! the command handlers that call cfk-core pure functions.

pub mod checks;
pub mod emc;
pub mod commands;
pub mod config;
pub mod events;
pub mod loader;
pub mod project;
pub mod runner;
pub mod store;

#[cfg(test)]
mod tests;
