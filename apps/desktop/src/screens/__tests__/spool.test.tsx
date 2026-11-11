import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }))

import { MAX_LINES, Spool, estimateTokens, tailLines, type SpoolEntryDto } from '../Spool'

const entry = (over: Partial<SpoolEntryDto> = {}): SpoolEntryDto => ({
  id: 'a1b2c3d4',
  timestamp: 1_760_000_000,
  raw_bytes: 2048,
  log_path: '/home/mara/.synapse/spool/a1b2c3d4.log',
  command: 'cargo test --workspace',
  ...over,
})

type Handlers = Record<string, (args?: Record<string, unknown>) => unknown>

function backend(h: Handlers) {
  invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    const fn = h[cmd]
    if (!fn) throw `command ${cmd} not found`
    return fn(args)
  })
}

function attachTauri() {
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
}

let writeText: ReturnType<typeof vi.fn>

beforeEach(() => {
  invokeMock.mockReset()
  attachTauri()
  writeText = vi.fn().mockResolvedValue(undefined)
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
})

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})

describe('Spool helpers', () => {
  it('estimates tokens as characters / 4 rounded up, like core', () => {
    expect(estimateTokens('')).toBe(0)
    expect(estimateTokens('abcd')).toBe(1)
    expect(estimateTokens('abcde')).toBe(2)
  })

  it('tails long text and reports both counts', () => {
    const text = Array.from({ length: 10 }, (_, i) => `l${i}`).join('\n')
    expect(tailLines(text, 3)).toEqual({ shown: 'l7\nl8\nl9', shownCount: 3, total: 10 })
    expect(tailLines('a\nb', 3).shownCount).toBe(2)
  })
})

describe('Spool states', () => {
  it('outside Tauri reports no backend and invokes nothing', () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
    render(<Spool />)
    expect(screen.getByText(/no backend attached/i)).toBeInTheDocument()
    expect(screen.getByText('synapse spool list')).toBeInTheDocument()
    expect(invokeMock).not.toHaveBeenCalled()
  })

  it('shows no demo data: an unwired backend is reported as absent', async () => {
    backend({})
    const { container } = render(<Spool />)
    expect(await screen.findByText(/spool not readable from the desktop yet/i)).toBeInTheDocument()
    expect(container.textContent).not.toMatch(/8420|97\.1|spool-9481|cargo test/)
  })

  it('empty spool says so and names the next command', async () => {
    backend({ spool_list: () => [] })
    render(<Spool />)
    expect(await screen.findByText(/spool is empty/i)).toBeInTheDocument()
    expect(screen.getByText('synapse squash <command>')).toBeInTheDocument()
  })

  it('list error shows the error and retry re-reads', async () => {
    const user = userEvent.setup()
    let fail = true
    backend({
      spool_list: () => {
        if (fail) throw 'permission denied: ~/.synapse/spool'
        return []
      },
    })
    render(<Spool />)
    expect(await screen.findByRole('alert')).toHaveTextContent('permission denied: ~/.synapse/spool')
    fail = false
    await user.click(screen.getByRole('button', { name: /re-read the spool/i }))
    expect(await screen.findByText(/spool is empty/i)).toBeInTheDocument()
  })
})

