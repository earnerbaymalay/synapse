import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import * as THREE from 'three'
import { invoke } from '@tauri-apps/api/core'
import { HintsProvider } from '../../lib/hints'
import {
  MAX_DISTANCE,
  MAX_NODES,
  MIN_DISTANCE,
  MemoryGraph,
  errorText,
  formatTs,
  graphSummary,
  keepSelection,
  layoutPosition,
  normalizeGraph,
} from '../MemoryGraph'

/**
 * three.js cannot render in jsdom, so WebGLRenderer is swapped for a recorder.
 * Everything else in three (scene, geometry, maths) is the real library, which
 * lets the tests check that geometry and materials are actually disposed.
 */
const gl = vi.hoisted(() => ({
  available: true,
  throwOnCreate: false,
  renderers: [] as Array<{
    domElement: HTMLCanvasElement
    render: ReturnType<typeof vi.fn>
    dispose: ReturnType<typeof vi.fn>
    forceContextLoss: ReturnType<typeof vi.fn>
  }>,
}))

vi.mock('three', async (importOriginal) => {
  const actual = await importOriginal<typeof import('three')>()
  class FakeRenderer {
    domElement = document.createElement('canvas')
    render = vi.fn()
    dispose = vi.fn()
    forceContextLoss = vi.fn()
    setSize = vi.fn()
    setPixelRatio = vi.fn()
    setClearColor = vi.fn()
    constructor() {
      if (gl.throwOnCreate) throw new Error('Error creating WebGL context.')
      gl.renderers.push(this)
    }
  }
  return { ...actual, WebGLRenderer: FakeRenderer }
})

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const invokeMock = vi.mocked(invoke)

const payload = (over: Record<string, unknown> = {}) => ({
  nodes: [
    { id: 'brain-root', label: 'CANONICAL BRAIN', kind: 'core', narrative: 'root', files: [], ts: 0, val: 20 },
    { id: 'p-alpha', label: 'alpha', kind: 'project', narrative: 'alpha project', files: [], ts: 1729600000, val: 14 },
    { id: 'o-1', label: 'Fixed the parser', kind: 'change', narrative: 'parser fix', files: ['src/parse.rs'], ts: 1729600010, val: 8 },
  ],
  edges: [
    { source: 'brain-root', target: 'p-alpha', label: 'tracks' },
    { source: 'p-alpha', target: 'o-1', label: 'observed' },
  ],
  total_observations: 1,
  ...over,
})

let rafId = 0
let rafSpy: ReturnType<typeof vi.spyOn>
let cafSpy: ReturnType<typeof vi.spyOn>

beforeEach(() => {
  gl.available = true
  gl.throwOnCreate = false
  gl.renderers.length = 0
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  invokeMock.mockReset()
  invokeMock.mockResolvedValue(payload())
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(
    () => (gl.available ? ({} as unknown as RenderingContext) : null) as never,
  )
  // The loop is driven by hand: frames are requested but never run on their own.
  rafSpy = vi.spyOn(window, 'requestAnimationFrame').mockImplementation(() => ++rafId)
  cafSpy = vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {})
  try {
    localStorage.clear()
  } catch {
    /* ignore */
  }
})

afterEach(() => {
  vi.restoreAllMocks()
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
  delete (window as unknown as Record<string, unknown>).matchMedia
})

const readout = () => screen.getByTestId('memory-readout').textContent ?? ''
const dist = () => Number(/dist (\d+)/.exec(readout())?.[1])

async function renderReady() {
  const utils = render(<MemoryGraph />)
  await screen.findByTestId('memory-viewport')
  return utils
}

