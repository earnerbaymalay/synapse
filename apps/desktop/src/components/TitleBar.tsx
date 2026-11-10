import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

/**
 * Custom window chrome. The OS frame is turned off (`decorations: false` in
 * tauri.conf.json), so this bar owns dragging and the three window controls.
 *
 * Dragging and double-click-to-maximise come from Tauri's own drag-region
 * handler: any element carrying `data-tauri-drag-region` starts a window drag
 * on mousedown and toggles maximise on double-click. The attribute does not
 * cascade to children, so every non-interactive child repeats it.
 */

const NOT_IN_TAURI = 'Window controls are only available in the desktop app'

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

type Glyph = 'minimise' | 'maximise' | 'restore' | 'close'

/** Thin line glyphs on a 12px grid, 1.5px stroke, drawn in currentColor. */
function GlyphPaths({ glyph }: { glyph: Glyph }) {
  switch (glyph) {
    case 'minimise':
      return <path d="M2 6h8" />
    case 'maximise':
      return <path d="M6 2v8M2 6h8" />
    case 'restore':
      // Two offset squares: the conventional "restore down" mark, in line.
      return (
        <>
          <path d="M2.5 4.5h5v5h-5z" />
          <path d="M4.5 4.5v-2h5v5h-2" />
        </>
      )
    case 'close':
      return <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
  }
}

function GlyphSvg({ glyph, className }: { glyph: Glyph; className: string }) {
  return (
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
      className={className}
    >
      <GlyphPaths glyph={glyph} />
    </svg>
  )
}

interface ControlProps {
  glyph: Glyph
  label: string
  onClick: () => void
  disabled: boolean
  danger?: boolean
}

function WindowControl({ glyph, label, onClick, disabled, danger = false }: ControlProps) {
  // The glow is a blurred copy of the glyph sitting behind the crisp one, so it
  // follows the stroke shape rather than the button box. Dim by default, a
  // touch stronger on hover/focus; close shifts to the alarm colour on hover.
  const glowHover = danger
    ? 'group-hover:text-alarm group-hover:opacity-70 group-focus-visible:opacity-60'
    : 'group-hover:opacity-60 group-focus-visible:opacity-60'
  const inkHover = danger ? 'group-hover:text-alarm' : 'group-hover:text-ink'

  return (
    <button
      type="button"
      aria-label={label}
      title={disabled ? NOT_IN_TAURI : label}
      disabled={disabled}
      onClick={onClick}
      className="group flex h-9 w-11 items-center justify-center outline-none focus-visible:outline focus-visible:outline-1 focus-visible:-outline-offset-2 focus-visible:outline-drape disabled:cursor-not-allowed disabled:opacity-40"
    >
      <span className="relative block h-3 w-3">
        <GlyphSvg
          glyph={glyph}
          className={`absolute inset-0 text-drape opacity-30 blur-[2px] transition-[opacity,color] duration-150 group-disabled:opacity-0 ${glowHover}`}
        />
        <GlyphSvg
          glyph={glyph}
          className={`absolute inset-0 text-ink-soft transition-colors duration-150 group-focus-visible:text-ink group-disabled:text-ink-soft ${inkHover}`}
        />
      </span>
    </button>
  )
}

export function TitleBar(): JSX.Element {
  const available = inTauri()
  const [maximised, setMaximised] = useState(false)

  useEffect(() => {
    if (!available) return
    const win = getCurrentWindow()
    let alive = true
    let unlisten: (() => void) | undefined

    const sync = () => {
      win
        .isMaximized()
        .then((m) => {
          if (alive) setMaximised(m)
        })
        .catch(() => {})
    }

    sync()
    win
      .onResized(sync)
      .then((fn) => {
        // Unmounted before the listener registered: release it immediately.
        if (alive) unlisten = fn
        else fn()
      })
      .catch(() => {})

    return () => {
      alive = false
      unlisten?.()
    }
  }, [available])

  const run = (action: 'minimize' | 'toggleMaximize' | 'close') => () => {
    if (!available) return
    void getCurrentWindow()[action]().catch(() => {})
  }

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
      <div className="flex items-center border-l border-rule">
        <WindowControl glyph="minimise" label="Minimise window" onClick={run('minimize')} disabled={!available} />
        <WindowControl
          glyph={maximised ? 'restore' : 'maximise'}
          label={maximised ? 'Restore window' : 'Maximise window'}
          onClick={run('toggleMaximize')}
          disabled={!available}
        />
        <WindowControl glyph="close" label="Close window" onClick={run('close')} disabled={!available} danger />
      </div>
    </header>
  )
}
