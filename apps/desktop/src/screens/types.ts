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
  gateway_status: string
  upstream_url: string
  model: string
  latency_ms: number
  requests_handled: number
  raw_prompt_tokens: number
  gated_prompt_tokens: number
  tokens_saved: number
  reduction_pct: number
  brain_path: string
  adapters_present: number
  adapters_total: number
  doctor_criticals: number
  doctor_fixable: number
}