describe('normalizeGraph', () => {
  it('tolerates missing and malformed fields', () => {
    const g = normalizeGraph({
      nodes: [
        { id: 'a' },
        { id: 'a', label: 'duplicate' },
        { label: 'no id' },
        null,
        { id: 7, label: 'numeric id', files: ['x', 3], val: 'heavy' },
      ],
      edges: [{ source: 'a', target: '7' }, { source: 'a', target: 'ghost' }, { source: 'a' }],
    })
    expect(g.nodes.map((n) => n.id)).toEqual(['a', '7'])
    expect(g.nodes[0]).toMatchObject({ label: 'a', kind: 'unknown', narrative: '', files: [], ts: null, weight: null })
    expect(g.nodes[1].files).toEqual(['x'])
    expect(g.edges).toEqual([{ source: 'a', target: '7', label: '' }])
    expect(g.danglingEdges).toBe(2)
    expect(g.observations).toBeNull()
  })

  it('returns an empty graph for a payload that is not a graph', () => {
    expect(normalizeGraph(undefined).nodes).toEqual([])
    expect(normalizeGraph({ nodes: 'nope', edges: {} }).edges).toEqual([])
  })

  it('scales weights to 0..1 for drawing but keeps the raw weight', () => {
    const g = normalizeGraph(payload())
    expect(g.nodes[0]).toMatchObject({ weight: 20, size: 1 })
    expect(g.nodes[2].size).toBeCloseTo(0.4)
  })

  it('caps very large graphs and reports what was cut', () => {
    const nodes = Array.from({ length: MAX_NODES + 25 }, (_, i) => ({ id: `n${i}`, val: 1 }))
    const edges = [{ source: 'n0', target: `n${MAX_NODES + 1}` }]
    const g = normalizeGraph({ nodes, edges })
    expect(g.nodes).toHaveLength(MAX_NODES)
    expect(g.totalNodes).toBe(MAX_NODES + 25)
    expect(g.cappedEdges).toBe(1)
    expect(g.danglingEdges).toBe(0)
  })

  it('formats second and millisecond timestamps, and says when there is none', () => {
    expect(formatTs(1729600000)).toBe('2024-10-22 12:26 UTC')
    expect(formatTs(1729600000000)).toBe('2024-10-22 12:26 UTC')
    expect(formatTs(null)).toBe('not recorded')
    expect(formatTs(0)).toBe('not recorded')
  })
})

describe('MemoryGraph states', () => {
  it('outside Tauri it reports no backend and draws nothing', () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
    render(<MemoryGraph />)
    expect(screen.getByText('No backend attached.')).toBeInTheDocument()
    expect(invokeMock).not.toHaveBeenCalled()
    expect(screen.queryByTestId('memory-viewport')).not.toBeInTheDocument()
  })

  it('shows a backend error plainly instead of simulated data', async () => {
    invokeMock.mockRejectedValue('memory dir unreadable')
    render(<MemoryGraph />)
    expect(await screen.findByText('Memory graph could not be read.')).toBeInTheDocument()
    expect(screen.getByText(/memory dir unreadable/)).toBeInTheDocument()
    expect(screen.queryByTestId('memory-viewport')).not.toBeInTheDocument()
    expect(gl.renderers).toHaveLength(0)
  })

  it('reports an empty graph and builds no renderer', async () => {
    invokeMock.mockResolvedValue({ nodes: [], edges: [], total_observations: 0 })
    render(<MemoryGraph />)
    expect(await screen.findByText('No memory nodes recorded.')).toBeInTheDocument()
    expect(gl.renderers).toHaveLength(0)
  })

  it('draws a single node without error', async () => {
    invokeMock.mockResolvedValue({ nodes: [{ id: 'solo', label: 'solo' }], edges: [] })
    await renderReady()
    expect(gl.renderers).toHaveLength(1)
    expect(readout()).toMatch(/dist \d+/)
  })

  it('reports edges that point at missing nodes', async () => {
    invokeMock.mockResolvedValue(payload({ edges: [{ source: 'brain-root', target: 'ghost', label: 'x' }] }))
    await renderReady()
    expect(screen.getByText(/1 edge references a node missing/)).toBeInTheDocument()
  })

  it('falls back to a list with an explanation when WebGL is unavailable', async () => {
    gl.available = false
    render(<MemoryGraph />)
    expect(await screen.findByText('WebGL is not available')).toBeInTheDocument()
    const table = screen.getByRole('table', { name: 'Memory nodes' })
    expect(within(table).getByText('Fixed the parser')).toBeInTheDocument()
    expect(screen.queryByTestId('memory-viewport')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'graph view' })).toBeDisabled()
  })

  it('falls back when the renderer cannot be created', async () => {
    gl.throwOnCreate = true
    render(<MemoryGraph />)
    expect(await screen.findByText('WebGL is not available')).toBeInTheDocument()
    expect(screen.getByRole('table', { name: 'Memory nodes' })).toBeInTheDocument()
  })
})

