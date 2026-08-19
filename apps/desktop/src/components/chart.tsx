import type { ReactNode } from 'react'
import { cn } from '../lib/utils'

/**
 * The clinical record primitives — the React counterpart to the CLI's
 * `chart.rs`, so both surfaces read alike (see IDENTITY.md).
 *
 * Structure comes from horizontal rules and column alignment, the way a
 * printed form does. There are no cards, no rounded corners and no shadows:
 * those are what make a dashboard look like every other dashboard.
 */

// ── status vocabulary ───────────────────────────────────────────────────

/** The fixed six-mark vocabulary from IDENTITY.md. No emoji, ever. */
export type Mark = 'present' | 'partial' | 'warning' | 'critical' | 'absent' | 'na'

const GLYPH: Record<Mark, string> = {
  present: '●',
  partial: '◐',
  warning: '▲',
  critical: '■',
  absent: '○',
  na: '·',
}

const TINT: Record<Mark, string> = {
  present: 'text-drape',
  partial: 'text-caution',
  warning: 'text-caution',
  critical: 'text-alarm',
  absent: 'text-ink-soft',
  na: 'text-ink-soft',
}

/** The one-cell status glyph. Fixed width so every row's columns line up. */
export function Glyph({ mark }: { mark: Mark }) {
  return (
    <span
      aria-hidden
      className={cn('inline-block w-4 shrink-0 text-center font-mono', TINT[mark])}
    >
      {GLYPH[mark]}
    </span>
  )
}

// ── chart furniture ─────────────────────────────────────────────────────

/**
 * Opens a chart: the procedure name on the left, measured context on the
 * right, and the rule underneath.
 */
export function ChartHead({
  procedure,
  context,
}: {
  procedure: string
  context?: ReactNode
}) {
  return (
    <div className="flex items-baseline justify-between border-b border-rule pb-2">
      <h1 className="font-mono text-sm font-semibold uppercase tracking-[0.14em]">
        Neurosurgeon <span className="text-ink-soft">·</span> {procedure}
      </h1>
      {context !== undefined && (
        <p className="font-mono text-xs text-ink-soft">{context}</p>
      )}
    </div>
  )
}

/** A labelled scalar above the table, e.g. `Site  /home/you/project`. */
export function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex gap-3 py-0.5 font-mono text-xs">
      <span className="w-12 shrink-0 text-ink-soft">{label}</span>
      <span className="break-all">{value}</span>
    </div>
  )
}

/**
 * One chart row: mark, subject, then free-form detail, with an optional
 * continuation line underneath.
 *
 * The subject sits in a fixed column so detail text aligns down the page.
 * Subjects are paths and can be long, so the column wraps rather than
 * truncating — a clinical record does not hide the thing it is describing —
 * and `break-all` keeps a long path inside its own column instead of running
 * under the neighbouring text.
 *
 * `detail` renders inside the row's ruled block rather than after it, so the
 * rule stays between findings instead of cutting one in half.
 *
 * Absent rows are dimmed but never hidden — absence is a finding.
 */
export function Row({
  mark,
  subject,
  detail,
  children,
}: {
  mark: Mark
  subject: string
  detail?: ReactNode
  children?: ReactNode
}) {
  return (
    <div className="border-b border-rule/60 py-1.5 last:border-0">
      <div className="flex items-baseline gap-3">
        <Glyph mark={mark} />
        <span
          className={cn(
            'w-56 shrink-0 break-all font-mono text-sm',
            mark === 'absent' && 'text-ink-soft',
          )}
        >
          {subject}
        </span>
        <span className="min-w-0 flex-1 text-sm text-ink-soft">{children}</span>
      </div>
      {detail !== undefined && (
        // Indented to the subject column, not the detail column: at this
        // width a deeper indent would push continuations off to the right of
        // everything they belong to.
        <div className="mt-1 pl-7 font-mono text-xs text-ink-soft">{detail}</div>
      )}
    </div>
  )
}

/**
 * Closes a chart with the finding and the next command to run.
 *
 * The `next` hint is the identity's first commitment: no surface is a dead
 * end. It names a real CLI invocation, so the desktop teaches the CLI rather
 * than competing with it.
 */
export function ChartFoot({ finding, next }: { finding: string; next?: string }) {
  return (
    <div className="mt-4 border-t border-rule pt-3">
      <p className="text-sm">{finding}</p>
      {next && (
        <p className="mt-1 font-mono text-xs">
          <span className="text-ink-soft">Next </span>
          <span className="text-drape">{next}</span>
        </p>
      )}
    </div>
  )
}

/** Pluralises `noun` against `n` — "1 tool" / "3 tools". */
export function plural(n: number, noun: string, plural?: string): string {
  return `${n} ${n === 1 ? noun : (plural ?? `${noun}s`)}`
}
