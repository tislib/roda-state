//! Roda is an ultra-high-performance, low-latency state computer for real-time analytics and event-driven systems.
//!
//! It enables building deterministic streaming pipelines with cache-friendly dataflows,
//! wait-free reads, and explicit memory bounds.

mod components;
mod engine;
mod journal_store;
mod macros;
pub mod measure;
mod op_counter;
mod pipe;
mod stage;
mod stage_engine;
mod storage;

pub use crate::components::*;
pub use crate::engine::RodaEngine;
pub use crate::journal_store::{JournalStore, JournalStoreOptions, StoreJournalReader};
pub use crate::pipe::*;
pub use crate::stage::{OutputCollector, Stage, StageExt};
pub use crate::stage_engine::StageEngine;
