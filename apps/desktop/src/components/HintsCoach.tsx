import { useCallback, useEffect, useState } from 'react'
import { HINT_FOCUS_RING, HINTS_TOGGLE_KEYS, safeGet, safeSet, useFadeIn, useHints } from '../lib/hints'
import { formatKeys } from '../lib/shortcuts'

/**
 * HintsCoach: a quiet ruled panel in the bottom-right corner that tells the user
 * hints are on and how to turn them off. Shown while hints are enabled and until
 * the user presses "Got it". Never takes focus; announced politely.
 */

export const COACH_KEY_DISMISSED = 'synapse.hints.coach.dismissed'

const readCoachDismissed = (): boolean => safeGet(COACH_KEY_DISMISSED) === 'true'

export function HintsCoach(): JSX.Element | null {
  const { enabled, setEnabled } = useHints()
  const [dismissed, setDismissed] = useState<boolean>(readCoachDismissed)
  const shown = useFadeIn()

  // Keep in step with other windows.
  useEffect(() => {
    if (typeof window === 'undefined') return
    const onStorage = (e: StorageEvent) => {
      if (e.key === null) setDismissed(false)
      else if (e.key === COACH_KEY_DISMISSED) setDismissed(e.newValue === 'true')
    }
    window.addEventListener('storage', onStorage)
    return () => window.removeEventListener('storage', onStorage)
  }, [])

  const gotIt = useCallback(() => {
    setDismissed(true)
    safeSet(COACH_KEY_DISMISSED, 'true')
  }, [])

  if (!enabled || dismissed) return null

  const toggleKeys = formatKeys(HINTS_TOGGLE_KEYS).join(' ')
  const helpKeys = formatKeys('?').join(' ')
  const button =
    'border border-rule bg-ground px-2 py-0.5 font-mono text-[11px] leading-[16px] text-ink-soft ' +
    'hover:border-ink-soft hover:text-ink focus-visible:text-ink ' +
    HINT_FOCUS_RING

  return (
    <aside
      role="status"
      aria-live="polite"
      aria-label="Hints"
      data-hints-coach=""
      className={
        'w-full border-t border-rule bg-ground px-3 py-2 ' +
        'font-mono text-[11px] leading-[16px] text-ink-soft ' +
        'motion-safe:transition-opacity motion-safe:duration-300 motion-reduce:transition-none ' +
        (shown ? 'opacity-100' : 'opacity-0')
      }
    >
      <p className="flex items-start gap-2">
        <span aria-hidden="true" className="text-drape">
          ›
        </span>
        <span className="min-w-0 flex-1">
          Hints are on. <kbd className="text-ink">{toggleKeys}</kbd> turns them off ·{' '}
          <kbd className="text-ink">{helpKeys}</kbd> lists every shortcut.
        </span>
      </p>
      <div className="mt-2 flex justify-end gap-2">
        <button type="button" className={button} onClick={() => setEnabled(false)}>
          Turn hints off
        </button>
        <button type="button" className={button} onClick={gotIt}>
          Got it
        </button>
      </div>
    </aside>
  )
}