describe('MemoryGraph controls', () => {
  it('exposes a focusable, labelled viewport', async () => {
    await renderReady()
    const vp = screen.getByRole('application')
    expect(vp).toHaveAttribute('tabindex', '0')
    expect(vp.getAttribute('aria-label')).toMatch(/Memory graph, 3 nodes/)
  })

  it('arrow keys orbit and +/- zoom within limits', async () => {
    const user = userEvent.setup()
    await renderReady()
    screen.getByRole('application').focus()

    const before = readout()
    await user.keyboard('{ArrowLeft}')
    expect(readout()).not.toBe(before)
    const afterOrbit = readout()
    await user.keyboard('{ArrowUp}')
    expect(readout()).not.toBe(afterOrbit)

    const d0 = dist()
    await user.keyboard('+')
    expect(dist()).toBeLessThan(d0)
    for (let i = 0; i < 30; i++) await user.keyboard('+')
    expect(dist()).toBe(MIN_DISTANCE)
    for (let i = 0; i < 40; i++) await user.keyboard('-')
    expect(dist()).toBe(MAX_DISTANCE)
  })

  it('reset view returns the camera home (instant under reduced motion)', async () => {
    window.matchMedia = vi.fn().mockReturnValue({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }) as unknown as typeof window.matchMedia
    const user = userEvent.setup()
    await renderReady()
    const home = readout()
    screen.getByRole('application').focus()
    await user.keyboard('{ArrowLeft}{ArrowLeft}+')
    expect(readout()).not.toBe(home)
    await user.click(screen.getByRole('button', { name: 'reset view' }))
    expect(readout()).toBe(home)

    screen.getByRole('application').focus()
    await user.keyboard('{ArrowRight}')
    await user.keyboard('0') // shortcut
    expect(readout()).toBe(home)
  })

  it('reduced motion turns auto-rotate off by default', async () => {
    window.matchMedia = vi.fn().mockReturnValue({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }) as unknown as typeof window.matchMedia
    await renderReady()
    expect(screen.getByRole('button', { name: 'auto-rotate off' })).toHaveAttribute('aria-pressed', 'false')
  })

  it('auto-rotate and sound buttons toggle and say their state', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'auto-rotate on' }))
    expect(screen.getByRole('button', { name: 'auto-rotate off' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'sound off' }))
    expect(screen.getByRole('button', { name: 'sound on' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('list view selects a node, Escape clears it', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'Fixed the parser' }))
    const panel = screen.getByRole('complementary', { name: 'Node Fixed the parser' })
    expect(within(panel).getByText('src/parse.rs')).toBeInTheDocument()
    expect(within(panel).getByText('8')).toBeInTheDocument() // raw weight, not a fake percentage
    // linked node is navigable from the panel
    await user.click(within(panel).getByRole('button', { name: 'alpha' }))
    expect(screen.getByRole('complementary', { name: 'Node alpha' })).toBeInTheDocument()

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument()
  })

  it('the L shortcut switches between graph and list', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.keyboard('l')
    expect(screen.getByRole('table', { name: 'Memory nodes' })).toBeInTheDocument()
    await user.keyboard('l')
    expect(await screen.findByTestId('memory-viewport')).toBeInTheDocument()
  })

  it('filter narrows the nodes and reports the match count', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.type(screen.getByRole('searchbox'), 'parse')
    expect(screen.getByText(/matching/)).toHaveTextContent('matching 1')
    await user.click(screen.getByRole('button', { name: 'list view' }))
    const rows = within(screen.getByRole('table')).getAllByRole('row')
    expect(rows).toHaveLength(2) // header + one match
  })

  it('shows the hints when hints are on', async () => {
    render(
      <HintsProvider>
        <MemoryGraph />
      </HintsProvider>,
    )
    await screen.findByTestId('memory-viewport')
    expect(document.querySelector('[data-hint="memory.orbit"]')).not.toBeNull()
    expect(document.querySelector('[data-hint="memory.inspect"]')).not.toBeNull()
  })
})

