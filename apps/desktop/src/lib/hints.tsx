import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'

/**
 * Hints: small, quiet tips shown from first boot until the user turns them off.
 *
 * - On by default the first time the app runs (no stored preference = on).
 * - Global off switch persists across launches (localStorage).
 * - Individual hints can be dismissed and stay dismissed.
 *
 * Usage:
 *   <HintsProvider> ... </HintsProvider>            (once, in the shell)
 *   <Hint id="intake.rescan">Press R to re-scan</Hint>
 *   const { enabled, setEnabled } = useHints()
 */

const KEY_ENABLED = 'synapse.hints.enabled'
const KEY_DISMISSED = 'synapse.hints.dismissed'

function readEnabled(): boolean {
  try {
    const v = localStorage.getItem(KEY_ENABLED)
    return v === null ? true : v === 'true'
  } catch {
    return true
  }
}

function readDismissed(): string[] {
  try {
    const v = localStorage.getItem(KEY_DISMISSED)
    const parsed = v ? JSON.parse(v) : []
    return Array.isArray(parsed) ? parsed.filter((x) => typeof x === 'string') : []
  } catch {
    return []
  }
}

export interface HintsApi {
  enabled: boolean
  setEnabled: (on: boolean) => void
  toggle: () => void
  isDismissed: (id: string) => boolean
  dismiss: (id: string) => void
  resetDismissed: () => void
}

const HintsContext = createContext<HintsApi | null>(null)

export function HintsProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabledState] = useState<boolean>(readEnabled)
  const [dismissed, setDismissed] = useState<string[]>(readDismissed)

  useEffect(() => {
    try {
      localStorage.setItem(KEY_ENABLED, String(enabled))
    } catch {
      /* storage unavailable: preference lives for this session only */
    }
  }, [enabled])

  useEffect(() => {
    try {
      localStorage.setItem(KEY_DISMISSED, JSON.stringify(dismissed))
    } catch {
      /* ignore */
    }
  }, [dismissed])

  const setEnabled = useCallback((on: boolean) => setEnabledState(on), [])
  const toggle = useCallback(() => setEnabledState((v) => !v), [])
  const isDismissed = useCallback((id: string) => dismissed.includes(id), [dismissed])
  const dismiss = useCallback((id: string) => setDismissed((d) => (d.includes(id) ? d : [...d, id])), [])
  const resetDismissed = useCallback(() => setDismissed([]), [])

  const api = useMemo<HintsApi>(
    () => ({ enabled, setEnabled, toggle, isDismissed, dismiss, resetDismissed }),
    [enabled, setEnabled, toggle, isDismissed, dismiss, resetDismissed],
  )
  return <HintsContext.Provider value={api}>{children}</HintsContext.Provider>
}

/** Safe outside a provider (tests, isolated screens): behaves as "hints off". */
export function useHints(): HintsApi {
  const ctx = useContext(HintsContext)
  return (
    ctx ?? {
      enabled: false,
      setEnabled: () => {},
      toggle: () => {},
      isDismissed: () => true,
      dismiss: () => {},
      resetDismissed: () => {},
    }
  )
}

/** A subtle inline tip. Renders nothing when hints are off or this hint was dismissed. */
export function Hint({ id, children, className }: { id: string; children: ReactNode; className?: string }) {
  const { enabled, isDismissed, dismiss } = useHints()
  if (!enabled || isDismissed(id)) return null
  return (
    <p
      role="note"
      data-hint={id}
      className={
        'mt-2 flex items-start gap-2 font-mono text-[11px] leading-snug text-ink-soft ' + (className ?? '')
      }
    >
      <span aria-hidden="true" className="text-drape">›</span>
      <span className="flex-1">{children}</span>
      <button
        type="button"
        onClick={() => dismiss(id)}
        aria-label="Dismiss this hint"
        className="text-ink-soft/70 hover:text-ink focus-visible:text-ink"
      >
        ×
      </button>
    </p>
  )
}
