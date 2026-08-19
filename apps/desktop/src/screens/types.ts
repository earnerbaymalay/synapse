/** The screens the desktop actually has. Both are backed by real core calls. */
export type ScreenId = 'intake' | 'examination'

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
