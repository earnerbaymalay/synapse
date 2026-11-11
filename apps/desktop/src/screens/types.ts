/** The screens the desktop actually has. All backed by real core calls. */
export type ScreenId = 'intake' | 'examination' | 'cockpit' | 'memory' | 'spool'

/** One adapter's presence under the scanned site, from `intake`. */
export interface ToolFinding {
  id: string
  present: boolean
  /** `null` means "not measured" — which is not the same as zero. */
  skills: number | null
  agents: number | null
  mcp_servers: number | null
  error: string | null
}

export interface IntakeReport {
  site: string
  total: number
  present: number
  findings: ToolFinding[]
}

/** One Doctor diagnosis, from `examine`. */
export interface Finding {
  rule_id: string
  severity: 'critical' | 'warning' | 'info'
  message: string
  subject: string | null
  auto_fixable: boolean
}

export interface ExamReport {
  brain: string
  tools: string
  criticals: number
  fixable: number
  findings: Finding[]
}

/** Synaptic Memory Cyber Node */
export interface MemoryNodeDto {
  id: string
  label: string
  kind: string // 'core' | 'project' | 'observation' | 'file' | 'change' | 'discovery'
  narrative: string
  files: string[]
  ts: number
  val: number
}

/** Synaptic Memory Cyber Edge */
export interface MemoryEdgeDto {
  source: string
  target: string
  label: string
}

export interface MemoryGraphPayload {
  nodes: MemoryNodeDto[]
  edges: MemoryEdgeDto[]
  total_observations: number
}

/** Real-time Cockpit Telemetry */
export interface CockpitTelemetry {
  // Gateway fields are null until the backend reads a running gateway;
  // today it never does, and the Cockpit shows "not measured".
  gateway_status: string | null
  upstream_url: string | null
  model: string | null
  latency_ms: number | null
  requests_handled: number | null
  raw_prompt_tokens: number | null
  gated_prompt_tokens: number | null
  tokens_saved: number | null
  reduction_pct: number | null
  brain_path: string
  adapters_present: number
  adapters_total: number
  doctor_criticals: number
  doctor_fixable: number
}

/**
 * `check_for_update` result (synapse_core::updater::DryRunResult). The
 * decision is internally tagged on `status` (serde snake_case). No screen
 * calls this command yet; the shape is here so the first caller is typed.
 */
export type UpdateDecision =
  | { status: 'up_to_date'; current: string }
  | {
      status: 'update_available'
      current: string
      version: string
      url: string
      signature: string
      notes: string
    }
  | { status: 'unsupported_platform'; target: string }
  | { status: 'no_release_for_channel'; channel: 'stable' | 'beta' }

export interface UpdateCheckResult {
  decision: UpdateDecision
  report: string
  signing: 'not_configured' | 'configured'
}

/**
 * One spooled run, from `spool_list` (commands.rs `SpoolEntryDto`). Mirrors
 * the interface exported from Spool.tsx. `command` is `null` until core
 * actually records the command that produced a log.
 */
export interface SpoolEntryDto {
  id: string
  /** Unix seconds (the log file's mtime). */
  timestamp: number
  raw_bytes: number
  log_path: string
  command: string | null
}