describe('MemoryGraph lifecycle', () => {
  it('cancels the frame loop and frees GPU resources on unmount', async () => {
    const geoDispose = vi.spyOn(THREE.BufferGeometry.prototype, 'dispose')
    const matDispose = vi.spyOn(THREE.Material.prototype, 'dispose')
    const { unmount } = await renderReady()
    const r = gl.renderers[0]
    expect(r.domElement.isConnected).toBe(true)
    const lastFrame = rafSpy.mock.results.at(-1)?.value

    unmount()

    expect(cafSpy).toHaveBeenCalledWith(lastFrame)
    expect(r.dispose).toHaveBeenCalledTimes(1)
    expect(r.forceContextLoss).toHaveBeenCalledTimes(1)
    expect(r.domElement.isConnected).toBe(false)
    expect(geoDispose).toHaveBeenCalled()
    expect(matDispose).toHaveBeenCalled()
  })

  it('switching to list view tears the scene down; switching back builds one fresh scene', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    expect(gl.renderers[0].dispose).toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'graph view' }))
    await screen.findByTestId('memory-viewport')
    expect(gl.renderers).toHaveLength(2)
    expect(gl.renderers[1].dispose).not.toHaveBeenCalled()
  })

  it('toggling auto-rotate or sound does not rebuild the scene', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'auto-rotate on' }))
    await user.click(screen.getByRole('button', { name: 'sound off' }))
    expect(gl.renderers).toHaveLength(1)
    expect(gl.renderers[0].dispose).not.toHaveBeenCalled()
  })

  it('removes its pointer and wheel listeners on unmount', async () => {
    const { unmount } = await renderReady()
    const vp = screen.getByTestId('memory-viewport')
    const removed = vi.spyOn(vp, 'removeEventListener')
    unmount()
    const types = removed.mock.calls.map((c) => c[0])
    expect(types).toEqual(expect.arrayContaining(['pointerdown', 'pointermove', 'pointerup', 'wheel']))
  })

  it('ignores a backend reply that lands after unmount', async () => {
    let resolve!: (v: unknown) => void
    invokeMock.mockReturnValue(new Promise((r) => (resolve = r)))
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { unmount } = render(<MemoryGraph />)
    unmount()
    resolve(payload())
    await waitFor(() => expect(gl.renderers).toHaveLength(0))
    expect(errors).not.toHaveBeenCalled()
  })
})

