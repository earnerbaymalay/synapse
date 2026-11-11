import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const win = vi.hoisted(() => ({
  minimize: vi.fn(() => Promise.resolve()),
  toggleMaximize: vi.fn(() => Promise.resolve()),
  close: vi.fn(() => Promise.resolve()),
  isMaximized: vi.fn(() => Promise.resolve(false)),
  onResized: vi.fn((_handler: () => void) => Promise.resolve(() => {})),
}))

const shim = vi.hoisted(() => ({ broken: false }))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => {
    // What the real API does when __TAURI_INTERNALS__ has no window metadata.
    if (shim.broken) throw new TypeError("Cannot read properties of undefined (reading 'currentWindow')")
    return win
  },
}))

import { TitleBar } from '../TitleBar'

type TauriWindow = Window & { __TAURI_INTERNALS__?: unknown }

function setTauri(on: boolean) {
  if (on) (window as TauriWindow).__TAURI_INTERNALS__ = {}
  else delete (window as TauriWindow).__TAURI_INTERNALS__
}

beforeEach(() => {
  vi.clearAllMocks()
  shim.broken = false
  win.isMaximized.mockImplementation(() => Promise.resolve(false))
  win.minimize.mockImplementation(() => Promise.resolve())
  win.toggleMaximize.mockImplementation(() => Promise.resolve())
  win.close.mockImplementation(() => Promise.resolve())
})

function deferred<T>() {
  let resolve!: (v: T) => void
  const promise = new Promise<T>((r) => (resolve = r))
  return { promise, resolve }
}

afterEach(() => setTauri(false))

