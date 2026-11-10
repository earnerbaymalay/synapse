import { useCallback, useEffect, useRef, useState } from 'react'
import { ChartFrame, ChartHead, Field, Glyph, Row, plural, type Mark } from '../components/chart'
import { Hint } from '../lib/hints'
import { formatKeys, useShortcut } from '../lib/shortcuts'
import { cn } from '../lib/utils'
import type { ExamReport, Finding } from './types'

/**
 * Examination — the Doctor's findings on the Brain and its projections,
 * rendered as a clinical record.
 *
 * Read-only: the backing command runs `diagnose` and never `apply_fixes`, so
 * opening this screen cannot change the Brain. Fixes are applied from the
 * CLI, which the footer names.
 *
 * Every count on this screen is derived from the findings it lists, never
 * from the report's own `criticals` / `fixable` totals, so a header or footer
 * can't disagree with the rows underneath it.
 */

const COPY_KEYS = 'c'

type Severity = Finding['severity']

const RANK: Record<Severity, number> = { critical: 0, warning: 1, info: 2 }

/** Same mapping as the CLI (`run_doctor`): info is an observation, ◐. */
const MARK: Record<Severity, Mark> = { critical: 'critical', warning: 'warning', info: 'partial' }

const WORD_TINT: Record<Severity, string> = {
  critical: 'text-alarm',
  warning: 'text-caution',
  info: 'text-ink-soft',
}

/** Unknown severities (a newer backend) sort last and render as `·`. */
function rank(s: string): number {
  return s in RANK ? RANK[s as Severity] : 3
}

export interface Tally {
  total: number
  critical: number
  warning: number
  info: number
  fixable: number
}

export function tally(findings: Finding[]): Tally {
  const t: Tally = { total: findings.length, critical: 0, warning: 0, info: 0, fixable: 0 }
  for (const f of findings) {
    if (f.severity === 'critical') t.critical++
    else if (f.severity === 'warning') t.warning++
    else t.info++
    if (f.auto_fixable) t.fixable++
  }
  return t
}

/** Critical first, then warning, then info; stable within a severity. */
export function bySeverity(findings: Finding[]): Finding[] {
  return findings
    .map((f, i) => ({ f, i }))
    .sort((a, b) => rank(a.f.severity) - rank(b.f.severity) || a.i - b.i)
    .map(({ f }) => f)
}

export function Examination({ report }: { report: ExamReport }) {
  const findings = bySeverity(report.findings)
  const t = tally(findings)
  const clean = t.total === 0
  const nextCmd = next(t)

  const [copied, setCopied] = useState<'idle' | 'copied' | 'failed'>('idle')
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current)
  }, [])

  const copyNext = useCallback(async () => {
    let ok = false
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(nextCmd)
        ok = true
      }
    } catch {
      ok = false
    }
    setCopied(ok ? 'copied' : 'failed')
    if (timer.current) clearTimeout(timer.current)
    timer.current = setTimeout(() => setCopied('idle'), 2500)
  }, [nextCmd])

  // Scoped to this screen by lifetime: App mounts Examination only while it
  // is the active screen, so the key is unregistered everywhere else.
  useShortcut(
    { id: 'examination.copy-next', keys: COPY_KEYS, label: 'Copy the next command', group: 'Actions' },
    () => void copyNext(),
  )

  return (
    <ChartFrame>
      <ChartHead procedure="Examination" context={clean ? 'no findings' : plural(t.total, 'finding')} />

      <div className="py-3">
        <Field label="Brain" value={report.brain} />
        <Field label="Tools" value={report.tools} />
      </div>

      <p
        data-testid="exam-tally"
        className="border-b border-rule pb-2 font-mono text-xs text-ink-soft"
      >
        {clean
          ? '0 critical · 0 warning · 0 info · 0 fixable'
          : `${t.critical} critical · ${t.warning} warning · ${t.info} info · ${t.fixable} fixable`}
      </p>

      {clean ? (
        <Row mark="present" subject="brain">
          no drift, no faults
        </Row>
      ) : (
        <ol aria-label="Findings, most severe first">
          {findings.map((f, i) => (
            <FindingRow key={`${f.rule_id}-${i}`} f={f} subject={subject(f, report)} />
          ))}
        </ol>
      )}

      {!clean && (
        <Hint id="examination.glyphs">
          ■ critical needs a human · ▲ warning can wait · ◐ info is an observation. Each row also names its
          severity in words.
        </Hint>
      )}

      <div className="mt-4 border-t border-rule pt-3">
        <p className="text-sm">{finding(t)}</p>
        <div className="mt-1 flex flex-wrap items-baseline gap-x-3 font-mono text-xs">
          <p>
            <span className="text-ink-soft">Next </span>
            <span className="text-drape">{nextCmd}</span>
          </p>
          <button
            type="button"
            onClick={() => void copyNext()}
            aria-label={`Copy ${nextCmd}`}
            title={`Copy to clipboard (${formatKeys(COPY_KEYS).join(' ')})`}
            className="text-ink-soft underline-offset-2 hover:text-ink hover:underline focus-visible:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape"
          >
            copy
          </button>
          <span role="status" aria-live="polite" className="text-ink-soft">
            {copied === 'copied' ? 'copied' : copied === 'failed' ? 'clipboard unavailable, select the command' : ''}
          </span>
        </div>
        {t.fixable > 0 && (
          <Hint id="examination.fix">
            --fix applies only the rows marked fixable and leaves the rest for a human. It does not take a
            snapshot first; run synapse snapshot beforehand to keep a rollback point.
          </Hint>
        )}
      </div>
    </ChartFrame>
  )
}

