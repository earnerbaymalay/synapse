//! Canonical engine for Synapse: scanner, Brain model, adapter
//! trait, projection policy, and sync outcomes. Consumed by `apps/cli`
//! directly and by `apps/desktop` via Tauri commands.
//!
//! Phase 2 (this crate's current state) only defines the shapes; adapter
//! implementations, real filesystem scanning, and the sync daemon land in
//! Phase 3/4 (see PLAN.md).

#[cfg(test)]
pub(crate) mod test_home;

pub mod adapter;
pub mod adapters;
pub mod compression;
pub mod conflict_queue;
pub mod doctor;
pub mod drift;
pub mod firewall;
pub mod fold;
pub mod gateway;
pub mod hud;
pub mod mappings;
pub mod marketplace;
pub mod master_prompt;
pub mod mcp_registry;
pub mod memory;
pub mod merge;
pub mod model;
pub mod projector;
pub mod scanner;
pub mod scheduler;
pub mod secrets;
pub mod security;
pub mod snapshot;
pub mod sync;
pub mod updater;
pub mod vibe;
pub mod watcher;

pub use adapter::Adapter;
pub use compression::{
    compress_text, detect_stream_kind, estimate_tokens, execute_with_compression, CompressedOutput,
    CompressionLevel, SpoolEntry, SpoolManager, StreamKind,
};
pub use conflict_queue::{reconcile, ConflictQueue, QueuedConflict};
pub use doctor::{diagnose, Diagnosis, DoctorContext, Severity};
pub use drift::{DriftReport, DriftStatus};
pub use firewall::{mask_chat_payload, unmask_chat_response, FirewallSession};
pub use gateway::{GatewayConfig, GatewayServer, GatewayStats, UpstreamKind};
pub use hud::{format_hud, HudSnapshot};
pub use mappings::{Mapping, MappingsFile};
pub use marketplace::{MarketplaceError, MarketplaceSkill};
pub use mcp_registry::{RegistryError, RegistryServer};
pub use memory::{MemoryStats, MemoryStore, NewObservation, Observation};
pub use merge::{three_way_merge, MergeOutcome};
pub use model::{Agent, McpServer, Skill};
pub use projector::{Artifact, ProjectionPolicy};
pub use scanner::ScanResult;
pub use scheduler::{ScheduledJob, SchedulerOs};
pub use secrets::{MemorySecretStore, SecretError, SecretStore};
pub use security::{
    audit_prompt_content, sanitize_prompt_content, strip_invisible_unicode, write_atomic,
    SecurityAuditReport, SecurityFinding, ThreatCategory,
};
pub use snapshot::{SnapshotError, SnapshotLock};
pub use sync::{perform_import, perform_project, SyncOutcome};
pub use updater::{check_for_update, Channel, ReleaseManifest, UpdateDecision, UpdateError};
pub use vibe::{compute_vibe, render_genmon, render_text, VibeReport};
pub use watcher::{DebouncedEvent, DebouncedWatcher};
