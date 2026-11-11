import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'

/**
 * Hints: small, quiet tips shown from first boot until the user turns them off.
 *
 * - On by default the first time the app runs (no stored preference = on).
 * - Global off switch persists across launches (localStorage).
 * - Individual hints can be dismissed and stay dismissed.
 * - Other windows of the app stay in sync through the `storage` event.
 * - Storage that is missing, full, blocked or holding junk never throws: the
 *   app falls back to defaults and keeps the preference for this session.
 *
 * Usage:
 *   <HintsProvider> ... </HintsProvider>            (once, in the shell)
 *   <Hint id="intake.rescan">Press R to re-scan</Hint>
 *   const { enabled, setEnabled } = useHints()
 */

export const HINTS_KEY_ENABLED = 'synapse.hints.enabled'
export const HINTS_KEY_DISMISSED = 'synapse.hints.dismissed'
/** Shortcut that toggles hints. The shell registers it; the coach advertises it. */
export const HINTS_TOGGLE_KEYS = 'mod+h'

/** Upper bound on remembered dismissals, so a corrupted or runaway list stays small. */
const MAX_DISMISSED = 500

function storage(): Storage | null {
  try {
    return typeof window !== 'undefined' && window.localStorage ? window.localStorage : null
  } catch {
    // Accessing localStorage itself can throw (sandboxed or privacy modes).
    return null
  }
}

export function safeGet(key: string): string | null {
  try {
    return storage()?.getItem(key) ?? null
  } catch {
    return null
  }
}

export function safeSet(key: string, value: string): void {
  try {
    storage()?.setItem(key, value)
  } catch {
    /* quota or blocked storage: the value lives for this session only */
  }
}

function parseEnabled(v: string | null): boolean {
  // Only an explicit "false" turns hints off. Missing or unrecognised = on.
  return v === null ? true : v.trim() !== 'false'
}

function parseDismissed(v: string | null): string[] {
  if (!v) return []
  try {
    const parsed: unknown = JSON.parse(v)
    if (!Array.isArray(parsed)) return []
    const ids = parsed.filter((x): x is string => typeof x === 'string' && x.length > 0)
    return Array.from(new Set(ids)).slice(-MAX_DISMISSED)
  } catch {
    return []
  }
}

const readEnabled = (): boolean => parseEnabled(safeGet(HINTS_KEY_ENABLED))
const readDismissed = (): string[] => parseDismissed(safeGet(HINTS_KEY_DISMISSED))

export interface HintsApi {
  enabled: boolean
  setEnabled: (on: boolean) => void
  toggle: () => void
  isDismissed: (id: string) => boolean
  dismiss: (id: string) => void
  resetDismissed: () => void
  /** How many hints the user has dismissed one by one (so a UI can offer them back). */
  dismissedCount: number
}

const HintsContext = createContext<HintsApi | null>(null)

export function HintsProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabledState] = useState<boolean>(readEnabled)
  const [dismissed, setDismissed] = useState<string[]>(readDismissed)

  useEffect(() => {
    safeSet(HINTS_KEY_ENABLED, String(enabled))
  }, [enabled])

  useEffect(() => {
    safeSet(HINTS_KEY_DISMISSED, JSON.stringify(dismissed))
  }, [dismissed])

  // Cross-window sync. `storage` only fires in *other* documents, and only when
  // the value actually changes, so echoing the value back cannot loop.
  useEffect(() => {
    if (typeof window === 'undefined') return
    const onStorage = (e: StorageEvent) => {
      // sessionStorage changes fire the same event; only localStorage holds hints.
      if (!fromLocalStorage(e)) return
      if (e.key === null) {
        // localStorage.clear() in another window: back to first-boot defaults.
        setEnabledState(true)
        setDismissed([])
        return
      }
      if (e.key === HINTS_KEY_ENABLED) setEnabledState(parseEnabled(e.newValue))
      else if (e.key === HINTS_KEY_DISMISSED) {
        const next = parseDismissed(e.newValue)
        setDismissed((cur) => (sameList(cur, next) ? cur : next))
      }
    }
    window.addEventListener('storage', onStorage)
    return () => window.removeEventListener('storage', onStorage)
  }, [])

  const setEnabled = useCallback((on: boolean) => setEnabledState(on), [])
  const toggle = useCallback(() => setEnabledState((v) => !v), [])
  const dismissedSet = useMemo(() => new Set(dismissed), [dismissed])
  const isDismissed = useCallback((id: string) => dismissedSet.has(id), [dismissedSet])
  const dismiss = useCallback(
    (id: string) => setDismissed((d) => (d.includes(id) ? d : [...d, id].slice(-MAX_DISMISSED))),
    [],
  )
  const resetDismissed = useCallback(() => setDismissed((d) => (d.length === 0 ? d : [])), [])

  const dismissedCount = dismissed.length
  const api = useMemo<HintsApi>(
    () => ({ enabled, setEnabled, toggle, isDismissed, dismiss, resetDismissed, dismissedCount }),
    [enabled, setEnabled, toggle, isDismissed, dismiss, resetDismissed, dismissedCount],
  )
  return <HintsContext.Provider value={api}>{children}</HintsContext.Provider>
}

