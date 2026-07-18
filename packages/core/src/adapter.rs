//! The per-tool adapter contract. Phase 3 (T3.1) implements one of these
//! per supported tool (see `docs/research/` for the 12 recon briefs); this
//! crate only defines the trait so the scanner/projector can be written
//! against a stable interface ahead of time.

use crate::model::{Agent, McpServer, Skill};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportResult {
    pub skills: Vec<Skill>,
    pub agents: Vec<Agent>,
    pub mcp_servers: Vec<McpServer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResult {
    pub written: Vec<String>,
    pub symlinked: Vec<String>,
}

/// A single AI tool's config format, e.g. Claude Code, Cursor, Aider.
pub trait Adapter {
    /// Stable identifier, e.g. `"claude-code"`, `"cursor"`.
    fn id(&self) -> &'static str;

    /// True if this tool's config is present under `root`.
    fn detect(&self, root: &std::path::Path) -> bool;

    /// Losslessly ingest this tool's config into the canonical model.
    fn import(&self, root: &std::path::Path) -> Result<ImportResult, AdapterError>;

    /// Project the canonical model back out to this tool's config.
    fn project(
        &self,
        root: &std::path::Path,
        skills: &[Skill],
        agents: &[Agent],
        mcp_servers: &[McpServer],
    ) -> Result<ProjectResult, AdapterError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    NotFound(String),
    Malformed(String),
    Io(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::NotFound(msg) => write!(f, "not found: {msg}"),
            AdapterError::Malformed(msg) => write!(f, "malformed config: {msg}"),
            AdapterError::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for AdapterError {}
