import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Hint } from '../lib/hints'
import type { ExamReport, Finding } from '../screens/types'

interface AlertBannerProps {
  exam: ExamReport | null
  onHealed: () => void
  onOpenOnboard?: () => void
}

type Level = 'unread' | 'healthy' | 'warning' | 'critical'

type HealOutcome =
  | { kind: 'ok'; fixed: number; offered: number }
  | { kind: 'error'; message: string }

/**
 * The house vocabulary (components/chart.tsx, IDENTITY.md): ■ critical,
 * ▲ warning, ◐ info. The banner must use the same marks as the Examination
 * rows under it, or one glyph would mean two severities on one screen.
 */
const GLYPH: Record<Level, string> = {
  unread: '○',
  healthy: '●',
  warning: '▲',
  critical: '■',
}

const FINDING_GLYPH: Record<string, string> = { critical: '■', warning: '▲', info: '◐' }

const TONE: Record<Level, string> = {
  unread: 'border-rule text-ink-soft',
  healthy: 'border-success/40 text-success',
  warning: 'border-caution/50 text-caution',
  critical: 'border-alarm/60 text-alarm',
}

const FOCUS =
  'focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-drape'

function plural(n: number, one: string, many = `${one}s`): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`
}

/**
 * Derived from the findings, never from the report's own `criticals` /
 * `fixable` totals, exactly as the Examination screen counts them, so the
 * banner and the screen below it can never disagree.
 */
function levelOf(exam: ExamReport | null): Level {
  if (!exam) return 'unread'
  if (exam.findings.some((f) => f.severity === 'critical')) return 'critical'
  if (exam.findings.some((f) => f.severity === 'warning')) return 'warning'
  return 'healthy'
}

function errorText(e: unknown): string {
  if (e instanceof Error) return e.message
  if (typeof e === 'string') return e
  try {
    return JSON.stringify(e)
  } catch {
    return String(e)
  }
}

/**
 * Global alert line at the top of the workspace. Reports the last Doctor
 * examination as-is: unread, clean, warnings or criticals, each with its own
 * glyph and words so colour is never the only signal. When there are
 * auto-fixable findings it offers `heal_vitals`, which applies them in place.
 */
export function AlertBanner({ exam, onHealed, onOpenOnboard }: AlertBannerProps) {
  const [healing, setHealing] = useState(false)
  const [outcome, setOutcome] = useState<HealOutcome | null>(null)
  const [expanded, setExpanded] = useState(false)
  // A ref, not state: two clicks in the same tick both see `healing === false`.
  const inFlight = useRef(false)
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])

  const level = levelOf(exam)
  const findings: Finding[] = exam?.findings ?? []
  const criticals = findings.filter((f) => f.severity === 'critical').length
  const warnings = findings.filter((f) => f.severity === 'warning').length
  const infos = findings.filter((f) => f.severity === 'info').length
  const fixable = findings.filter((f) => f.auto_fixable).length
  const hasTamper = findings.some((f) => f.rule_id === 'master-prompt-tampered')

  const handleHeal = async () => {
    if (inFlight.current) return
    inFlight.current = true
    setHealing(true)
    setOutcome(null)
    const offered = fixable
    try {
      const fixed = await invoke<number>('heal_vitals', { brain: null, tools: null })
      // The Brain changed either way, so the parent still re-reads it.
      if (mounted.current) setOutcome({ kind: 'ok', fixed, offered })
      onHealed()
    } catch (e) {
      if (mounted.current) setOutcome({ kind: 'error', message: errorText(e).replace(/[.\s]+$/, '') })
    } finally {
      inFlight.current = false
      if (mounted.current) setHealing(false)
    }
  }

  let headline: string
  let detail: string | null = null
  if (level === 'unread') {
    headline = 'Vitals not read'
    detail = 'No examination has been returned yet, so nothing is reported here.'
  } else if (level === 'healthy') {
    headline = infos > 0 ? `No faults · ${plural(infos, 'note')}` : 'No faults'
    detail = null
  } else if (level === 'warning') {
    headline = plural(warnings, 'warning')
    detail = `${plural(fixable, 'finding')} auto-fixable.`
  } else {
    headline = hasTamper
      ? `${plural(criticals, 'critical finding')} · master prompt Zone 1 altered`
      : plural(criticals, 'critical finding')
    detail = [
      warnings > 0 ? plural(warnings, 'warning') : null,
      `${plural(fixable, 'finding')} auto-fixable.`,
    ]
      .filter(Boolean)
      .join(' · ')
  }

  const showActions = level === 'warning' || level === 'critical'

  return (
    <section
      aria-label="Vitals alert"
      data-level={level}
      className={`mb-4 border bg-ground px-4 py-2 font-mono text-xs ${TONE[level]}`}
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-start gap-2">
          <span aria-hidden="true" className="leading-5">
            {GLYPH[level]}
          </span>
          <div className="min-w-0">
            <p role="status" className="font-semibold uppercase tracking-wider tabular-nums leading-5">
              <span className="sr-only">
                {level === 'critical' ? 'Critical: ' : level === 'warning' ? 'Warning: ' : level === 'healthy' ? 'Healthy: ' : 'Not read: '}
              </span>
              {headline}
            </p>
            {detail && <p className="mt-0.5 text-[11px] text-ink-soft tabular-nums">{detail}</p>}
            {exam && (
              <p className="mt-0.5 truncate text-[11px] text-ink-soft" title={exam.brain}>
                brain <span className="text-ink">{exam.brain}</span>
              </p>
            )}
          </div>
        </div>

        {showActions && (
          <div className="flex flex-wrap items-center gap-2">
            {findings.length > 0 && (
              <button
                type="button"
                onClick={() => setExpanded((v) => !v)}
                aria-expanded={expanded}
                aria-controls={expanded ? 'alert-findings' : undefined}
                className={`border border-rule px-3 py-1 text-ink-soft hover:text-ink ${FOCUS}`}
              >
                {expanded ? 'Hide findings' : `Show ${plural(findings.length, 'finding')}`}
              </button>
            )}
            {fixable > 0 && (
              <button
                type="button"
                onClick={() => void handleHeal()}
                // aria-disabled, not disabled: a disabled button drops keyboard focus
                // to the page. handleHeal ignores presses while a heal is in flight.
                aria-disabled={healing || undefined}
                aria-busy={healing}
                aria-describedby="alert-heal-note"
                className={`border border-current px-3 py-1 font-semibold uppercase tracking-wider text-ink hover:bg-chart aria-disabled:cursor-wait aria-disabled:text-ink-soft aria-disabled:hover:bg-transparent ${FOCUS}`}
              >
                {healing ? 'Healing…' : `Heal ${plural(fixable, 'finding')}`}
              </button>
            )}
          </div>
        )}
      </div>

      {showActions && fixable > 0 && (
        <p id="alert-heal-note" className="mt-1 text-[11px] text-ink-soft">
          Heal rewrites the auto-fixable files in place. A snapshot of the Brain is taken first; to undo a
          heal, run <span className="text-drape">synapse rollback</span>.
        </p>
      )}

      {expanded && findings.length > 0 && (
        <ul id="alert-findings" className="mt-2 divide-y divide-rule border-t border-rule">
          {findings.map((f, i) => (
            <li key={`${f.rule_id}-${f.subject ?? ''}-${i}`} className="flex gap-2 py-1 text-[11px]">
              <span aria-hidden="true" className="w-3 shrink-0 text-center">
                {FINDING_GLYPH[f.severity] ?? '·'}
              </span>
              <span className="w-16 shrink-0 uppercase text-ink-soft">{f.severity}</span>
              <span className="min-w-0 flex-1 break-words text-ink [overflow-wrap:anywhere]">
                {f.message}
                {f.subject && <span className="ml-2 text-ink-soft">{f.subject}</span>}
              </span>
              <span className="shrink-0 text-ink-soft">{f.auto_fixable ? 'fixable' : 'manual'}</span>
            </li>
          ))}
        </ul>
      )}

      {outcome && (
        <div
          role={outcome.kind === 'error' ? 'alert' : 'status'}
          className={`mt-2 flex items-start justify-between gap-3 border-t border-rule pt-2 text-[11px] tabular-nums ${
            outcome.kind === 'error' ? 'text-alarm' : 'text-ink'
          }`}
        >
          <span>
            {outcome.kind === 'ok' ? (
              <>
                <span aria-hidden="true">● </span>
                Healed {outcome.fixed.toLocaleString()} of {plural(outcome.offered, 'auto-fixable finding')}. Re-reading
                vitals. <span className="text-ink-soft">Next</span> <span className="text-drape">synapse doctor</span>
              </>
            ) : (
              <>
                <span aria-hidden="true">▲ </span>
                Heal failed: {outcome.message}. Nothing was reported as fixed.{' '}
                <span className="text-ink-soft">Next</span> <span className="text-drape">synapse doctor --fix</span>
              </>
            )}
          </span>
          <button
            type="button"
            onClick={() => setOutcome(null)}
            aria-label="Dismiss heal result"
            className={`shrink-0 text-ink-soft hover:text-ink ${FOCUS}`}
          >
            ×
          </button>
        </div>
      )}

      {showActions && fixable > 0 && (
        <Hint id="alerts.heal">
          Heal applies only findings marked fixable; the rest stay listed for a person to resolve.
        </Hint>
      )}
    </section>
  )
}