describe('Spool inspector', () => {
  const two = [entry(), entry({ id: 'ffff0000', command: null, raw_bytes: 10, log_path: '/s/ffff0000.log' })]
  const logs: Record<string, string> = { a1b2c3d4: 'abcdefgh\nok', ffff0000: '' }

  it('selects the newest run, computes tokens from the log and switches runs', async () => {
    const user = userEvent.setup()
    backend({ spool_list: () => two, spool_show: (a) => logs[a!.id as string] })
    render(<Spool />)
    const inspector = await screen.findByRole('region', { name: /spool inspector/i })
    // 11 chars -> 3 tokens, measured, not hard-coded
    await waitFor(() => expect(within(inspector).getAllByText(/≈ 3 tokens/).length).toBeGreaterThan(0))
    expect(within(inspector).getByText('/home/mara/.synapse/spool/a1b2c3d4.log')).toBeInTheDocument()

    // second run has no command: falls back to its id, and its empty log is reported
    await user.click(within(within(screen.getByRole('navigation', { name: /spooled runs/i })).getByRole('list')).getAllByRole('button')[1])
    expect(await screen.findByText(/log is empty/i)).toBeInTheDocument()
    expect(invokeMock).toHaveBeenCalledWith('spool_show', { id: 'ffff0000' })
  })

  it('view toggles show the right panes and report aria-pressed', async () => {
    const user = userEvent.setup()
    backend({ spool_list: () => two, spool_show: (a) => logs[a!.id as string] })
    render(<Spool />)
    await screen.findByLabelText('Raw output', { selector: 'pre' })
    expect(screen.getByText(/gated output/i, { selector: 'p' })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Raw' }))
    expect(screen.getByRole('button', { name: 'Raw' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.queryByText(/not recorded/i)).toBeNull()
    expect(screen.getByLabelText('Raw output', { selector: 'pre' })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Gated' }))
    expect(screen.getByText(/not recorded/i)).toBeInTheDocument()
    expect(screen.queryByLabelText('Raw output', { selector: 'pre' })).toBeNull()

    await user.click(screen.getByRole('button', { name: 'Side by side' }))
    expect(screen.getByText(/not recorded/i)).toBeInTheDocument()
    expect(screen.getByLabelText('Raw output', { selector: 'pre' })).toBeInTheDocument()
  })

  it('the v shortcut cycles the view while the screen is mounted', async () => {
    backend({ spool_list: () => two, spool_show: (a) => logs[a!.id as string] })
    const { unmount } = render(<Spool />)
    await screen.findByLabelText('Raw output', { selector: 'pre' })
    fireEvent.keyDown(window, { key: 'v' })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Raw' })).toHaveAttribute('aria-pressed', 'true'))
    unmount()
  })

  it('caps a very large log with a "showing N of M" note', async () => {
    const big = Array.from({ length: MAX_LINES + 500 }, (_, i) => `line ${i}`).join('\n')
    backend({ spool_list: () => [entry()], spool_show: () => big })
    render(<Spool />)
    expect(
      await screen.findByText(`showing last ${MAX_LINES.toLocaleString('en-US')} of ${(MAX_LINES + 500).toLocaleString('en-US')} lines`, {
        exact: false,
      }),
    ).toBeInTheDocument()
    const pre = screen.getByLabelText('Raw output', { selector: 'pre' })
    expect(pre.textContent).toContain(`line ${MAX_LINES + 499}`)
    expect(pre.textContent).not.toContain('line 0\n')
  })

  it('log read error is shown in the raw pane', async () => {
    backend({
      spool_list: () => [entry()],
      spool_show: () => {
        throw "Spool log 'a1b2c3d4' not found"
      },
    })
    render(<Spool />)
    expect(await screen.findByRole('alert')).toHaveTextContent("Spool log 'a1b2c3d4' not found")
  })

  it('copy buttons copy path, full log and command', async () => {
    const user = userEvent.setup()
    const big = Array.from({ length: MAX_LINES + 5 }, (_, i) => `l${i}`).join('\n')
    backend({ spool_list: () => [entry()], spool_show: () => big })
    render(<Spool />)
    await screen.findByLabelText('Raw output', { selector: 'pre' })
    // user-event installs its own clipboard; re-pin ours after setup
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })

    await user.click(screen.getByRole('button', { name: 'Copy path' }))
    expect(writeText).toHaveBeenLastCalledWith('/home/mara/.synapse/spool/a1b2c3d4.log')
    expect(await screen.findByText('Copied log path.')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Copy full log' }))
    expect(writeText).toHaveBeenLastCalledWith(big) // the full log, not the capped view

    await user.click(screen.getByRole('button', { name: /copy synapse spool show/i }))
    expect(writeText).toHaveBeenLastCalledWith('synapse spool show a1b2c3d4')
  })

  it('clipboard failure is reported, not swallowed', async () => {
    const user = userEvent.setup()
    backend({ spool_list: () => [entry()], spool_show: () => 'x' })
    render(<Spool />)
    await screen.findByLabelText('Raw output', { selector: 'pre' })
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn().mockRejectedValue(new Error('Document is not focused')) },
      configurable: true,
    })
    await user.click(screen.getByRole('button', { name: 'Copy path' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Copy failed: Document is not focused')

    Object.defineProperty(navigator, 'clipboard', { value: undefined, configurable: true })
    await user.click(screen.getByRole('button', { name: 'Copy path' }))
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/clipboard is not available/))
  })

  it('caps a very long run list with a note', async () => {
    const many = Array.from({ length: 205 }, (_, i) => entry({ id: `id${i}`, log_path: `/s/id${i}.log` }))
    backend({ spool_list: () => many, spool_show: () => '' })
    render(<Spool />)
    expect(await screen.findByText(/showing 200 of 205 runs/)).toBeInTheDocument()
    expect(within(within(screen.getByRole('navigation', { name: /spooled runs/i })).getByRole('list')).getAllByRole('button')).toHaveLength(200)
  })
})
