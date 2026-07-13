import type { ReactNode } from 'react'
import { cn } from '../lib/utils'

/** A bordered panel — the boxed sections every DESIGN_PACK.md wireframe uses. */
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
        'rounded-lg border border-slate-800 bg-slate-900/60 p-4',
        className,
      )}
    >
      {title && (
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-slate-300">
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
  ok: 'bg-emerald-500/15 text-emerald-400',
  warn: 'bg-amber-500/15 text-amber-400',
  error: 'bg-red-500/15 text-red-400',
  idle: 'bg-slate-500/15 text-slate-400',
}

const TONE_DOT: Record<Tone, string> = {
  ok: 'bg-emerald-400',
  warn: 'bg-amber-400',
  error: 'bg-red-400',
  idle: 'bg-slate-500',
}

/** A colored status badge — "Status: OK" / "Syncing..." / "Error" across every screen. */
export function StatusPill({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-xs font-medium',
        TONE_STYLES[tone],
      )}
    >
      <span className={cn('h-1.5 w-1.5 rounded-full', TONE_DOT[tone])} />
      {children}
    </span>
  )
}

/** The bottom action-bar row every wireframe has (e.g. "← Cancel │ Apply │ Save"). */
export function Toolbar({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-900/60 px-4 py-3">
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
        'rounded-md px-3 py-1.5 text-sm font-medium transition-colors',
        primary
          ? 'bg-primary text-white hover:bg-primary-600'
          : 'text-slate-300 hover:bg-slate-800',
      )}
    >
      {children}
    </button>
  )
}

export function PageHeader({ title, subtitle }: { title: string; subtitle?: string }) {
  return (
    <div className="mb-6">
      <h1 className="text-xl font-semibold text-white">{title}</h1>
      {subtitle && <p className="mt-1 text-sm text-slate-400">{subtitle}</p>}
    </div>
  )
}
