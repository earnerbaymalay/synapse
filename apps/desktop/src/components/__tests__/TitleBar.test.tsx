import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const win = vi.hoisted(() => ({
  minimize: vi.fn(() => Promise.resolve()),
  toggleMaximize: vi.fn(() => Promise.resolve()),
  close: vi.fn(() => Promise.resolve()),
  isMaximized: vi.fn(() => Promise.resolve(false)),
  onResized: vi.fn((_handler: () => void) => Promise.resolve(() => {})),
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => win,
}))

import { TitleBar } from '../TitleBar'

type TauriWindow = Window & { __TAURI_INTERNALS__?: unknown }

function setTauri(on: boolean) {
  if (on) (window as TauriWindow).__TAURI_INTERNALS__ = {}
  else delete (window as TauriWindow).__TAURI_INTERNALS__
}

beforeEach(() => {
  vi.clearAllMocks()
  win.isMaximized.mockImplementation(() => Promise.resolve(false))
})

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
})