/** A promise the test settles by hand. */
function deferred<T = unknown>() {
  let resolve!: (v: T) => void
  let reject!: (e: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

const reducedMotion = () => {
  window.matchMedia = vi.fn().mockReturnValue({
    matches: true,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }) as unknown as typeof window.matchMedia
}

describe('pure helpers', () => {
  it('layoutPosition is finite for 0, 1, 2 and many nodes, and for bad input', () => {
    const finite = (p: number[]) => p.every(Number.isFinite)
    expect(layoutPosition(0, 0, false)).toEqual([0, 0, 0])
    expect(layoutPosition(0, 1, false)).toEqual([0, 0, 0])
    expect(finite(layoutPosition(0, 2, false))).toBe(true)
    expect(finite(layoutPosition(1, 2, false))).toBe(true)
    expect(layoutPosition(0, 2, false)).not.toEqual(layoutPosition(1, 2, false))
    for (let i = 0; i < MAX_NODES; i += 37) expect(finite(layoutPosition(i, MAX_NODES, true))).toBe(true)
    expect(finite(layoutPosition(NaN, 3, false))).toBe(true)
    expect(finite(layoutPosition(5, 3, false))).toBe(true) // index past the end is clamped
    expect(finite(layoutPosition(1, Infinity, false))).toBe(true)
  })

  it('keepSelection keeps an id that still exists and drops one that does not', () => {
    const g = normalizeGraph(payload())
    expect(keepSelection(g, 'o-1')).toBe('o-1')
    expect(keepSelection(g, 'gone')).toBeNull()
    expect(keepSelection(g, null)).toBeNull()
  })

  it('errorText passes the backend text through', () => {
    expect(errorText('memory dir unreadable')).toBe('memory dir unreadable')
    expect(errorText(new Error('boom'))).toBe('boom')
    expect(errorText({ message: 'shaped' })).toBe('shaped')
    expect(errorText({ code: 3 })).toBe('{"code":3}')
  })

  it('graphSummary describes counts, kinds and the filter', () => {
    const g = normalizeGraph(payload())
    expect(graphSummary(g, [['core', 1], ['project', 1]], null)).toBe('3 nodes and 2 edges. Kinds: core 1, project 1.')
    expect(graphSummary(g, [], 0)).toBe('3 nodes and 2 edges. 0 match the filter.')
  })
})

describe('MemoryGraph graph edge cases', () => {
  it('draws two nodes, a self-link and a duplicate id without NaN in the camera', async () => {
    invokeMock.mockResolvedValue({
      nodes: [{ id: 'a' }, { id: 'b' }, { id: 'a', label: 'dup' }],
      edges: [
        { source: 'a', target: 'a', label: 'self' },
        { source: 'a', target: 'b', label: 'x' },
      ],
    })
    await renderReady()
    expect(readout()).not.toMatch(/NaN|Infinity/)
    expect(screen.getByRole('application').getAttribute('aria-label')).toMatch(/2 nodes/)
  })

  it('a self-link in the panel keeps focus on the panel, not <body>', async () => {
    const user = userEvent.setup()
    invokeMock.mockResolvedValue({ nodes: [{ id: 'a', label: 'loop' }], edges: [{ source: 'a', target: 'a', label: 'self' }] })
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'loop' }))
    const panel = screen.getByRole('complementary', { name: 'Node loop' })
    await user.click(within(panel).getByRole('button', { name: 'loop' }))
    expect(within(panel).getByRole('heading', { name: 'loop' })).toHaveFocus()
  })
})

