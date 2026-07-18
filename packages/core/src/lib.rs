//! Canonical engine for LLM Neurosurgeon: scanner, Brain model, adapter
//! trait, projection policy, and sync outcomes. Consumed by `apps/cli`
//! directly and by `apps/desktop` via Tauri commands.
//!
//! Phase 2 (this crate's current state) only defines the shapes; adapter
//! implementations, real filesystem scanning, and the sync daemon land in
//! Phase 3/4 (see PLAN.md).

pub mod adapter;
pub mod adapters;
pub mod conflict_queue;
pub mod doctor;
pub mod drift;
pub mod mappings;
pub mod marketplace;
pub mod mcp_registry;
pub mod merge;
pub mod model;
pub mod projector;
pub mod scanner;
pub mod scheduler;
pub mod secrets;
pub mod snapshot;
pub mod sync;
pub mod updater;
pub mod watcher;

pub use adapter::Adapter;
pub use conflict_queue::{reconcile, ConflictQueue, QueuedConflict};
pub use doctor::{diagnose, Diagnosis, DoctorContext, Severity};
pub use drift::{DriftReport, DriftStatus};
pub use mappings::{Mapping, MappingsFile};
pub use marketplace::{MarketplaceError, MarketplaceSkill};
pub use mcp_registry::{RegistryError, RegistryServer};
pub use merge::{three_way_merge, MergeOutcome};
pub use model::{Agent, McpServer, Skill};
pub use projector::{Artifact, ProjectionPolicy};
pub use scanner::ScanResult;
pub use scheduler::{ScheduledJob, SchedulerOs};
pub use secrets::{MemorySecretStore, SecretError, SecretStore};
pub use snapshot::{SnapshotError, SnapshotLock};
pub use sync::SyncOutcome;
pub use updater::{check_for_update, Channel, ReleaseManifest, UpdateDecision, UpdateError};
pub use watcher::{DebouncedEvent, DebouncedWatcher};
