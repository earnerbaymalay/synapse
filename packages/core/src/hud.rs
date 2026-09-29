//! SYNAPSE Live Terminal Cockpit / Vital Signs HUD
//!
//! Provides a zero-telemetry, ultra-low-RAM terminal HUD for monitoring
//! local AI coding agent telemetry, real-time Sensory Gating compression,
//! and `~/AIBrain` synchronization lockstep.

use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::doctor::{diagnose, DoctorContext};
use crate::gateway::GatewayStats;

/// Real-time snapshot of the developer's entire AI environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudSnapshot {
    pub gateway_active: bool,
    pub gateway_bind: String,
    pub upstream_url: String,
    pub stats: GatewayStats,
    pub brain_path: String,
    pub total_adapters: usize,
    pub doctor_healthy: bool,
    pub doctor_criticals: usize,
    pub doctor_warnings: usize,
}

impl HudSnapshot {
    /// Polls the local gateway and inspects the canonical Brain
    pub fn collect(gateway_addr: &str, brain_dir: &Path, tool_root: &Path) -> Self {
        // 1. Check Gateway Health & Stats
        let health_url = format!("http://{}/health", gateway_addr);
        let (active, upstream, stats) = match ureq::get(&health_url).timeout(std::time::Duration::from_millis(500)).call() {
            Ok(res) => {
                if let Ok(json_val) = res.into_json::<Value>() {
                    let upstream = json_val["upstream"].as_str().unwrap_or("Unknown").to_string();
                    let stats: GatewayStats = serde_json::from_value(json_val["stats"].clone()).unwrap_or_default();
                    (true, upstream, stats)
                } else {
                    (true, "Connected".to_string(), GatewayStats::default())
                }
            }
            Err(_) => (false, "Offline (Start with `synapse serve`)".to_string(), GatewayStats::default()),
        };

        // 2. Run Doctor Diagnostics on Brain
        let adapters = crate::adapters::all_adapters();
        let total_adapters = adapters.len();

        let ctx = DoctorContext {
            brain_root: brain_dir.to_path_buf(),
            tool_root: tool_root.to_path_buf(),
            mappings_path: brain_dir.join("mappings.json"),
        };
        let issues = diagnose(&ctx);
        let criticals = issues.iter().filter(|i| i.severity == crate::doctor::Severity::Critical).count();
        let warnings = issues.iter().filter(|i| i.severity == crate::doctor::Severity::Warning).count();

        Self {
            gateway_active: active,
            gateway_bind: gateway_addr.to_string(),
            upstream_url: upstream,
            stats,
            brain_path: brain_dir.display().to_string(),
            total_adapters,
            doctor_healthy: criticals == 0,
            doctor_criticals: criticals,
            doctor_warnings: warnings,
        }
    }
}

/// Formats the complete HUD display into an ANSI clinical terminal view
pub fn format_hud(snap: &HudSnapshot, use_color: bool) -> String {
    let cyan = if use_color { "\x1b[1;36m" } else { "" };
    let green = if use_color { "\x1b[1;32m" } else { "" };
    let red = if use_color { "\x1b[1;31m" } else { "" };
    let yellow = if use_color { "\x1b[1;33m" } else { "" };
    let bold = if use_color { "\x1b[1m" } else { "" };
    let dim = if use_color { "\x1b[2m" } else { "" };
    let reset = if use_color { "\x1b[0m" } else { "" };

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "{cyan}┌──────────────────────────────────────────────────────────────────────────┐{reset}\n"
    ));
    out.push_str(&format!(
        "{cyan}│{reset}  {bold}SYNAPSE VITAL SIGNS HUD // RUNTIME COCKPIT{reset}                     {dim}v1.0.0{reset}  {cyan}│{reset}\n"
    ));
    out.push_str(&format!(
        "{cyan}└──────────────────────────────────────────────────────────────────────────┘{reset}\n\n"
    ));

    // Gateway Section
    let gw_status = if snap.gateway_active {
        format!("{green}● ONLINE{reset} (Listening on {})", snap.gateway_bind)
    } else {
        format!("{red}○ OFFLINE{reset} (Run `synapse serve` to activate)")
    };

    out.push_str(&format!("{bold}🩺 RUNTIME GATEWAY{reset}\n"));
    out.push_str(&format!("  Status:   {}\n", gw_status));
    out.push_str(&format!("  Upstream: {cyan}{}{reset}\n", snap.upstream_url));
    if !snap.stats.last_model.is_empty() {
        out.push_str(&format!("  Model:    {bold}{}{reset} ({dim}{}ms latency{reset})\n", snap.stats.last_model, snap.stats.last_latency_ms));
    }
    out.push_str("\n");

    // Sensory Gating & Token Metrics
    out.push_str(&format!("{bold}⚡ SYNAPTIC SENSORY GATING & TOKEN METRICS{reset}\n"));
    out.push_str(&format!("  Requests Handled:     {bold}{}{reset}\n", snap.stats.total_requests));
    out.push_str(&format!("  Raw Prompt Tokens:    {dim}{}{reset}\n", snap.stats.prompt_tokens_raw));
    out.push_str(&format!("  Gated Prompt Tokens:  {cyan}{}{reset}\n", snap.stats.prompt_tokens_gated));
    out.push_str(&format!(
        "  Tokens Pruned/Saved:  {green}{}{reset} ({bold}{}% reduction{reset})\n\n",
        snap.stats.tokens_saved, snap.stats.avg_reduction_percent
    ));

    // Canonical Brain & Doctor Status
    let brain_status = if snap.doctor_healthy {
        if snap.doctor_warnings > 0 {
            format!("{yellow}⚠ ATTENTION REQUIRED{reset} ({} warnings)", snap.doctor_warnings)
        } else {
            format!("{green}✔ LOCKSTEP SYNCED{reset} (0 faults)",)
        }
    } else {
        format!("{red}✖ CORRUPTED / DRIFTED{reset} ({} criticals, run `synapse doctor --fix`)", snap.doctor_criticals)
    };

    out.push_str(&format!("{bold}🏛️ CANONICAL BRAIN (THE ORGAN){reset}\n"));
    out.push_str(&format!("  Path:      {dim}{}{reset}\n", snap.brain_path));
    out.push_str(&format!("  Adapters:  {bold}{} verified tools{reset}\n", snap.total_adapters));
    out.push_str(&format!("  Doctor:    {}\n\n", brain_status));

    // Command Bar
    out.push_str(&format!(
        "{dim}────────────────────────────────────────────────────────────────────────────{reset}\n"
    ));
    out.push_str(&format!(
        "  {bold}Shortcuts:{reset} [s] Sync Brain  •  [d] Doctor Diagnostic  •  [p] Project  •  [q] Quit\n"
    ));

    out
}
