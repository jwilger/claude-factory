//! Factory phase state machines — pure functions.
//!
//! Each public function takes current state (as a list of events) and a
//! command, and returns either new events or an error. No I/O.
//!
//! This module is intentionally sparse in M1 — the full state machines
//! are implemented in M2 (development slice) and M5 (all phases).
//! M1 only needs enough to support the `cf_next_step` round-trip.

pub mod tdd;
pub mod work_item;
pub mod review;

pub use work_item::{WorkItem, WorkItemState, WorkItemError};
