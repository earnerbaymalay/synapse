import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

import { invoke } from '@tauri-apps/api/core'
import { Cockpit, POLL_MS, beatAt, fmtPct, normalize } from '../Cockpit'
import { HintsProvider } from '../../lib/hints'
import type { CockpitTelemetry } from '../types'

const mockInvoke = vi.mocked(invoke)

const telemetry = (over: Partial<CockpitTelemetry> = {}): CockpitTelemetry => ({
  gateway_status: 'ONLINE',
  upstream_url: 'http://127.0.0.1:11434',
  model: 'qwen2.5-coder:7b',
  latency_ms: 12,
  requests_handled: 1800,
  raw_prompt_tokens: 184600,
  gated_prompt_tokens: 11200,
  tokens_saved: 173400,
  reduction_pct: 93.93,
  brain_path: '/home/mara/AIBrain',
  adapters_present: 14,
  adapters_total: 15,
  doctor_criticals: 0,
  doctor_fixable: 0,
  ...over,
})

/** Let pending invoke promises settle and React commit. */
const flush = () => act(async () => {
  await vi.advanceTimersByTimeAsync(0)
})
const advance = (ms: number) => act(async () => {
  await vi.advanceTimersByTimeAsync(ms)
})

let hidden = false
let reducedMotion = false
const ctxStub = new Proxy({}, { get: () => () => undefined, set: () => true }) as CanvasRenderingContext2D

beforeEach(() => {
  vi.useFakeTimers()
  // Testing Library's asyncWrapper (used by user-event) awaits a setTimeout(0)
  // and only advances fake timers itself when it sees a `jest` global. Under
  // vitest that global is absent, so the faked timeout never fires and every
  // user-event call hangs. This shim is the documented workaround.
  ;(globalThis as unknown as Record<string, unknown>).jest = {
    advanceTimersByTime: (ms: number) => vi.advanceTimersByTime(ms),
  }
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  mockInvoke.mockReset()
  mockInvoke.mockResolvedValue(telemetry())
  hidden = false
  reducedMotion = false
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => hidden })
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(
    () => ctxStub as unknown as ReturnType<HTMLCanvasElement['getContext']>,
  )
  window.matchMedia = vi.fn().mockImplementation((q: string) => ({
    matches: q.includes('reduce') ? reducedMotion : false,
    media: q,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })) as unknown as typeof window.matchMedia
  localStorage.clear()
})

afterEach(() => {
  delete (globalThis as unknown as Record<string, unknown>).jest
  vi.clearAllTimers()
  vi.useRealTimers()
  vi.restoreAllMocks()
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})

const user = () => userEvent.setup({ advanceTimers: vi.advanceTimersByTime })

