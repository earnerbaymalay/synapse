import { useCallback, useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

/**
 * Custom window chrome. The OS frame is turned off (`decorations: false` in
 * tauri.conf.json), so this bar owns dragging and the three window controls.
 *
 * Dragging and double-click-to-maximise come from Tauri's own drag-region
 * handler: any element carrying `data-tauri-drag-region` starts a window drag
 * on mousedown and toggles maximise on double-click. The attribute does not
 * cascade to children, so every non-interactive child repeats it, and the
 * controls (and their group) deliberately do not carry it, so a press on a
 * control is never taken as the start of a drag.
 *
 * The controls are three line glyphs: minus, plus, cross. Each carries a soft
 * neon glow drawn with `filter: drop-shadow()` on the SVG itself, so the glow
 * follows the stroke rather than the button box. The look lives in index.css
 * (`.window-control`), driven by the `--glow-*` tokens.
 */

const NOT_IN_TAURI = 'Window controls are only available in the desktop app'

type TauriWindow = ReturnType<typeof getCurrentWindow>
type WindowAction = 'minimize' | 'toggleMaximize' | 'close'

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/**
 * The current window, or null when the Tauri internals are present but
 * incomplete (a browser session with a partial shim). `getCurrentWindow()`
 * reads window metadata synchronously and throws in that case; uncaught, the
 * throw would take down the whole shell from an effect.
 */
function currentWindow(): TauriWindow | null {
  try {
    return getCurrentWindow()
  } catch {
    return null
  }
}

function warn(what: string, e: unknown) {
  console.warn(`synapse: window ${what} failed`, e)
}

type Glyph = 'minimise' | 'maximise' | 'close'

/** Thin line glyphs on a 12px grid, 1.5px stroke, drawn in currentColor. */
const GLYPH_PATH: Record<Glyph, string> = {
  minimise: 'M2 6h8',
  maximise: 'M6 2v8M2 6h8',
  close: 'M2.5 2.5l7 7M9.5 2.5l-7 7',
}

interface ControlProps {
  glyph: Glyph
  label: string
  onClick: () => void
  disabled: boolean
  danger?: boolean
  state?: string
}

function WindowControl({ glyph, label, onClick, disabled, danger = false, state }: ControlProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={disabled ? NOT_IN_TAURI : label}
      disabled={disabled}
      onClick={onClick}
      data-tone={danger ? 'danger' : undefined}
      data-state={state}
      className="window-control flex h-9 w-11 items-center justify-center disabled:cursor-not-allowed"
    >
      <svg
        viewBox="0 0 12 12"
        width={12}
        height={12}
        fill="none"
        stroke="currentColor"
        strokeWidth={1.5}
        strokeLinecap="square"
        aria-hidden="true"
        focusable="false"
        data-glyph={glyph}
      >
        <path d={GLYPH_PATH[glyph]} />
      </svg>
    </button>
  )
}

export function TitleBar(): JSX.Element {
  const available = inTauri()
  const [maximised, setMaximised] = useState(false)
  // Re-reads the maximised state; set by the effect while mounted.
  const syncRef = useRef<() => void>(() => {})

  useEffect(() => {
    if (!available) return
    const win = currentWindow()
    if (!win) return
    let alive = true
    let seq = 0
    let unlisten: (() => void) | undefined

    // Responses can arrive out of order during a resize storm: only the
    // newest answer may set the state.
    const sync = () => {
      const mine = ++seq
      let pending: Promise<boolean>
      try {
        pending = win.isMaximized()
      } catch (e) {
        warn('state read', e)
        return
      }
      pending
        .then((m) => {
          if (alive && mine === seq) setMaximised(m)
        })
        .catch((e) => warn('state read', e))
    }
    syncRef.current = sync

    sync()
    // Catches maximise by other means: double-click on the drag region, the
    // OS shortcut, window snapping.
    try {
      win
        .onResized(sync)
        .then((fn) => {
          // Unmounted before the listener registered: release it immediately.
          if (alive) unlisten = fn
          else fn()
        })
        .catch((e) => warn('resize listener', e))
    } catch (e) {
      warn('resize listener', e)
    }

    return () => {
      alive = false
      syncRef.current = () => {}
      unlisten?.()
    }
  }, [available])

  const run = useCallback(
    (action: WindowAction) => {
      if (!available) return
      const win = currentWindow()
      if (!win) return
      let pending: Promise<void>
      try {
        pending = win[action]()
      } catch (e) {
        warn(action, e)
        return
      }
      pending
        .then(() => {
          // Not every platform emits a resize for a maximise toggle; read the
          // state back so the label and glyph never lag behind the window.
          if (action === 'toggleMaximize') syncRef.current()
        })
        .catch((e) => warn(action, e))
    },
    [available],
  )

  return (
    <header
      data-tauri-drag-region
      className="flex h-9 shrink-0 select-none items-center justify-between border-b border-rule bg-ground"
    >
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-baseline gap-3 self-stretch pl-4 pt-[11px]">
        <span data-tauri-drag-region className="font-display text-[11px] leading-none tracking-wider text-ink">
          SYNAPSE
        </span>
        <span data-tauri-drag-region className="truncate font-mono text-[10px] leading-none text-ink-soft">
          clinical engine for ai context
        </span>
      </div>
      <div role="group" aria-label="Window controls" className="flex items-center border-l border-rule">
        <WindowControl glyph="minimise" label="Minimise window" onClick={() => run('minimize')} disabled={!available} />
        <WindowControl
          glyph="maximise"
          label={maximised ? 'Restore window' : 'Maximise window'}
          state={maximised ? 'maximised' : 'normal'}
          onClick={() => run('toggleMaximize')}
          disabled={!available}
        />
        <WindowControl glyph="close" label="Close window" onClick={() => run('close')} disabled={!available} danger />
      </div>
    </header>
  )
}
