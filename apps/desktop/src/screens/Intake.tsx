import { ChartFoot, ChartFrame, ChartHead, Row, plural, type Mark } from '../components/chart'
import { Hint } from '../lib/hints'
import { useShortcut } from '../lib/shortcuts'
import type { IntakeReport, ToolFinding } from './types'

/**
 * Intake — which AI tools are installed under the scanned site, and what an
 * import would take from each.
 *
 * Every registered adapter gets a row, including the ones that are not
 * installed: absence is a finding, not an empty table (IDENTITY.md). Counts
 * are measured by actually reading each detected config; nothing here is
 * estimated, and a count the backend did not measure (`null`) is shown as
 * "not measured", never as zero.
 *
 * The header and footer counts are derived from the rows themselves, so the
 * "N of M present" line can never disagree with the table under it.
 */
export function Intake({
  report,
  onRescan,
  busy = false,
}: {
  report: IntakeReport
  /** Re-runs the `intake` call. When absent, no re-scan control is shown. */
  onRescan?: () => void
  /** True while a re-scan is in flight. */
  busy?: boolean
}) {
  const findings = ordered(report.findings)
  const total = findings.length
  const present = findings.filter((f) => f.present)
  const read = present.filter((f) => status(f) === 'read')
  const failed = present.filter((f) => status(f) === 'failed')
  const unmeasured = present.filter((f) => status(f) === 'unmeasured')

  const skills = sum(read.map((f) => f.skills))
  const agents = sum(read.map((f) => f.agents))
  const servers = sum(read.map((f) => f.mcp_servers))

  return (
    <ChartFrame>
      <ChartHead
        procedure="Intake"
        context={
          <span data-testid="intake-context" className="tabular-nums">
            {present.length} of {total} present
          </span>
        }
      />

      <div className="flex items-baseline gap-3 py-3 font-mono text-xs">
        <span className="w-12 shrink-0 text-ink-soft">Site</span>
        <span
          data-testid="intake-site"
          className="min-w-0 flex-1 truncate"
          title={report.site}
        >
          {report.site || '(not reported)'}
        </span>
        {onRescan && (
          <>
            <RescanShortcut onRescan={onRescan} busy={busy} />
            <button
              type="button"
              onClick={() => {
                if (!busy) onRescan()
              }}
              // aria-disabled keeps keyboard focus on the button while it is busy
              aria-disabled={busy || undefined}
              aria-keyshortcuts="R"
              title="Re-scan this site (R)"
              className="shrink-0 text-drape outline-none hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape aria-disabled:cursor-default aria-disabled:text-ink-soft aria-disabled:hover:text-ink-soft"
            >
              {busy ? 're-scanning…' : 're-scan'}
            </button>
          </>
        )}
      </div>

      <div role="list" aria-label="Supported tools">
        {findings.map((f) => (
          <div role="listitem" key={f.id} data-tool={f.id}>
            <Row
              mark={rowMark(f)}
              subject={f.id}
              detail={
                f.error ? (
                  <span className="block break-words" title={f.error}>
                    {f.error}
                  </span>
                ) : undefined
              }
            >
              {describe(f)}
            </Row>
          </div>
        ))}
      </div>

      {total > 0 && (
        <Hint id="intake.present">
          Present means the tool's config was found under this site. Counts come
          from reading it; nothing is written.
        </Hint>
      )}

      <ChartFoot
        finding={finding({ total, present, read, failed, unmeasured, skills, agents, servers })}
        next={next({ total, present: present.length, failed: failed.length })}
      />

      <Hint id="intake.next">
        Run the Next command in a terminal. Intake only reads; import --dry-run
        previews before anything is copied.
      </Hint>
    </ChartFrame>
  )
}

/** Registers R for re-scan; mounted only when a re-scan is actually wired. */
function RescanShortcut({ onRescan, busy }: { onRescan: () => void; busy: boolean }) {
  useShortcut(
    {
      id: 'intake.rescan',
      keys: 'r',
      label: 'Re-scan site (Intake)',
      group: 'Actions',
      // Only registered while Intake is mounted, i.e. while it is on screen.
      when: () => !busy,
    },
    () => onRescan(),
  )
  return null
}

type Status = 'absent' | 'failed' | 'unmeasured' | 'read'

function status(f: ToolFinding): Status {
  if (!f.present) return 'absent'
  if (f.error) return 'failed'
  if (f.skills === null || f.agents === null || f.mcp_servers === null) return 'unmeasured'
  return 'read'
}

function rowMark(f: ToolFinding): Mark {
  switch (status(f)) {
    case 'absent':
      return 'absent'
    case 'failed':
      return 'critical'
    case 'unmeasured':
      return 'partial'
    case 'read':
      return 'present'
  }
}

function describe(f: ToolFinding) {
  switch (status(f)) {
    case 'absent':
      return 'not present'
    case 'failed':
      return 'detected, but could not be read'
    case 'unmeasured':
    case 'read':
      return (
        <span className="font-mono text-xs tabular-nums">
          <Count n={f.skills} noun="skill" width="min-w-[10ch]" /> ·{' '}
          <Count n={f.agents} noun="agent" width="min-w-[10ch]" /> ·{' '}
          <Count n={f.mcp_servers} noun="mcp server" width="min-w-[15ch]" />
        </span>
      )
  }
}

/** One fixed-width count cell so the three columns line up down the table. */
function Count({ n, noun, width }: { n: number | null; noun: string; width: string }) {
  return (
    <span className={`inline-block ${width}`}>
      {n === null ? `${noun}s not measured` : plural(n, noun)}
    </span>
  )
}

function finding({
  total,
  present,
  read,
  failed,
  unmeasured,
  skills,
  agents,
  servers,
}: {
  total: number
  present: ToolFinding[]
  read: ToolFinding[]
  failed: ToolFinding[]
  unmeasured: ToolFinding[]
  skills: number
  agents: number
  servers: number
}): string {
  if (total === 0) return 'No tool adapters are registered, so nothing was scanned.'
  if (present.length === 0)
    return `None of the ${plural(total, 'supported tool')} has a config under this site. Nothing has been written.`

  const parts: string[] = []
  if (read.length > 0) {
    parts.push(
      `${plural(skills, 'skill')}, ${plural(agents, 'agent')} and ` +
        `${plural(servers, 'mcp server')} would enter the Brain from ` +
        `${plural(read.length, 'tool')}.`,
    )
  }
  if (failed.length > 0) {
    parts.push(
      `${plural(failed.length, 'tool')} could not be read.`,
    )
  }
  if (unmeasured.length > 0) {
    parts.push(`${plural(unmeasured.length, 'tool')} could not be fully measured.`)
  }
  parts.push('Nothing has been written.')
  return parts.join(' ')
}

function next({
  total,
  present,
  failed,
}: {
  total: number
  present: number
  failed: number
}): string {
  if (total === 0 || failed > 0) return 'synapse doctor'
  if (present === 0) return 'synapse onboard'
  return 'synapse import --dry-run'
}

/** Present first, then absent, alphabetical within each group (as the CLI). */
function ordered(findings: ToolFinding[]): ToolFinding[] {
  return [...findings].sort(
    (a, b) => Number(b.present) - Number(a.present) || a.id.localeCompare(b.id),
  )
}

/** Sums measured counts; callers pass only fully measured rows. */
function sum(values: (number | null)[]): number {
  return values.reduce<number>((total, v) => total + (v ?? 0), 0)
}