describe('MemoryGraph reading the backend', () => {
  it('calls get_memory_graph with no arguments (the Rust brain arg is optional)', async () => {
    await renderReady()
    expect(invokeMock).toHaveBeenCalledTimes(1)
    expect(invokeMock).toHaveBeenCalledWith('get_memory_graph')
  })

  it('error state offers a retry that reads again and then draws the real graph', async () => {
    const user = userEvent.setup()
    invokeMock.mockRejectedValueOnce('memory dir unreadable')
    render(<MemoryGraph />)
    const retry = await screen.findByRole('button', { name: 'try again' })
    const next = deferred()
    invokeMock.mockReturnValueOnce(next.promise as never)
    await user.click(retry)
    expect(screen.getByRole('button', { name: 'reading…' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'reading…' }))
    expect(invokeMock).toHaveBeenCalledTimes(2) // double press ignored
    await act(async () => next.resolve(payload()))
    expect(await screen.findByTestId('memory-viewport')).toBeInTheDocument()
    expect(screen.queryByText('Memory graph could not be read.')).not.toBeInTheDocument()
  })

  it('a failed retry shows the new backend text and stays usable', async () => {
    const user = userEvent.setup()
    invokeMock.mockRejectedValueOnce('first').mockRejectedValueOnce('second failure')
    render(<MemoryGraph />)
    await user.click(await screen.findByRole('button', { name: 'try again' }))
    expect(await screen.findByText(/second failure/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'try again' })).toBeEnabled()
  })

  it('empty brain shows no nodes and offers refresh, which then draws what was recorded', async () => {
    const user = userEvent.setup()
    invokeMock.mockResolvedValueOnce({ nodes: [], edges: [], total_observations: 0 })
    render(<MemoryGraph />)
    expect(await screen.findByText('No memory nodes recorded.')).toBeInTheDocument()
    expect(screen.queryByRole('table')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'refresh' }))
    expect(await screen.findByTestId('memory-viewport')).toBeInTheDocument()
  })

  it('refresh blocks double submit and keeps a selection whose node still exists', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'Fixed the parser' }))
    const next = deferred()
    invokeMock.mockReturnValueOnce(next.promise as never)
    await user.click(screen.getByRole('button', { name: 'refresh' }))
    const busy = screen.getByRole('button', { name: 'reading…' })
    expect(busy).toBeDisabled()
    expect(busy).toHaveAttribute('aria-busy', 'true')
    await user.click(busy)
    expect(invokeMock).toHaveBeenCalledTimes(2)
    // the old graph stays on screen while reading
    expect(screen.getByRole('complementary', { name: 'Node Fixed the parser' })).toBeInTheDocument()
    await act(async () =>
      next.resolve(
        payload({
          nodes: [...payload().nodes, { id: 'o-2', label: 'new obs', kind: 'change', narrative: '', files: [], ts: 1, val: 1 }],
        }),
      ),
    )
    expect(await screen.findByRole('button', { name: 'refresh' })).toBeEnabled()
    expect(screen.getByRole('complementary', { name: 'Node Fixed the parser' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'new obs' })).toBeInTheDocument()
  })

  it('refresh clears a selection whose node is gone', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'Fixed the parser' }))
    invokeMock.mockResolvedValueOnce(payload({ nodes: payload().nodes.slice(0, 2), edges: [] }))
    await user.click(screen.getByRole('button', { name: 'refresh' }))
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Fixed the parser' })).not.toBeInTheDocument())
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument()
    expect(screen.getByTestId('memory-announce')).toHaveTextContent('')
  })

  it('a failed refresh keeps the last graph and shows the backend error text', async () => {
    const user = userEvent.setup()
    await renderReady()
    invokeMock.mockRejectedValueOnce("Couldn't read the memory log in /x. (denied)")
    await user.click(screen.getByRole('button', { name: 'refresh' }))
    expect(await screen.findByRole('alert')).toHaveTextContent("Couldn't read the memory log in /x. (denied)")
    expect(screen.getByTestId('memory-viewport')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'refresh' })).toBeEnabled()
    // a later success clears the error
    await user.click(screen.getByRole('button', { name: 'refresh' }))
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument())
  })

  it('unmount during a refresh sets no state', async () => {
    const user = userEvent.setup()
    const { unmount } = await renderReady()
    const next = deferred()
    invokeMock.mockReturnValueOnce(next.promise as never)
    await user.click(screen.getByRole('button', { name: 'refresh' }))
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {})
    unmount()
    await act(async () => next.resolve(payload()))
    expect(errors).not.toHaveBeenCalled()
    expect(gl.renderers).toHaveLength(1)
  })
})