describe('TitleBar', () => {
  it('renders the wordmark and three labelled controls in order', () => {
    render(<TitleBar />)
    expect(screen.getByText('SYNAPSE')).toBeInTheDocument()
    const buttons = screen.getAllByRole('button')
    expect(buttons.map((b) => b.getAttribute('aria-label'))).toEqual([
      'Minimise window',
      'Maximise window',
      'Close window',
    ])
  })

  it('marks the bar as a drag region', () => {
    const { container } = render(<TitleBar />)
    expect(container.querySelector('header')).toHaveAttribute('data-tauri-drag-region')
  })

  it('disables controls outside Tauri and explains why', () => {
    setTauri(false)
    render(<TitleBar />)
    for (const b of screen.getAllByRole('button')) {
      expect(b).toBeDisabled()
      expect(b.getAttribute('title')).toMatch(/desktop app/i)
    }
    expect(win.isMaximized).not.toHaveBeenCalled()
  })

  it('calls the matching window method inside Tauri', async () => {
    setTauri(true)
    render(<TitleBar />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())

    fireEvent.click(screen.getByRole('button', { name: 'Minimise window' }))
    expect(win.minimize).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByRole('button', { name: 'Maximise window' }))
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByRole('button', { name: 'Close window' }))
    expect(win.close).toHaveBeenCalledTimes(1)
  })

  it('switches to a restore label when the window is maximised', async () => {
    setTauri(true)
    render(<TitleBar />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())
    expect(screen.getByRole('button', { name: 'Maximise window' })).toHaveAttribute('title', 'Maximise window')

    win.isMaximized.mockImplementation(() => Promise.resolve(true))
    const handler = win.onResized.mock.calls[0][0]
    await act(async () => {
      handler()
    })
    expect(await screen.findByRole('button', { name: 'Restore window' })).toBeEnabled()
  })

  it('releases the resize listener on unmount', async () => {
    setTauri(true)
    const unlisten = vi.fn()
    win.onResized.mockImplementationOnce(() => Promise.resolve(unlisten))
    const { unmount } = render(<TitleBar />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())
    await act(async () => {})
    unmount()
    expect(unlisten).toHaveBeenCalledTimes(1)
  })

  it('draws minus, plus and cross as single line glyphs that carry the glow class', () => {
    render(<TitleBar />)
    const paths = screen.getAllByRole('button').map((b) => {
      // One SVG per control: the glow is a drop-shadow on that SVG (index.css
      // .window-control), not a second blurred copy or a box behind it.
      expect(b.querySelectorAll('svg')).toHaveLength(1)
      expect(b).toHaveClass('window-control')
      return b.querySelector('path')?.getAttribute('d')
    })
    expect(paths).toEqual(['M2 6h8', 'M6 2v8M2 6h8', 'M2.5 2.5l7 7M9.5 2.5l-7 7'])
  })

  it('keeps the plus glyph when maximised and exposes the state', async () => {
    setTauri(true)
    win.isMaximized.mockImplementation(() => Promise.resolve(true))
    render(<TitleBar />)
    const btn = await screen.findByRole('button', { name: 'Restore window' })
    expect(btn).toHaveAttribute('data-state', 'maximised')
    expect(btn.querySelector('path')?.getAttribute('d')).toBe('M6 2v8M2 6h8')
  })

  it('gives every control at least a 28px hit target', () => {
    render(<TitleBar />)
    for (const b of screen.getAllByRole('button')) {
      // h-9 w-11 = 36 x 44 css px
      expect(b).toHaveClass('h-9', 'w-11')
    }
  })

  it('keeps the drag region off the controls so presses are clicks, not drags', () => {
    render(<TitleBar />)
    const group = screen.getByRole('group', { name: 'Window controls' })
    expect(group).not.toHaveAttribute('data-tauri-drag-region')
    for (const el of group.querySelectorAll('*')) expect(el).not.toHaveAttribute('data-tauri-drag-region')
    expect(screen.getByText('SYNAPSE')).toHaveAttribute('data-tauri-drag-region')
  })

  it('reads the state back after a toggle even when no resize event arrives', async () => {
    setTauri(true)
    render(<TitleBar />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())
    win.isMaximized.mockImplementation(() => Promise.resolve(true))
    // The resize handler is never fired here.
    fireEvent.click(screen.getByRole('button', { name: 'Maximise window' }))
    expect(await screen.findByRole('button', { name: 'Restore window' })).toBeInTheDocument()
  })

  it('ignores a stale maximised answer that arrives after a newer one', async () => {
    setTauri(true)
    const first = deferred<boolean>()
    win.isMaximized.mockImplementationOnce(() => first.promise)
    render(<TitleBar />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())

    win.isMaximized.mockImplementation(() => Promise.resolve(true))
    const handler = win.onResized.mock.calls[0][0]
    await act(async () => {
      handler()
    })
    expect(await screen.findByRole('button', { name: 'Restore window' })).toBeInTheDocument()

    // The boot read was slow and answers last with the old state.
    await act(async () => {
      first.resolve(false)
    })
    expect(screen.getByRole('button', { name: 'Restore window' })).toBeInTheDocument()
  })

  it('survives Tauri internals without window metadata', async () => {
    setTauri(true)
    shim.broken = true
    expect(() => render(<TitleBar />)).not.toThrow()
    expect(() => fireEvent.click(screen.getByRole('button', { name: 'Close window' }))).not.toThrow()
    expect(win.close).not.toHaveBeenCalled()
  })

  it('swallows and logs a rejected window call instead of leaving an unhandled rejection', async () => {
    setTauri(true)
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const unhandled = vi.fn()
    process.on('unhandledRejection', unhandled)
    try {
      win.close.mockImplementation(() => Promise.reject(new Error('denied')))
      win.toggleMaximize.mockImplementation(() => Promise.reject(new Error('denied')))
      render(<TitleBar />)
      await waitFor(() => expect(win.onResized).toHaveBeenCalled())
      fireEvent.click(screen.getByRole('button', { name: 'Close window' }))
      fireEvent.click(screen.getByRole('button', { name: 'Maximise window' }))
      await waitFor(() => expect(warn).toHaveBeenCalledTimes(2))
      await new Promise((r) => setTimeout(r, 0))
      expect(unhandled).not.toHaveBeenCalled()
      // A failed toggle leaves the label alone.
      expect(screen.getByRole('button', { name: 'Maximise window' })).toBeInTheDocument()
    } finally {
      process.off('unhandledRejection', unhandled)
      warn.mockRestore()
    }
  })
})