describe('Cockpit data path', () => {
  it('does not present the hard-coded gateway numbers as measured (default build)', async () => {
    render(<Cockpit />)
    await flush()
    expect(screen.getAllByText(/Not measured in this build/)).toHaveLength(2)
    expect(screen.queryByText(/184,600|11,200|1,800|93\.9%|12 ms|qwen|11434/)).not.toBeInTheDocument()
    // Brain fields are measured and shown
    expect(screen.getByText('14/15 grafts')).toBeInTheDocument()
    expect(screen.getByText('/home/mara/AIBrain')).toBeInTheDocument()
    expect(screen.getByText('Brain steady. Gateway not measured.')).toBeInTheDocument()
    expect(screen.getByRole('img')).toHaveAttribute('data-mode', 'beat')
  })

  it('reads telemetry on mount and formats numbers', async () => {
    render(<Cockpit gatewayMeasured />)
    expect(screen.getByText('Reading vital signs…')).toBeInTheDocument()
    await flush()
    expect(mockInvoke).toHaveBeenCalledWith('get_cockpit_telemetry', { brain: null, tools: null })
    expect(screen.getByText('184,600')).toBeInTheDocument()
    expect(screen.getByText('11,200')).toBeInTheDocument()
    expect(screen.getByText('1,800')).toBeInTheDocument()
    expect(screen.getByText('93.9%')).toBeInTheDocument()
    expect(screen.getByText('12 ms')).toBeInTheDocument()
    expect(screen.getByText('14/15 grafts')).toBeInTheDocument()
    expect(screen.getByText('Live')).toHaveClass('text-success')
  })

  it('polls on the interval', async () => {
    render(<Cockpit />)
    await flush()
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    await advance(POLL_MS)
    expect(mockInvoke).toHaveBeenCalledTimes(2)
    await advance(POLL_MS)
    expect(mockInvoke).toHaveBeenCalledTimes(3)
  })

  it('does not overlap reads when one is slow', async () => {
    let resolve!: (v: CockpitTelemetry) => void
    mockInvoke.mockImplementationOnce(() => new Promise((r) => (resolve = r as typeof resolve)))
    render(<Cockpit />)
    await advance(POLL_MS * 3)
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    await act(async () => resolve(telemetry()))
    await advance(POLL_MS)
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })

  it('leaves no timers or animation frames behind on unmount', async () => {
    const { unmount } = render(<Cockpit />)
    await flush()
    await advance(POLL_MS)
    expect(vi.getTimerCount()).toBeGreaterThan(0)
    unmount()
    expect(vi.getTimerCount()).toBe(0)
    await advance(POLL_MS * 4)
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })

  it('does not set state after unmount when a read resolves late', async () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {})
    let resolve!: (v: CockpitTelemetry) => void
    mockInvoke.mockImplementationOnce(() => new Promise((r) => (resolve = r as typeof resolve)))
    const { unmount } = render(<Cockpit />)
    unmount()
    await act(async () => resolve(telemetry()))
    await flush()
    expect(err).not.toHaveBeenCalled()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('reports missing fields as not reported, and keeps zero as zero', async () => {
    mockInvoke.mockResolvedValue({ gateway_status: 'ONLINE', doctor_criticals: 0, latency_ms: 0 })
    render(<Cockpit gatewayMeasured />)
    await flush()
    expect(screen.getAllByText('not reported').length).toBeGreaterThanOrEqual(8)
    expect(screen.getByText('grafts not reported')).toBeInTheDocument()
    expect(screen.getByText('0 ms')).toBeInTheDocument()
    expect(screen.getByText('0')).toBeInTheDocument()
  })

  it('survives a null or garbage payload', async () => {
    mockInvoke.mockResolvedValue(null)
    render(<Cockpit gatewayMeasured />)
    await flush()
    expect(screen.getByText('Gateway status not reported.')).toBeInTheDocument()
    expect(screen.getByText('Flat trace: gateway not online')).toBeInTheDocument()
    expect(screen.getByRole('img')).toHaveAttribute('data-mode', 'flat')
  })

  it('reports a failed read, keeps the last good reading, and retries', async () => {
    render(<Cockpit gatewayMeasured />)
    await flush()
    mockInvoke.mockRejectedValueOnce('brain root not found')
    await advance(POLL_MS)
    expect(screen.getByRole('alert')).toHaveTextContent('Telemetry read failed: brain root not found')
    expect(screen.getByText('Signal lost')).toBeInTheDocument()
    expect(screen.getByText('184,600')).toBeInTheDocument()
    expect(screen.getByRole('img')).toHaveAttribute('data-mode', 'flat')
    // retry reads immediately and recovers
    await act(async () => screen.getByRole('button', { name: 'retry' }).click())
    await flush()
    expect(mockInvoke).toHaveBeenCalledTimes(3)
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('shows an error with no fake data when the first read fails', async () => {
    mockInvoke.mockRejectedValue(new Error('boom'))
    render(<Cockpit />)
    await flush()
    expect(screen.getByRole('alert')).toHaveTextContent('boom')
    expect(screen.getByText('No reading yet.')).toBeInTheDocument()
    expect(screen.queryByText('Runtime gateway')).not.toBeInTheDocument()
  })

  it('outside Tauri reports absence and never invokes or polls', async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
    render(<Cockpit />)
    await advance(POLL_MS * 3)
    expect(mockInvoke).not.toHaveBeenCalled()
    expect(screen.getByText(/No telemetry/)).toBeInTheDocument()
    expect(screen.queryByText(/184,600|qwen/)).not.toBeInTheDocument()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('skips reads while the window is hidden and reads once when shown', async () => {
    render(<Cockpit />)
    await flush()
    hidden = true
    act(() => document.dispatchEvent(new Event('visibilitychange')))
    await advance(POLL_MS * 4)
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    hidden = false
    act(() => document.dispatchEvent(new Event('visibilitychange')))
    await flush()
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })

  it('names the next command from the reading', async () => {
    mockInvoke.mockResolvedValue(telemetry({ doctor_criticals: 2, doctor_fixable: 1 }))
    render(<Cockpit />)
    await flush()
    expect(screen.getByText('2 critical findings need a human.')).toBeInTheDocument()
    expect(screen.getByText('synapse doctor --fix')).toBeInTheDocument()
    expect(screen.getByText('2')).toHaveClass('text-alarm')
  })
})

describe('Cockpit controls', () => {
  it('pause stops polling and resume restarts it', async () => {
    const u = user()
    render(<Cockpit />)
    await flush()
    await u.click(screen.getByRole('button', { name: /pause/ }))
    expect(screen.getByText(/paused/)).toBeInTheDocument()
    expect(screen.getByRole('img')).toHaveAttribute('data-mode', 'frozen')
    await advance(POLL_MS * 3)
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    await u.click(screen.getByRole('button', { name: /resume/ }))
    await advance(POLL_MS)
    expect(mockInvoke).toHaveBeenCalledTimes(2)
    expect(screen.getByRole('img')).toHaveAttribute('data-mode', 'beat')
  })

  it('P toggles live updates', async () => {
    const u = user()
    render(<Cockpit />)
    await flush()
    await u.keyboard('p')
    expect(screen.getByRole('button', { name: /resume/ })).toHaveAttribute('aria-pressed', 'true')
    await u.keyboard('p')
    expect(screen.getByRole('button', { name: /pause/ })).toHaveAttribute('aria-pressed', 'false')
  })

  it('P is unregistered when the screen unmounts', async () => {
    const { unmount } = render(<Cockpit />)
    await flush()
    unmount()
    const { listShortcuts } = await import('../../lib/shortcuts')
    expect(listShortcuts().some((s) => s.id === 'cockpit.toggle-live')).toBe(false)
  })

  it('read now reads immediately and works while paused', async () => {
    const u = user()
    render(<Cockpit />)
    await flush()
    await u.click(screen.getByRole('button', { name: /pause/ }))
    await u.click(screen.getByRole('button', { name: 'read now' }))
    await flush()
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })
})

describe('Cockpit trace', () => {
  // Counted via a spy rather than vi.getTimerCount(): React's scheduler also
  // queues (faked) setImmediate tasks, so the raw timer count is not stable.
  it('animates only while live, and not under reduced motion', async () => {
    const raf = vi.spyOn(window, 'requestAnimationFrame')
    const { unmount } = render(<Cockpit />)
    await flush()
    await advance(100)
    expect(raf.mock.calls.length).toBeGreaterThan(1)
    unmount()

    raf.mockClear()
    reducedMotion = true
    render(<Cockpit />)
    await flush()
    await advance(100)
    expect(raf).not.toHaveBeenCalled()
    expect(screen.getByRole('img')).toHaveAttribute('data-mode', 'beat')
  })

  it('does not animate while paused', async () => {
    const raf = vi.spyOn(window, 'requestAnimationFrame')
    render(<Cockpit />)
    await flush()
    await act(async () => screen.getByRole('button', { name: /pause/ }).click())
    raf.mockClear()
    await advance(200)
    expect(raf).not.toHaveBeenCalled()
  })

  it('stops animating while the window is hidden and resumes when shown', async () => {
    const raf = vi.spyOn(window, 'requestAnimationFrame')
    const caf = vi.spyOn(window, 'cancelAnimationFrame')
    render(<Cockpit />)
    await flush()
    hidden = true
    act(() => document.dispatchEvent(new Event('visibilitychange')))
    expect(caf).toHaveBeenCalled()
    raf.mockClear()
    await advance(200)
    expect(raf).not.toHaveBeenCalled()
    hidden = false
    act(() => document.dispatchEvent(new Event('visibilitychange')))
    await advance(50)
    expect(raf).toHaveBeenCalled()
  })

  it('beat shape repeats and is flat between complexes', () => {
    expect(beatAt(90)).toBe(45)
    expect(beatAt(290)).toBe(45)
    expect(beatAt(-110)).toBe(45)
    expect(beatAt(20)).toBe(0)
  })
})

describe('Cockpit helpers', () => {
  it('normalize drops non-finite and wrong-typed values', () => {
    const r = normalize({ latency_ms: NaN, model: '', requests_handled: '5', reduction_pct: 0 })
    expect(r.latency_ms).toBeNull()
    expect(r.model).toBeNull()
    expect(r.requests_handled).toBeNull()
    expect(r.reduction_pct).toBe(0)
  })

  it('fmtPct keeps one decimal', () => {
    expect(fmtPct(93.94)).toBe('93.9%')
    expect(fmtPct(0)).toBe('0.0%')
    expect(fmtPct(null)).toBeNull()
  })
})

describe('Cockpit hints', () => {
  const withHints = (ui: ReactNode) => <HintsProvider>{ui}</HintsProvider>

  it('explains the gateway and gating numbers and what to run next', async () => {
    render(withHints(<Cockpit />))
    await flush()
    const ids = Array.from(document.querySelectorAll('[data-hint]')).map((n) => n.getAttribute('data-hint'))
    expect(ids).toEqual(['cockpit.gateway', 'cockpit.gating', 'cockpit.next'])
    for (const n of document.querySelectorAll('[data-hint]')) expect(n.textContent).not.toMatch(/!/)
  })
})
