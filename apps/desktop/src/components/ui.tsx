import type { ReactNode } from 'react'
import { cn } from '../lib/utils'

/** A bordered panel — square corners, hairline border, corner registration marks. */
export function Card({
  title,
  icon,
  className,
  children,
}: {
  title?: string
  icon?: ReactNode
  className?: string
  children: ReactNode
}) {
  return (
    <div
      className={cn(
        'relative rounded-none border border-ink-800 bg-ink-900/60 p-4 transition-colors hover:border-ink-700',
        className,
      )}
    >
      {/* Corner Registration Marks */}
      <span className="pointer-events-none absolute -left-px -top-px h-1.5 w-1.5 border-l border-t border-accent-500/60" />
      <span className="pointer-events-none absolute -right-px -top-px h-1.5 w-1.5 border-r border-t border-accent-500/60" />
      <span className="pointer-events-none absolute -bottom-px -left-px h-1.5 w-1.5 border-b border-l border-accent-500/60" />
      <span className="pointer-events-none absolute -bottom-px -right-px h-1.5 w-1.5 border-b border-r border-accent-500/60" />

      {title && (
        <h3 className="mb-3 flex items-center gap-2 font-mono text-xs font-semibold uppercase tracking-wider text-ink-300">
          {icon}
          {title}
        </h3>
      )}
      {children}
    </div>
  )
}

export type Tone = 'ok' | 'warn' | 'error' | 'idle'

const TONE_STYLES: Record<Tone, string> = {
  ok: 'border-semantic-success/30 bg-semantic-success/10 text-semantic-success',
  warn: 'border-semantic-warning/30 bg-semantic-warning/10 text-semantic-warning',
  error: 'border-semantic-error/30 bg-semantic-error/10 text-semantic-error',
  idle: 'border-ink-700 bg-ink-800/40 text-ink-300',
}

const TONE_DOT: Record<Tone, string> = {
  ok: 'bg-semantic-success',
  warn: 'bg-semantic-warning',
  error: 'bg-semantic-error',
  idle: 'bg-ink-400',
}

/** A colored status badge — square hairline border, mono text. */
export function StatusPill({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-none border px-2 py-0.5 font-mono text-xs font-medium uppercase tracking-wider',
        TONE_STYLES[tone],
      )}
    >
      <span className={cn('h-1.5 w-1.5 rounded-none', TONE_DOT[tone])} />
      {children}
    </span>
  )
}

/** The bottom action-bar row — hairline border, ink-900 background. */
export function Toolbar({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-center gap-2 rounded-none border border-ink-800 bg-ink-900/60 px-4 py-3">
      {children}
    </div>
  )
}

export function ToolbarButton({
  children,
  primary,
  onClick,
}: {
  children: ReactNode
  primary?: boolean
  onClick?: () => void
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'rounded-none border px-3 py-1.5 font-mono text-xs font-semibold uppercase tracking-wider transition-colors',
        primary
          ? 'border-accent-500 bg-accent-500 text-ink-950 hover:border-accent-300 hover:bg-accent-300'
          : 'border-ink-700 bg-ink-900/80 text-ink-300 hover:border-ink-600 hover:text-ink-100',
      )}
    >
      {children}
    </button>
  )
}

export function PageHeader({ title, subtitle }: { title: string; subtitle?: string }) {
  return (
    <div className="mb-6">
      <h1 className="font-display text-lg tracking-wider text-ink-100 uppercase">{title}</h1>
      {subtitle && <p className="mt-1 font-mono text-xs text-accent-500 uppercase tracking-widest">{subtitle}</p>}
    </div>
  )
}