/**
 * One finding: glyph, severity word, subject, message — four aligned columns,
 * so severity is carried by shape and text as well as colour. Long subjects
 * truncate with the full path in `title`; messages wrap rather than hide.
 */
function FindingRow({ f, subject }: { f: Finding; subject: string }) {
  const known = f.severity in MARK
  const mark: Mark = known ? MARK[f.severity] : 'na'
  return (
    <li
      data-severity={f.severity}
      className="border-b border-rule/60 py-1.5 last:border-0"
    >
      <div className="flex items-baseline gap-3">
        <Glyph mark={mark} />
        <span
          className={cn(
            'w-16 shrink-0 font-mono text-xs uppercase tracking-[0.06em]',
            known ? WORD_TINT[f.severity] : 'text-ink-soft',
          )}
        >
          {f.severity}
        </span>
        <span
          title={f.subject ?? subject}
          className="w-56 shrink-0 truncate font-mono text-sm"
        >
          {subject}
        </span>
        <span className="min-w-0 flex-1 break-words text-sm text-ink-soft [overflow-wrap:anywhere]">
          {f.message}
        </span>
      </div>
      <div className="mt-1 pl-7 font-mono text-xs text-ink-soft">
        <span title={f.rule_id}>rule {f.rule_id}</span>
        <span aria-hidden> · </span>
        {f.auto_fixable ? (
          <span className="text-ink">fixable with --fix</span>
        ) : (
          <span>fix by hand</span>
        )}
      </div>
    </li>
  )
}

/**
 * Shortens a finding's subject by stripping the Brain or tool root, matching
 * `chart::abbreviate` in the CLI (brain takes precedence when they nest).
 * Unlike a bare `startsWith`, the root must end at a path separator, so
 * `/x/AIBrain2/a` is not mistaken for something under `/x/AIBrain`.
 */
export function subject(f: Finding, report: Pick<ExamReport, 'brain' | 'tools'>): string {
  if (!f.subject) return 'brain'
  for (const [label, rawRoot] of [
    ['brain', report.brain],
    ['tools', report.tools],
  ] as const) {
    const root = rawRoot.replace(/[/\\]+$/, '')
    if (!root) continue
    if (f.subject === root) return label
    const sep = f.subject.charAt(root.length)
    if (f.subject.startsWith(root) && (sep === '/' || sep === '\\')) {
      const rest = f.subject.slice(root.length).replace(/^[/\\]+/, '')
      return rest ? `${label}/${rest}` : label
    }
  }
  return f.subject
}

function finding(t: Tally): string {
  if (t.total === 0) return 'Clean bill of health.'
  if (t.critical > 0) {
    const verb = t.critical === 1 ? 'needs' : 'need'
    const wait = t.warning > 0 ? ` ${plural(t.warning, 'warning')} can wait.` : ''
    return `${plural(t.critical, 'critical finding')} ${verb} a human.${wait}`
  }
  return `No critical findings. ${plural(t.total, 'observation')} noted.`
}

/**
 * Never a dead end. With fixable rows, --fix. With criticals that only a
 * human can repair, re-examine once they are repaired. Otherwise, sync.
 */
function next(t: Tally): string {
  if (t.fixable > 0) return 'synapse doctor --fix'
  if (t.critical > 0) return 'synapse doctor'
  return 'synapse sync --once'
}
