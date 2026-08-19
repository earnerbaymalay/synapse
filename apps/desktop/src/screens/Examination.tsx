import { ChartFoot, ChartHead, Field, Row, plural } from '../components/chart'
import type { ExamReport, Finding } from './types'

/**
 * Examination — the Doctor's findings on the Brain and its projections,
 * rendered as a clinical record.
 *
 * Read-only: the backing command runs `diagnose` and never `apply_fixes`, so
 * opening this screen cannot change the Brain. Fixes are applied from the
 * CLI, which the footer names.
 */
export function Examination({ report }: { report: ExamReport }) {
  const clean = report.findings.length === 0

  return (
    <div>
      <ChartHead
        procedure="Examination"
        context={clean ? 'no findings' : plural(report.findings.length, 'finding')}
      />

      <div className="py-3">
        <Field label="Brain" value={report.brain} />
        <Field label="Tools" value={report.tools} />
      </div>

      <div>
        {clean ? (
          <Row mark="present" subject="brain">
            no drift, no faults
          </Row>
        ) : (
          report.findings.map((f, i) => (
            <Row
              key={`${f.rule_id}-${i}`}
              mark={severityMark(f.severity)}
              subject={subject(f, report)}
              detail={f.auto_fixable ? 'fixable — run doctor --fix' : undefined}
            >
              {f.message}
            </Row>
          ))
        )}
      </div>

      <ChartFoot finding={finding(report)} next={next(report)} />
    </div>
  )
}

function severityMark(severity: Finding['severity']) {
  if (severity === 'critical') return 'critical' as const
  if (severity === 'warning') return 'warning' as const
  return 'partial' as const
}

/**
 * Shortens a finding's subject by stripping the Brain or tool root, matching
 * `chart::abbreviate` in the CLI — absolute paths are far too wide for the
 * subject column.
 */
function subject(f: Finding, report: ExamReport): string {
  if (!f.subject) return 'brain'
  for (const [label, root] of [
    ['brain', report.brain],
    ['tools', report.tools],
  ] as const) {
    if (root && f.subject.startsWith(root)) {
      const rest = f.subject.slice(root.length).replace(/^[/\\]+/, '')
      return rest ? `${label}/${rest}` : label
    }
  }
  return f.subject
}

function finding(report: ExamReport): string {
  if (report.findings.length === 0) return 'Clean bill of health.'
  if (report.criticals > 0) {
    const verb = report.criticals === 1 ? 'needs' : 'need'
    return `${plural(report.criticals, 'critical finding')} ${verb} a human.`
  }
  return `No critical findings. ${plural(report.findings.length, 'observation')} noted.`
}

function next(report: ExamReport): string | undefined {
  if (report.fixable > 0) return 'neurosurgeon doctor --fix'
  if (report.criticals > 0) return undefined
  return 'neurosurgeon sync --once'
}
