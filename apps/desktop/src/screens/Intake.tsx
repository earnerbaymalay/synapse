import { ChartFoot, ChartFrame, ChartHead, Field, Row, plural } from '../components/chart'
import type { IntakeReport } from './types'

/**
 * Intake — which AI tools are installed under the scanned site, and what an
 * import would take from each.
 *
 * Every registered adapter gets a row, including the ones that are not
 * installed: absence is a finding, not an empty table (IDENTITY.md). Counts
 * are measured by actually reading each detected config; nothing here is
 * estimated.
 */
export function Intake({ report }: { report: IntakeReport }) {
  const measured = report.findings.filter((f) => f.present)
  const skills = sum(measured.map((f) => f.skills))
  const agents = sum(measured.map((f) => f.agents))
  const servers = sum(measured.map((f) => f.mcp_servers))
  const failed = measured.filter((f) => f.error).length

  return (
    <ChartFrame>
      <ChartHead
        procedure="Intake"
        context={`${report.present} of ${report.total} present`}
      />

      <div className="py-3">
        <Field label="Site" value={report.site} />
      </div>

      <div>
        {report.findings.map((f) => (
          <Row
            key={f.id}
            mark={rowMark(f)}
            subject={f.id}
            detail={f.error ?? undefined}
          >
            {describe(f)}
          </Row>
        ))}
      </div>

      <ChartFoot
        finding={
          report.present === 0
            ? 'No supported tool configs under this site.'
            : `${plural(skills, 'skill')}, ${plural(agents, 'agent')} and ` +
              `${plural(servers, 'mcp server')} would enter the Brain from ` +
              `${plural(report.present, 'tool')}. Nothing has been written.`
        }
        next={
          report.present === 0
            ? undefined
            : failed > 0
              ? 'synapse doctor'
              : 'synapse import --dry-run'
        }
      />
    </ChartFrame>
  )
}

function rowMark(f: IntakeReport['findings'][number]) {
  if (!f.present) return 'absent' as const
  if (f.error) return 'critical' as const
  return 'present' as const
}

function describe(f: IntakeReport['findings'][number]) {
  if (!f.present) return 'not present'
  if (f.error) return 'detected, but could not be read'
  return (
    <span className="font-mono text-xs">
      {plural(f.skills ?? 0, 'skill')} · {plural(f.agents ?? 0, 'agent')} ·{' '}
      {plural(f.mcp_servers ?? 0, 'mcp server')}
    </span>
  )
}

/** Sums measured counts, treating an unmeasured (`null`) reading as absent. */
function sum(values: (number | null)[]): number {
  return values.reduce<number>((total, v) => total + (v ?? 0), 0)
}