/** True when a storage event came from localStorage (or the area is unknown). */
export function fromLocalStorage(e: StorageEvent): boolean {
  if (!e.storageArea) return true
  const local = storage()
  return local === null || e.storageArea === local
}

function sameList(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((x, i) => x === b[i])
}

const OFF_API: HintsApi = {
  enabled: false,
  setEnabled: () => {},
  toggle: () => {},
  isDismissed: () => true,
  dismiss: () => {},
  resetDismissed: () => {},
  dismissedCount: 0,
}

/** Safe outside a provider (tests, isolated screens): behaves as "hints off". */
export function useHints(): HintsApi {
  return useContext(HintsContext) ?? OFF_API
}

/**
 * Fade in on mount, unless the user prefers reduced motion. Starts visible when
 * motion is reduced so nothing ever sits at opacity 0.
 */
export function useFadeIn(): boolean {
  const [shown, setShown] = useState<boolean>(prefersReducedMotion)
  useEffect(() => {
    if (shown) return
    const raf = typeof requestAnimationFrame === 'function' ? requestAnimationFrame : (cb: () => void) => setTimeout(cb, 0)
    const cancel = typeof cancelAnimationFrame === 'function' ? cancelAnimationFrame : clearTimeout
    const id = raf(() => setShown(true)) as number
    return () => cancel(id)
  }, [shown])
  return shown
}

function prefersReducedMotion(): boolean {
  try {
    return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
      : false
  } catch {
    return false
  }
}

/** Shared focus ring for hint controls: square, 1px, accent coloured, keyboard only. */
export const HINT_FOCUS_RING =
  'outline-none focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape'

/**
 * A subtle inline tip. Renders nothing when hints are off or this hint was dismissed.
 * One line of 11px mono with a fixed line box, so showing or dismissing it moves
 * surrounding content by a single quiet line at most.
 */
export function Hint({ id, children, className }: { id: string; children: ReactNode; className?: string }) {
  const { enabled, isDismissed, dismiss } = useHints()
  const shown = useFadeIn()
  if (!enabled || isDismissed(id)) return null
  return (
    <p
      role="note"
      data-hint={id}
      className={
        'mt-2 flex items-start gap-2 font-mono text-[11px] leading-[16px] text-ink-soft ' +
        'motion-safe:transition-opacity motion-safe:duration-300 motion-reduce:transition-none ' +
        (shown ? 'opacity-100 ' : 'opacity-0 ') +
        (className ?? '')
      }
    >
      <span aria-hidden="true" className="text-drape">
        ›
      </span>
      <span className="min-w-0 flex-1">{children}</span>
      <button
        type="button"
        onClick={() => dismiss(id)}
        aria-label="Dismiss this hint"
        title="Dismiss this hint"
        className={
          'h-4 w-4 shrink-0 text-center leading-[16px] text-ink-soft hover:text-ink focus-visible:text-ink ' +
          HINT_FOCUS_RING
        }
      >
        ×
      </button>
    </p>
  )
}
