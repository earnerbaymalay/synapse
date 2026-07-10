//! Canonical engine for LLM Neurosurgeon: scanner, Brain model, adapter
//! trait, projection policy, and sync outcomes. Consumed by `apps/cli`
//! directly and by `apps/desktop` via Tauri commands.
//!
//! Phase 2 (this crate's current state) only defines the shapes; adapter
//! implementations, real filesystem scanning, and the sync daemon land in
//! Phase 3/4 (see PLAN.md).

pub mod adapter;
pub mod adapters;
pub mod model;
pub mod projector;
pub mod scanner;
pub mod sync;

pub use adapter::Adapter;
pub use model::{Agent, McpServer, Skill};
pub use projector::ProjectionPolicy;
pub use scanner::ScanResult;
pub use sync::SyncOutcome;