describe('MemoryGraph controls, second wave', () => {
  it('auto-rotate cannot claim to be on under reduced motion', async () => {
    reducedMotion()
    const user = userEvent.setup()
    await renderReady()
    const btn = screen.getByRole('button', { name: 'auto-rotate off' })
    expect(btn).toBeDisabled()
    await user.click(btn)
    expect(screen.queryByRole('button', { name: 'auto-rotate on' })).not.toBeInTheDocument()
  })

  it('Ctrl/Cmd/Alt + and - are left to the browser', async () => {
    await renderReady()
    const vp = screen.getByRole('application')
    const before = readout()
    const ev = new KeyboardEvent('keydown', { key: '=', ctrlKey: true, bubbles: true, cancelable: true })
    act(() => {
      vp.dispatchEvent(ev)
    })
    expect(ev.defaultPrevented).toBe(false)
    expect(readout()).toBe(before)
  })

  it('wheel zooms within limits and blocks page scroll', async () => {
    await renderReady()
    const vp = screen.getByRole('application')
    const d0 = dist()
    expect(fireEvent.wheel(vp, { deltaY: -100 })).toBe(false) // preventDefault called
    expect(dist()).toBeLessThan(d0)
    for (let i = 0; i < 40; i++) fireEvent.wheel(vp, { deltaY: 1000 })
    expect(dist()).toBe(MAX_DISTANCE)
  })

  it('lost graphics context falls back to the list and rebuild view builds a fresh scene', async () => {
    const user = userEvent.setup()
    await renderReady()
    act(() => {
      gl.renderers[0].domElement.dispatchEvent(new Event('webglcontextlost', { cancelable: true }))
    })
    expect(await screen.findByText('The graphics context was lost')).toBeInTheDocument()
    expect(screen.getByRole('table', { name: 'Memory nodes' })).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'rebuild view' }))
    expect(await screen.findByTestId('memory-viewport')).toBeInTheDocument()
    expect(gl.renderers).toHaveLength(2)
  })

  it('the viewport is described by a text summary of the graph', async () => {
    await renderReady()
    const vp = screen.getByRole('application')
    const id = vp.getAttribute('aria-describedby')
    expect(id).toBeTruthy()
    expect(document.getElementById(id!)).toHaveTextContent('3 nodes and 2 edges. Kinds:')
  })

  it('selection is announced in a polite live region', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'alpha' }))
    expect(screen.getByTestId('memory-announce')).toHaveTextContent('Selected alpha, project, 2 links.')
    expect(screen.getByTestId('memory-announce')).toHaveAttribute('aria-live', 'polite')
  })

  it('following a link in the panel moves focus to the new node heading', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'Fixed the parser' }))
    await user.click(within(screen.getByRole('complementary')).getByRole('button', { name: 'alpha' }))
    expect(screen.getByRole('heading', { name: 'alpha' })).toHaveFocus()
  })

  it('closing the panel returns focus to the node row instead of <body>', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'Fixed the parser' }))
    await user.click(screen.getByRole('button', { name: 'Close details for Fixed the parser' }))
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Fixed the parser' })).toHaveFocus()
  })

  it('in graph view, closing the panel returns focus to the viewport', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    await user.click(screen.getByRole('button', { name: 'alpha' }))
    await user.click(screen.getByRole('button', { name: 'graph view' }))
    await screen.findByTestId('memory-viewport')
    within(screen.getByRole('complementary')).getByRole('button', { name: /Close details/ }).focus()
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument()
    expect(screen.getByRole('application')).toHaveFocus()
  })

  it('long labels wrap instead of overflowing the list and panel', async () => {
    const user = userEvent.setup()
    const long = 'src/' + 'very_long_path_segment/'.repeat(12) + 'file.rs'
    invokeMock.mockResolvedValue({ nodes: [{ id: 'x', label: long, kind: 'file' }], edges: [] })
    await renderReady()
    await user.click(screen.getByRole('button', { name: 'list view' }))
    const row = screen.getByRole('button', { name: long })
    expect(row.className).toMatch(/overflow-wrap:anywhere/)
    await user.click(row)
    expect(screen.getByRole('heading', { name: long }).className).toMatch(/overflow-wrap:anywhere/)
  })

  it('typing L or 0 in the filter box does not switch view or reset', async () => {
    const user = userEvent.setup()
    await renderReady()
    await user.type(screen.getByRole('searchbox'), 'l0')
    expect(screen.getByTestId('memory-viewport')).toBeInTheDocument()
    expect(screen.getByRole('searchbox')).toHaveValue('l0')
  })
})
