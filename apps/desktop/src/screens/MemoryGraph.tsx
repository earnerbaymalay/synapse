import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode, type RefObject } from 'react'
import { invoke } from '@tauri-apps/api/core'
import * as THREE from 'three'
import { Hint } from '../lib/hints'
import { useShortcut } from '../lib/shortcuts'
import { cn } from '../lib/utils'

/**
 * Memory: the brain's memory store drawn as a 3D node graph, with a plain
 * list view beside it. Everything on screen comes from `get_memory_graph`;
 * when that call fails, or there is no WebGL, the screen says so instead of
 * drawing something in its place (IDENTITY.md rules 2 and 3).
 */

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

/** Nodes drawn at most. Beyond this the graph is cut and the cut is reported. */
export const MAX_NODES = 500
/** Above this many nodes the scene drops to cheaper geometry. */
export const LOD_THRESHOLD = 150
export const MIN_DISTANCE = 40
export const MAX_DISTANCE = 700
const HOME = new THREE.Vector3(0, 45, 290)
const FOCUS_DISTANCE = 70
const ORBIT_STEP = 0.12 // radians per arrow key press
const ZOOM_STEP = 0.85 // distance factor per +/- press
const DRAG_SPEED = 0.005
const CLICK_SLOP = 4 // px of pointer travel still treated as a click

// ---------------------------------------------------------------------------
// Payload normalisation (pure, exported for tests)
// ---------------------------------------------------------------------------

export interface GraphNode {
  id: string
  label: string
  kind: string
  narrative: string
  files: string[]
  /** null when the payload did not carry a usable timestamp */
  ts: number | null
  /** raw weight from the backend; null when not measured */
  weight: number | null
  /** weight scaled to 0..1 against the heaviest node, for drawing only */
  size: number
}

export interface GraphEdge {
  source: string
  target: string
  label: string
}

export interface NormalizedGraph {
  nodes: GraphNode[]
  edges: GraphEdge[]
  /** valid nodes in the payload, before the MAX_NODES cap */
  totalNodes: number
  /** edges dropped because an endpoint is missing from the payload */
  danglingEdges: number
  /** edges dropped only because an endpoint fell outside the node cap */
  cappedEdges: number
  observations: number | null
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const str = (v: unknown): string | null =>
  typeof v === 'string' ? v : typeof v === 'number' && Number.isFinite(v) ? String(v) : null
const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null)

export function normalizeGraph(raw: unknown): NormalizedGraph {
  const payload = isObj(raw) ? raw : {}
  const rawNodes = Array.isArray(payload.nodes) ? payload.nodes : []
  const rawEdges = Array.isArray(payload.edges) ? payload.edges : []

  const seen = new Set<string>()
  const valid: Omit<GraphNode, 'size'>[] = []
  for (const n of rawNodes) {
    if (!isObj(n)) continue
    const id = str(n.id)
    if (!id || seen.has(id)) continue
    seen.add(id)
    valid.push({
      id,
      label: str(n.label)?.trim() || id,
      kind: str(n.kind)?.trim() || 'unknown',
      narrative: str(n.narrative) ?? '',
      files: Array.isArray(n.files) ? n.files.filter((f): f is string => typeof f === 'string') : [],
      ts: num(n.ts),
      weight: num(n.val),
    })
  }

  const kept = valid.slice(0, MAX_NODES)
  const maxWeight = kept.reduce((m, n) => (n.weight !== null && n.weight > m ? n.weight : m), 0)
  const nodes: GraphNode[] = kept.map((n) => ({
    ...n,
    size: n.weight !== null && maxWeight > 0 ? Math.max(0, Math.min(1, n.weight / maxWeight)) : 0.5,
  }))

  const keptIds = new Set(nodes.map((n) => n.id))
  const edges: GraphEdge[] = []
  let danglingEdges = 0
  let cappedEdges = 0
  for (const e of rawEdges) {
    if (!isObj(e)) continue
    const source = str(e.source)
    const target = str(e.target)
    if (!source || !target) {
      danglingEdges++
      continue
    }
    if (!seen.has(source) || !seen.has(target)) {
      danglingEdges++
      continue
    }
    if (!keptIds.has(source) || !keptIds.has(target)) {
      cappedEdges++
      continue
    }
    edges.push({ source, target, label: str(e.label) ?? '' })
  }

  return {
    nodes,
    edges,
    totalNodes: valid.length,
    danglingEdges,
    cappedEdges,
    observations: num(payload.total_observations),
  }
}

export function nodeMatches(node: GraphNode, query: string): boolean {
  const q = query.toLowerCase().trim()
  if (!q) return true
  return (
    node.label.toLowerCase().includes(q) ||
    node.kind.toLowerCase().includes(q) ||
    node.narrative.toLowerCase().includes(q) ||
    node.files.some((f) => f.toLowerCase().includes(q))
  )
}

/** Accepts seconds or milliseconds since the epoch. */
export function formatTs(ts: number | null): string {
  if (ts === null || ts <= 0) return 'not recorded'
  const ms = ts < 1e12 ? ts * 1000 : ts
  const d = new Date(ms)
  return Number.isNaN(d.getTime()) ? 'not recorded' : d.toISOString().replace('T', ' ').slice(0, 16) + ' UTC'
}

/**
 * Where node `idx` of `count` sits: an even Fibonacci-sphere spread. Always
 * finite: 0 or 1 nodes sit at the origin, and a bad index or count never
 * yields NaN or Infinity.
 */
export function layoutPosition(idx: number, count: number, lod: boolean): [number, number, number] {
  if (!Number.isFinite(idx) || !Number.isFinite(count) || count <= 1 || idx < 0) return [0, 0, 0]
  const i = Math.min(Math.floor(idx), count - 1)
  const phi = Math.acos(Math.max(-1, Math.min(1, 1 - (2 * (i + 0.5)) / count)))
  const theta = Math.PI * (1 + Math.sqrt(5)) * i
  const dist = 80 + (i % 3) * 20 + (lod ? Math.sqrt(count) * 4 : 0)
  return [dist * Math.cos(theta) * Math.sin(phi), dist * Math.cos(phi) * 0.7, dist * Math.sin(theta) * Math.sin(phi)]
}

/** The selection to keep after a re-read: the same id if that node still exists, else none. */
export function keepSelection(graph: NormalizedGraph, id: string | null): string | null {
  return id !== null && graph.nodes.some((n) => n.id === id) ? id : null
}

/** The backend's own error text: Tauri commands reject with a string. */
export function errorText(err: unknown): string {
  if (err instanceof Error) return err.message
  if (typeof err === 'string') return err
  if (isObj(err) && typeof err.message === 'string') return err.message
  try {
    return JSON.stringify(err) ?? String(err)
  } catch {
    return String(err)
  }
}

/** One-sentence text summary of the graph for screen readers. */
export function graphSummary(graph: NormalizedGraph, kinds: Array<[string, number]>, matching: number | null): string {
  const parts = [
    `${graph.nodes.length} ${graph.nodes.length === 1 ? 'node' : 'nodes'} and ${graph.edges.length} ${graph.edges.length === 1 ? 'edge' : 'edges'}.`,
  ]
  if (kinds.length) parts.push(`Kinds: ${kinds.map(([k, n]) => `${k} ${n}`).join(', ')}.`)
  if (matching !== null) parts.push(`${matching} match the filter.`)
  return parts.join(' ')
}

/** True when a WebGL context can be created. Frees the probe context. */
export function detectWebGL(): boolean {
  try {
    if (typeof document === 'undefined') return false
    const canvas = document.createElement('canvas')
    const gl = (canvas.getContext('webgl2') ?? canvas.getContext('webgl')) as WebGLRenderingContext | null
    if (!gl) return false
    gl.getExtension?.('WEBGL_lose_context')?.loseContext()
    return true
  } catch {
    return false
  }
}

function prefersReducedMotion(): boolean {
  try {
    return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  } catch {
    return false
  }
}

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

// Kind colours, drawn from the SYNAPSE tokens. The accent (drape) is kept for
// interaction only: the selection marker and focus rings.
const KIND_COLORS: Record<string, string> = {
  core: '#d4dae2', // ink
  project: '#3fb950', // success
  observation: '#8b949e', // ink-soft
  change: '#d29922', // caution
  discovery: '#ff7b72', // alarm
  file: '#5d6673',
}
const kindColor = (kind: string) => KIND_COLORS[kind] ?? '#4a525e'
const DRAPE = '#1d9bf0'
const GROUND = '#0b0e13'
const RULE = '#262c37'

// ---------------------------------------------------------------------------
// Scene controls handed from the WebGL effect to React handlers
// ---------------------------------------------------------------------------

interface SceneControls {
  orbit: (dTheta: number, dPhi: number) => void
  zoom: (factor: number) => void
  reset: () => void
  select: (id: string | null) => void
  filter: (query: string) => void
}

type Load =
  | { status: 'loading' }
  | { status: 'outside' }
  | { status: 'error'; message: string }
  | { status: 'ready'; graph: NormalizedGraph }

type GlState = 'ok' | 'unavailable' | 'lost'

export function MemoryGraph() {
  const [load, setLoad] = useState<Load>({ status: 'loading' })
  const [gl, setGl] = useState<GlState>(() => (detectWebGL() ? 'ok' : 'unavailable'))
  const [sceneKey, setSceneKey] = useState(0)
  const [view, setView] = useState<'graph' | 'list'>('graph')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [hover, setHover] = useState<{ label: string; kind: string } | null>(null)
  const [query, setQuery] = useState('')
  const [reducedMotion, setReducedMotion] = useState(prefersReducedMotion)
  const [autoRotate, setAutoRotate] = useState(() => !prefersReducedMotion())
  const [sound, setSound] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  const [refreshError, setRefreshError] = useState<string | null>(null)

  const viewportRef = useRef<HTMLDivElement | null>(null)
  const mountRef = useRef<HTMLDivElement | null>(null)
  const readoutRef = useRef<HTMLSpanElement | null>(null)
  const tooltipRef = useRef<HTMLDivElement | null>(null)
  const controlsRef = useRef<SceneControls | null>(null)
  const audioRef = useRef<AudioContext | null>(null)
  const asideRef = useRef<HTMLElement | null>(null)
  const panelHeadingRef = useRef<HTMLHeadingElement | null>(null)
  const filterRef = useRef<HTMLInputElement | null>(null)
  const listRef = useRef<HTMLDivElement>(null)
  /** 'panel': move focus to the detail heading; a string: return focus to that node's row */
  const focusAfterRef = useRef<'panel' | { returnTo: string } | null>(null)
  const mountedRef = useRef(true)
  const inFlightRef = useRef(false)
  const seqRef = useRef(0)

  // Live values the render loop reads without rebuilding the scene.
  const autoRotateRef = useRef(autoRotate)
  autoRotateRef.current = autoRotate
  const reducedRef = useRef(reducedMotion)
  reducedRef.current = reducedMotion
  const soundRef = useRef(sound)
  soundRef.current = sound
  const selectedRef = useRef(selectedId)
  selectedRef.current = selectedId
  const queryRef = useRef(query)
  queryRef.current = query

  const graph = load.status === 'ready' ? load.graph : null
  const graphView = view === 'graph' && gl === 'ok'

  // --- data ---------------------------------------------------------------
  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
    }
  }, [])

  /**
   * Read the graph. One call in flight at a time (a second press is ignored),
   * and only the newest call may write state. A failed re-read keeps the last
   * graph on screen and reports the backend's text beside it.
   */
  const readGraph = useCallback(() => {
    if (!inTauri()) {
      setLoad({ status: 'outside' })
      return
    }
    if (inFlightRef.current) return
    inFlightRef.current = true
    const seq = ++seqRef.current
    setRefreshing(true)
    const current = () => mountedRef.current && seq === seqRef.current
    invoke<unknown>('get_memory_graph')
      .then(
        (res) => {
          if (!current()) return
          const next = normalizeGraph(res)
          setLoad({ status: 'ready', graph: next })
          setRefreshError(null)
          setSelectedId((id) => keepSelection(next, id))
        },
        (err: unknown) => {
          if (!current()) return
          const message = errorText(err)
          setLoad((prev) => (prev.status === 'ready' ? prev : { status: 'error', message }))
          setRefreshError(message)
        },
      )
      .finally(() => {
        if (seq !== seqRef.current) return
        inFlightRef.current = false
        if (mountedRef.current) setRefreshing(false)
      })
  }, [])

  useEffect(() => {
    readGraph()
  }, [readGraph])

  // --- reduced motion follows the OS setting live -------------------------
  useEffect(() => {
    const mq = typeof window !== 'undefined' ? window.matchMedia?.('(prefers-reduced-motion: reduce)') : undefined
    if (!mq?.addEventListener) return
    const onChange = (e: MediaQueryListEvent) => {
      setReducedMotion(e.matches)
      if (e.matches) setAutoRotate(false)
    }
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }, [])

  // --- sound (opt-in, released on unmount) --------------------------------
  const playTone = useCallback((freq: number, duration = 0.2, level = 0.08) => {
    if (!soundRef.current) return
    try {
      const Ctor =
        window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
      if (!Ctor) return
      if (!audioRef.current) audioRef.current = new Ctor()
      const ctx = audioRef.current
      if (ctx.state === 'suspended') void ctx.resume()
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()
      osc.frequency.setValueAtTime(freq, ctx.currentTime)
      gain.gain.setValueAtTime(level, ctx.currentTime)
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + duration)
      osc.connect(gain)
      gain.connect(ctx.destination)
      osc.start()
      osc.stop(ctx.currentTime + duration)
    } catch {
      /* audio is decoration; failure is silent by design */
    }
  }, [])
  const playToneRef = useRef(playTone)
  playToneRef.current = playTone

  useEffect(
    () => () => {
      void audioRef.current?.close?.().catch(() => {})
      audioRef.current = null
    },
    [],
  )

  // --- WebGL scene ----------------------------------------------------------
  useEffect(() => {
    const viewport = viewportRef.current
    const mount = mountRef.current
    if (!graph || !graphView || !viewport || !mount || graph.nodes.length === 0) return

    const lod = graph.nodes.length > LOD_THRESHOLD
    const size = () => ({ w: Math.max(1, viewport.clientWidth), h: Math.max(1, viewport.clientHeight) })
    let { w, h } = size()

    let renderer: THREE.WebGLRenderer
    try {
      renderer = new THREE.WebGLRenderer({ antialias: !lod, alpha: false })
    } catch {
      setGl('unavailable')
      return
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2))
    renderer.setSize(w, h)
    renderer.setClearColor(GROUND, 1)
    const canvas = renderer.domElement
    canvas.setAttribute('aria-hidden', 'true')
    canvas.style.display = 'block'
    mount.appendChild(canvas)

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(55, w / h, 1, 3000)

    scene.add(new THREE.AmbientLight(0xffffff, 0.75))
    const key = new THREE.DirectionalLight(0xffffff, 0.9)
    key.position.set(120, 200, 160)
    scene.add(key)

    const grid = new THREE.GridHelper(600, 24, RULE, RULE)
    grid.position.y = -110
    scene.add(grid)

    // Nodes: one shared sphere geometry, one material per kind.
    const sphere = lod ? new THREE.SphereGeometry(1, 8, 6) : new THREE.SphereGeometry(1, 20, 14)
    const materials = new Map<string, THREE.MeshLambertMaterial>()
    const materialFor = (kind: string) => {
      let m = materials.get(kind)
      if (!m) {
        const c = new THREE.Color(kindColor(kind))
        m = new THREE.MeshLambertMaterial({ color: c, emissive: c, emissiveIntensity: 0.25 })
        materials.set(kind, m)
      }
      return m
    }

    const meshes = new Map<string, THREE.Mesh>()
    const count = graph.nodes.length
    graph.nodes.forEach((node, idx) => {
      const mesh = new THREE.Mesh(sphere, materialFor(node.kind))
      const r = 3 + node.size * 7
      mesh.scale.setScalar(r)
      mesh.position.set(...layoutPosition(idx, count, lod))
      mesh.userData = { id: node.id, radius: r }
      scene.add(mesh)
      meshes.set(node.id, mesh)
    })

    // Edges: per-edge point runs, merged into one LineSegments draw call.
    const segs = lod ? 1 : 12
    const edgeRuns = graph.edges.map((e) => {
      const a = meshes.get(e.source)!.position
      const b = meshes.get(e.target)!.position
      const mid = a.clone().add(b).multiplyScalar(0.5)
      if (!lod && mid.lengthSq() > 0) mid.add(mid.clone().normalize().multiplyScalar(15))
      const pts = new THREE.QuadraticBezierCurve3(a.clone(), mid, b.clone()).getPoints(segs)
      const flat = new Float32Array(segs * 6)
      for (let i = 0; i < segs; i++) {
        flat.set([pts[i].x, pts[i].y, pts[i].z, pts[i + 1].x, pts[i + 1].y, pts[i + 1].z], i * 6)
      }
      return { edge: e, flat }
    })
    const edgeMat = new THREE.LineBasicMaterial({ color: '#8b949e', transparent: true, opacity: 0.35 })
    const edgeLines = new THREE.LineSegments(new THREE.BufferGeometry(), edgeMat)
    scene.add(edgeLines)

    // Selection marker (accent: interaction only).
    const markerGeo = new THREE.IcosahedronGeometry(1, 1)
    const markerMat = new THREE.MeshBasicMaterial({ color: DRAPE, wireframe: true })
    const marker = new THREE.Mesh(markerGeo, markerMat)
    marker.visible = false
    scene.add(marker)

    // --- orbit state ------------------------------------------------------
    const target = new THREE.Vector3()
    const sph = new THREE.Spherical().setFromVector3(HOME)
    let tween: {
      t: number
      fromT: THREE.Vector3
      toT: THREE.Vector3
      fromS: THREE.Spherical
      toS: THREE.Spherical
    } | null = null
    let dirty = true

    const applyCamera = () => {
      sph.phi = Math.max(0.1, Math.min(Math.PI - 0.1, sph.phi))
      sph.radius = Math.max(MIN_DISTANCE, Math.min(MAX_DISTANCE, sph.radius))
      camera.position.setFromSpherical(sph).add(target)
      camera.lookAt(target)
      dirty = true
    }
    const moveTo = (toT: THREE.Vector3, toS: THREE.Spherical) => {
      if (reducedRef.current) {
        target.copy(toT)
        sph.copy(toS)
        tween = null
        applyCamera()
        return
      }
      tween = { t: 0, fromT: target.clone(), toT: toT.clone(), fromS: sph.clone(), toS: toS.clone() }
    }
    applyCamera()

    // --- filter & pickables ----------------------------------------------
    let pickables: THREE.Mesh[] = []
    let hovered: THREE.Mesh | null = null
    const setHovered = (m: THREE.Mesh | null) => {
      if (hovered === m) return
      if (hovered) hovered.scale.setScalar(hovered.userData.radius)
      hovered = m
      if (m) {
        m.scale.setScalar(m.userData.radius * 1.2)
        const node = graph.nodes.find((n) => n.id === m.userData.id)
        setHover(node ? { label: node.label, kind: node.kind } : null)
        playToneRef.current(720, 0.12, 0.05)
      } else {
        setHover(null)
      }
      viewport.style.cursor = m ? 'pointer' : 'grab'
      dirty = true
    }

    const applyFilter = (q: string) => {
      const visible = new Set<string>()
      for (const node of graph.nodes) {
        const ok = nodeMatches(node, q)
        meshes.get(node.id)!.visible = ok
        if (ok) visible.add(node.id)
      }
      pickables = [...meshes.values()].filter((m) => m.visible)
      if (hovered && !hovered.visible) setHovered(null)
      const runs = edgeRuns.filter((r) => visible.has(r.edge.source) && visible.has(r.edge.target))
      const buf = new Float32Array(runs.length * segs * 6)
      runs.forEach((r, i) => buf.set(r.flat, i * segs * 6))
      const old = edgeLines.geometry
      const geo = new THREE.BufferGeometry()
      geo.setAttribute('position', new THREE.BufferAttribute(buf, 3))
      edgeLines.geometry = geo
      old.dispose()
      const sel = selectedRef.current ? meshes.get(selectedRef.current) : undefined
      marker.visible = !!sel && sel.visible
      dirty = true
    }

    const select = (id: string | null) => {
      const m = id ? meshes.get(id) : undefined
      if (!m) {
        marker.visible = false
        dirty = true
        return
      }
      marker.position.copy(m.position)
      marker.scale.setScalar(m.userData.radius * 1.7)
      marker.visible = m.visible
      const toS = sph.clone()
      toS.radius = FOCUS_DISTANCE
      moveTo(m.position, toS)
      dirty = true
    }

    applyFilter(queryRef.current)
    select(selectedRef.current)

    // --- pointer ------------------------------------------------------------
    const raycaster = new THREE.Raycaster()
    const ndc = new THREE.Vector2()
    const pick = (cx: number, cy: number): THREE.Mesh | null => {
      const rect = canvas.getBoundingClientRect()
      if (rect.width === 0 || rect.height === 0) return null
      ndc.set(((cx - rect.left) / rect.width) * 2 - 1, -((cy - rect.top) / rect.height) * 2 + 1)
      raycaster.setFromCamera(ndc, camera)
      const hit = raycaster.intersectObjects(pickables, false)[0]
      return hit ? (hit.object as THREE.Mesh) : null
    }
    const placeTooltip = (cx: number, cy: number) => {
      const tip = tooltipRef.current
      if (!tip) return
      const rect = viewport.getBoundingClientRect()
      tip.style.left = `${cx - rect.left + 14}px`
      tip.style.top = `${cy - rect.top + 14}px`
    }

    let drag: { id: number; x: number; y: number; sx: number; sy: number } | null = null
    const onPointerDown = (e: PointerEvent) => {
      if (e.button !== 0) return
      drag = { id: e.pointerId, x: e.clientX, y: e.clientY, sx: e.clientX, sy: e.clientY }
      tween = null
      viewport.setPointerCapture?.(e.pointerId)
      viewport.style.cursor = 'grabbing'
    }
    const onPointerMove = (e: PointerEvent) => {
      if (drag && drag.id === e.pointerId) {
        sph.theta -= (e.clientX - drag.x) * DRAG_SPEED
        sph.phi -= (e.clientY - drag.y) * DRAG_SPEED
        drag.x = e.clientX
        drag.y = e.clientY
        applyCamera()
        return
      }
      setHovered(pick(e.clientX, e.clientY))
      placeTooltip(e.clientX, e.clientY)
    }
    const onPointerUp = (e: PointerEvent) => {
      if (!drag || drag.id !== e.pointerId) return
      const moved = Math.hypot(e.clientX - drag.sx, e.clientY - drag.sy)
      drag = null
      viewport.releasePointerCapture?.(e.pointerId)
      viewport.style.cursor = hovered ? 'pointer' : 'grab'
      if (moved > CLICK_SLOP) return // a drag, not a click
      const hit = pick(e.clientX, e.clientY)
      if (hit) {
        setSelectedId(hit.userData.id as string)
        playToneRef.current(440, 0.3, 0.1)
      }
    }
    const onPointerLeave = () => {
      if (!drag) setHovered(null)
    }
    const onPointerCancel = () => {
      drag = null
    }
    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      tween = null
      sph.radius *= Math.exp(Math.max(-200, Math.min(200, e.deltaY)) * 0.0015)
      applyCamera()
      frame() // same as the keyboard zoom: the readout follows the wheel at once
    }

    viewport.addEventListener('pointerdown', onPointerDown)
    viewport.addEventListener('pointermove', onPointerMove)
    viewport.addEventListener('pointerup', onPointerUp)
    viewport.addEventListener('pointerleave', onPointerLeave)
    viewport.addEventListener('pointercancel', onPointerCancel)
    viewport.addEventListener('wheel', onWheel, { passive: false })

    const onContextLost = (e: Event) => {
      e.preventDefault()
      setGl('lost')
    }
    canvas.addEventListener('webglcontextlost', onContextLost)

    // --- resize -----------------------------------------------------------
    const onResize = () => {
      ;({ w, h } = size())
      camera.aspect = w / h
      camera.updateProjectionMatrix()
      renderer.setSize(w, h)
      dirty = true
    }
    let ro: ResizeObserver | null = null
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(onResize)
      ro.observe(viewport)
    } else {
      window.addEventListener('resize', onResize)
    }

    // --- render loop --------------------------------------------------------
    const writeReadout = () => {
      const el = readoutRef.current
      if (!el) return
      const p = camera.position
      el.textContent = `x ${p.x.toFixed(0)}  y ${p.y.toFixed(0)}  z ${p.z.toFixed(0)}  dist ${sph.radius.toFixed(0)}`
    }
    const frame = () => {
      if (tween) {
        tween.t = Math.min(1, tween.t + 0.06)
        const k = 1 - Math.pow(1 - tween.t, 3)
        target.lerpVectors(tween.fromT, tween.toT, k)
        sph.radius = tween.fromS.radius + (tween.toS.radius - tween.fromS.radius) * k
        sph.phi = tween.fromS.phi + (tween.toS.phi - tween.fromS.phi) * k
        sph.theta = tween.fromS.theta + (tween.toS.theta - tween.fromS.theta) * k
        if (tween.t >= 1) tween = null
        applyCamera()
      } else if (autoRotateRef.current && !reducedRef.current && !drag) {
        sph.theta += 0.002
        applyCamera()
      }
      if (marker.visible && !reducedRef.current) {
        marker.rotation.y += 0.01
        dirty = true
      }
      if (dirty) {
        renderer.render(scene, camera)
        writeReadout()
        dirty = false
      }
    }
    let raf = 0
    const loop = () => {
      raf = requestAnimationFrame(loop)
      frame()
    }
    raf = requestAnimationFrame(loop)

    controlsRef.current = {
      orbit: (dTheta, dPhi) => {
        tween = null
        sph.theta += dTheta
        sph.phi += dPhi
        applyCamera()
        frame()
      },
      zoom: (factor) => {
        tween = null
        sph.radius *= factor
        applyCamera()
        frame()
      },
      reset: () => {
        moveTo(new THREE.Vector3(), new THREE.Spherical().setFromVector3(HOME))
        frame()
      },
      select: (id) => {
        select(id)
        frame()
      },
      filter: (q) => {
        applyFilter(q)
        frame()
      },
    }
    frame()

    return () => {
      cancelAnimationFrame(raf)
      controlsRef.current = null
      ro?.disconnect()
      window.removeEventListener('resize', onResize)
      viewport.removeEventListener('pointerdown', onPointerDown)
      viewport.removeEventListener('pointermove', onPointerMove)
      viewport.removeEventListener('pointerup', onPointerUp)
      viewport.removeEventListener('pointerleave', onPointerLeave)
      viewport.removeEventListener('pointercancel', onPointerCancel)
      viewport.removeEventListener('wheel', onWheel)
      // Remove before forcing context loss, or our own teardown reads as a GPU reset.
      canvas.removeEventListener('webglcontextlost', onContextLost)
      const geos = new Set<THREE.BufferGeometry>()
      const mats = new Set<THREE.Material>()
      scene.traverse((o) => {
        const g = (o as THREE.Mesh).geometry
        const m = (o as THREE.Mesh).material
        if (g) geos.add(g)
        if (Array.isArray(m)) m.forEach((x) => mats.add(x))
        else if (m) mats.add(m)
      })
      geos.forEach((g) => g.dispose())
      mats.forEach((m) => m.dispose())
      scene.clear()
      renderer.dispose()
      renderer.forceContextLoss?.()
      canvas.remove()
      setHover(null)
    }
  }, [graph, graphView, sceneKey])

  // Push React state into the live scene without rebuilding it.
  useEffect(() => {
    controlsRef.current?.select(selectedId)
  }, [selectedId])

  // Focus follows selection changes made from inside the detail panel, so a
  // keyboard user is never dropped on <body> when the button they pressed goes.
  useEffect(() => {
    const want = focusAfterRef.current
    if (!want) return
    focusAfterRef.current = null
    if (want === 'panel') {
      panelHeadingRef.current?.focus()
      return
    }
    const row = [...(listRef.current?.querySelectorAll<HTMLElement>('[data-node-id]') ?? [])].find(
      (el) => el.dataset.nodeId === want.returnTo,
    )
    ;(row ?? viewportRef.current ?? filterRef.current)?.focus()
  }, [selectedId])

  const selectFromPanel = (id: string) => {
    if (id === selectedRef.current) {
      // a self-link: nothing changes, so no effect will run to move focus
      panelHeadingRef.current?.focus()
      return
    }
    focusAfterRef.current = 'panel'
    setSelectedId(id)
  }
  const clearSelection = useCallback(() => {
    const active = typeof document !== 'undefined' ? document.activeElement : null
    const id = selectedRef.current
    if (id !== null && asideRef.current && active && asideRef.current.contains(active)) {
      focusAfterRef.current = { returnTo: id }
    }
    setSelectedId(null)
  }, [])
  useEffect(() => {
    controlsRef.current?.filter(query)
  }, [query])

  // --- shortcuts (registered only while this screen is mounted) -------------
  useShortcut(
    { id: 'memory.reset', keys: '0', label: 'Memory: reset view', group: 'View', when: () => graphView },
    () => controlsRef.current?.reset(),
  )
  useShortcut(
    { id: 'memory.clear', keys: 'escape', label: 'Memory: clear selection', group: 'View', when: () => selectedRef.current !== null },
    clearSelection,
  )
  useShortcut(
    { id: 'memory.list', keys: 'l', label: 'Memory: switch graph or list view', group: 'View', when: () => gl === 'ok' },
    () => setView((v) => (v === 'graph' ? 'list' : 'graph')),
  )

  const onViewportKey = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    const c = controlsRef.current
    // Leave Ctrl/Cmd/Alt combos (browser zoom, app shortcuts) alone.
    if (!c || e.ctrlKey || e.metaKey || e.altKey) return
    switch (e.key) {
      case 'ArrowLeft':
        c.orbit(ORBIT_STEP, 0)
        break
      case 'ArrowRight':
        c.orbit(-ORBIT_STEP, 0)
        break
      case 'ArrowUp':
        c.orbit(0, -ORBIT_STEP)
        break
      case 'ArrowDown':
        c.orbit(0, ORBIT_STEP)
        break
      case '+':
      case '=':
        c.zoom(ZOOM_STEP)
        break
      case '-':
      case '_':
        c.zoom(1 / ZOOM_STEP)
        break
      default:
        return
    }
    e.preventDefault()
  }

  // --- derived --------------------------------------------------------------
  const selected = useMemo(
    () => (graph && selectedId ? graph.nodes.find((n) => n.id === selectedId) ?? null : null),
    [graph, selectedId],
  )
  const links = useMemo(() => {
    if (!graph || !selected) return []
    const byId = new Map(graph.nodes.map((n) => [n.id, n]))
    return graph.edges
      .filter((e) => e.source === selected.id || e.target === selected.id)
      .map((e) => {
        const other = byId.get(e.source === selected.id ? e.target : e.source)!
        return { edge: e, other, outgoing: e.source === selected.id }
      })
  }, [graph, selected])
  const kinds = useMemo(() => {
    const m = new Map<string, number>()
    graph?.nodes.forEach((n) => m.set(n.kind, (m.get(n.kind) ?? 0) + 1))
    return [...m.entries()].sort((a, b) => b[1] - a[1])
  }, [graph])
  const visibleNodes = useMemo(() => graph?.nodes.filter((n) => nodeMatches(n, query)) ?? [], [graph, query])
  const summary = useMemo(
    () => (graph ? graphSummary(graph, kinds, query.trim() ? visibleNodes.length : null) : ''),
    [graph, kinds, query, visibleNodes.length],
  )

  // --- render ---------------------------------------------------------------
  if (load.status === 'outside') {
    return (
      <Section>
        <p className="font-mono text-sm">No backend attached.</p>
        <p className="mt-2 max-w-prose text-sm text-ink-soft">
          This window is running outside the desktop shell, so there is no memory store to read. Nothing is drawn
          in its place.
        </p>
      </Section>
    )
  }
  if (load.status === 'error') {
    return (
      <Section>
        <p className="font-mono text-sm text-alarm">Memory graph could not be read.</p>
        <p className="mt-2 max-w-prose break-words font-mono text-xs text-ink-soft">
          get_memory_graph: {load.message}
        </p>
        <button
          type="button"
          className="mt-3 border border-rule px-2.5 py-1 font-mono text-xs text-ink-soft hover:border-drape hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-drape disabled:opacity-50"
          disabled={refreshing}
          aria-busy={refreshing}
          onClick={readGraph}
        >
          {refreshing ? 'reading…' : 'try again'}
        </button>
      </Section>
    )
  }
  if (load.status === 'loading' || !graph) {
    return (
      <Section>
        <p className="font-mono text-xs text-ink-soft" role="status">
          reading memory store…
        </p>
      </Section>
    )
  }

  const btn =
    'border border-rule px-2.5 py-1 font-mono text-xs text-ink-soft hover:border-drape hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-drape disabled:opacity-50 disabled:hover:border-rule disabled:hover:text-ink-soft'

  return (
    <Section>
      {/* Readout: measured counts only */}
      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1 border-b border-rule pb-3 font-mono text-xs text-ink-soft">
        <span>
          nodes <span className="text-ink">{graph.nodes.length}</span>
          {graph.totalNodes > graph.nodes.length && <span> of {graph.totalNodes}</span>}
        </span>
        <span>
          edges <span className="text-ink">{graph.edges.length}</span>
        </span>
        <span>
          observations{' '}
          <span className="text-ink">{graph.observations === null ? 'not reported' : graph.observations}</span>
        </span>
        {query.trim() && (
          <span>
            matching <span className="text-ink">{visibleNodes.length}</span>
          </span>
        )}
        <button
          type="button"
          className={cn(btn, 'ml-auto')}
          disabled={refreshing}
          aria-busy={refreshing}
          onClick={readGraph}
        >
          {refreshing ? 'reading…' : 'refresh'}
        </button>
      </div>
      <p className="sr-only" role="status" aria-live="polite" data-testid="memory-announce">
        {refreshing
          ? 'Reading memory store.'
          : selected
            ? `Selected ${selected.label}, ${selected.kind}, ${links.length} ${links.length === 1 ? 'link' : 'links'}.`
            : ''}
      </p>
      {refreshError && (
        <p className="mt-2 break-words font-mono text-xs text-alarm" role="alert">
          Refresh failed; showing the last graph read. get_memory_graph: {refreshError}
        </p>
      )}
      {graph.totalNodes > graph.nodes.length && (
        <p className="mt-2 font-mono text-xs text-caution">
          Showing the first {graph.nodes.length} of {graph.totalNodes} nodes. {graph.cappedEdges} edges to nodes past
          the cap are not drawn.
        </p>
      )}
      {graph.danglingEdges > 0 && (
        <p className="mt-2 font-mono text-xs text-caution">
          {graph.danglingEdges} {graph.danglingEdges === 1 ? 'edge references' : 'edges reference'} a node missing
          from the payload and {graph.danglingEdges === 1 ? 'is' : 'are'} not drawn.
        </p>
      )}

      {graph.nodes.length === 0 ? (
        <div className="mt-4">
          <p className="font-mono text-sm">No memory nodes recorded.</p>
          <p className="mt-2 max-w-prose text-sm text-ink-soft">
            The memory store under this brain returned an empty graph. Nodes appear here once a tool records an
            observation.
          </p>
        </div>
      ) : (
        <>
          {/* Toolbar */}
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <label className="flex items-center gap-2 font-mono text-xs text-ink-soft">
              filter
              <input
                ref={filterRef}
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="label, kind, file"
                className="w-48 border border-rule bg-ground px-2 py-1 font-mono text-xs text-ink placeholder:text-ink-soft focus:border-drape focus:outline-none"
              />
            </label>
            <button
              type="button"
              className={btn}
              aria-pressed={view === 'list' || gl !== 'ok'}
              disabled={gl !== 'ok'}
              onClick={() => setView((v) => (v === 'graph' ? 'list' : 'graph'))}
            >
              {graphView ? 'list view' : 'graph view'}
            </button>
            {graphView && (
              <>
                <button type="button" className={btn} onClick={() => controlsRef.current?.reset()}>
                  reset view
                </button>
                <button
                  type="button"
                  className={btn}
                  aria-pressed={autoRotate && !reducedMotion}
                  disabled={reducedMotion}
                  onClick={() => setAutoRotate((v) => !v)}
                  title={reducedMotion ? 'Your system asks for reduced motion, so the graph does not rotate' : undefined}
                >
                  auto-rotate {autoRotate && !reducedMotion ? 'on' : 'off'}
                </button>
                <button
                  type="button"
                  className={btn}
                  aria-pressed={sound}
                  onClick={() => {
                    const next = !sound
                    soundRef.current = next
                    setSound(next)
                    if (next) playTone(580)
                  }}
                >
                  sound {sound ? 'on' : 'off'}
                </button>
              </>
            )}
          </div>

          {gl === 'unavailable' && (
            <p className="mt-3 max-w-prose text-sm text-ink-soft" role="note">
              <span className="font-mono text-caution">WebGL is not available</span> in this window, so the 3D graph
              cannot be drawn. The same nodes and edges are listed below.
            </p>
          )}
          {gl === 'lost' && (
            <p className="mt-3 max-w-prose text-sm text-ink-soft" role="alert">
              <span className="font-mono text-caution">The graphics context was lost</span> (GPU reset or too many
              3D views open).{' '}
              <button
                type="button"
                className={btn}
                onClick={() => {
                  setGl('ok')
                  setSceneKey((k) => k + 1)
                }}
              >
                rebuild view
              </button>
            </p>
          )}

          {graphView && (
            <>
              <Hint id="memory.orbit">Drag or use the arrow keys to orbit. Scroll or press + and - to zoom.</Hint>
              <Hint id="memory.inspect">Click a node to inspect it. Esc clears the selection, 0 resets the view.</Hint>
            </>
          )}

          <div className="mt-3 flex border border-rule">
            <div className="relative min-w-0 flex-1">
              {graphView ? (
                <div
                  ref={viewportRef}
                  tabIndex={0}
                  role="application"
                  aria-roledescription="3D graph"
                  aria-label={`Memory graph, ${graph.nodes.length} nodes. Arrow keys orbit, plus and minus zoom, Escape clears the selection. Press L for a list view.`}
                  aria-describedby="memory-summary"
                  onKeyDown={onViewportKey}
                  className="relative h-[60vh] min-h-[360px] cursor-grab touch-none bg-ground outline-none focus-visible:outline focus-visible:outline-1 focus-visible:outline-drape"
                  data-testid="memory-viewport"
                >
                  <div ref={mountRef} className="absolute inset-0" />
                  <p id="memory-summary" className="sr-only">
                    {summary}
                  </p>
                  <div className="pointer-events-none absolute left-3 top-2 font-mono text-[11px] text-ink-soft">
                    camera <span ref={readoutRef} data-testid="memory-readout" />
                  </div>
                  <div
                    ref={tooltipRef}
                    role="tooltip"
                    className={cn(
                      'pointer-events-none absolute z-10 max-w-[18rem] truncate border border-rule bg-chart px-2 py-1 font-mono text-[11px]',
                      !hover && 'hidden',
                    )}
                  >
                    {hover && (
                      <>
                        <span className="text-ink">{hover.label}</span>
                        <span className="text-ink-soft"> · {hover.kind}</span>
                      </>
                    )}
                  </div>
                  {query.trim() && visibleNodes.length === 0 && (
                    <p className="absolute inset-x-0 top-1/2 text-center font-mono text-xs text-ink-soft">
                      No node matches the filter.
                    </p>
                  )}
                </div>
              ) : (
                <NodeList
                  listRef={listRef}
                  nodes={visibleNodes}
                  edges={graph.edges}
                  selectedId={selectedId}
                  onSelect={setSelectedId}
                />
              )}
            </div>

            {selected && (
              <aside
                ref={asideRef}
                aria-label={`Node ${selected.label}`}
                className="w-72 shrink-0 overflow-y-auto border-l border-rule bg-chart p-4 text-xs"
              >
                <div className="flex items-baseline justify-between border-b border-rule pb-2">
                  <span className="min-w-0 truncate font-mono uppercase tracking-wider text-ink-soft" title={selected.kind}>
                    {selected.kind}
                  </span>
                  <button
                    type="button"
                    aria-label={`Close details for ${selected.label}`}
                    onClick={clearSelection}
                    className="font-mono text-ink-soft hover:text-drape focus-visible:text-drape"
                  >
                    close
                  </button>
                </div>
                <h2
                  ref={panelHeadingRef}
                  tabIndex={-1}
                  className="mt-3 text-sm text-ink outline-none [overflow-wrap:anywhere] focus-visible:outline focus-visible:outline-1 focus-visible:outline-drape"
                >
                  {selected.label}
                </h2>
                <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 font-mono text-[11px]">
                  <dt className="text-ink-soft">id</dt>
                  <dd className="break-all">{selected.id}</dd>
                  <dt className="text-ink-soft">weight</dt>
                  <dd>{selected.weight === null ? 'not measured' : selected.weight}</dd>
                  <dt className="text-ink-soft">recorded</dt>
                  <dd>{formatTs(selected.ts)}</dd>
                </dl>
                <h3 className="mt-4 border-b border-rule pb-1 font-mono text-[11px] uppercase tracking-wider text-ink-soft">
                  files
                </h3>
                {selected.files.length === 0 ? (
                  <p className="mt-1 font-mono text-[11px] text-ink-soft">none recorded</p>
                ) : (
                  <ul className="mt-1 space-y-0.5 font-mono text-[11px]">
                    {selected.files.map((f) => (
                      <li key={f} className="break-all">
                        {f}
                      </li>
                    ))}
                  </ul>
                )}
                <h3 className="mt-4 border-b border-rule pb-1 font-mono text-[11px] uppercase tracking-wider text-ink-soft">
                  narrative
                </h3>
                <p className="mt-1 leading-relaxed text-ink [overflow-wrap:anywhere]">
                  {selected.narrative || <span className="font-mono text-ink-soft">none recorded</span>}
                </p>
                <h3 className="mt-4 border-b border-rule pb-1 font-mono text-[11px] uppercase tracking-wider text-ink-soft">
                  links {links.length}
                </h3>
                {links.length === 0 ? (
                  <p className="mt-1 font-mono text-[11px] text-ink-soft">no edges</p>
                ) : (
                  <ul className="mt-1 space-y-0.5 font-mono text-[11px]">
                    {links.map(({ edge, other, outgoing }, i) => (
                      <li key={`${edge.source}-${edge.target}-${i}`}>
                        <span className="text-ink-soft">
                          {outgoing ? '→' : '←'} {edge.label || 'linked'}{' '}
                        </span>
                        <button
                          type="button"
                          onClick={() => selectFromPanel(other.id)}
                          className="text-left text-ink underline [overflow-wrap:anywhere] decoration-rule underline-offset-2 hover:text-drape focus-visible:text-drape"
                        >
                          {other.label}
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </aside>
            )}
          </div>

          {/* Legend: kinds present in this payload, with counts */}
          <ul className="mt-2 flex flex-wrap gap-x-4 gap-y-1 font-mono text-[11px] text-ink-soft" aria-label="Node kinds">
            {kinds.map(([kind, n]) => (
              <li key={kind} className="flex items-center gap-1.5">
                <span aria-hidden="true" className="inline-block h-2 w-2" style={{ background: kindColor(kind) }} />
                {kind} <span className="text-ink">{n}</span>
              </li>
            ))}
          </ul>
        </>
      )}
    </Section>
  )
}

function Section({ children }: { children: ReactNode }) {
  return (
    <section aria-labelledby="memory-title">
      <h1 id="memory-title" className="mb-3 font-mono text-xs uppercase tracking-[0.2em] text-ink-soft">
        Memory
      </h1>
      {children}
    </section>
  )
}

function NodeList({
  listRef,
  nodes,
  edges,
  selectedId,
  onSelect,
}: {
  listRef: RefObject<HTMLDivElement>
  nodes: GraphNode[]
  edges: GraphEdge[]
  selectedId: string | null
  onSelect: (id: string) => void
}) {
  const degree = useMemo(() => {
    const d = new Map<string, number>()
    for (const e of edges) {
      d.set(e.source, (d.get(e.source) ?? 0) + 1)
      d.set(e.target, (d.get(e.target) ?? 0) + 1)
    }
    return d
  }, [edges])
  if (nodes.length === 0) return <p className="p-3 font-mono text-xs text-ink-soft">No node matches the filter.</p>
  return (
    <div ref={listRef} className="max-h-[60vh] overflow-y-auto">
      <table className="w-full border-collapse font-mono text-xs" aria-label="Memory nodes">
        <thead>
          <tr className="border-b border-rule text-left text-ink-soft">
            <th scope="col" className="px-3 py-1.5 font-normal">kind</th>
            <th scope="col" className="px-3 py-1.5 font-normal">label</th>
            <th scope="col" className="px-3 py-1.5 text-right font-normal">weight</th>
            <th scope="col" className="px-3 py-1.5 text-right font-normal">links</th>
            <th scope="col" className="px-3 py-1.5 text-right font-normal">files</th>
          </tr>
        </thead>
        <tbody>
          {nodes.map((n) => (
            <tr key={n.id} className={cn('border-b border-rule', n.id === selectedId && 'bg-chart')}>
              <td className="px-3 py-1 text-ink-soft [overflow-wrap:anywhere]">{n.kind}</td>
              <td className="px-3 py-1">
                <button
                  type="button"
                  data-node-id={n.id}
                  aria-pressed={n.id === selectedId}
                  onClick={() => onSelect(n.id)}
                  className="text-left text-ink [overflow-wrap:anywhere] hover:text-drape focus-visible:text-drape focus-visible:outline focus-visible:outline-1 focus-visible:outline-drape"
                >
                  {n.label}
                </button>
              </td>
              <td className="px-3 py-1 text-right">{n.weight === null ? '—' : n.weight}</td>
              <td className="px-3 py-1 text-right">{degree.get(n.id) ?? 0}</td>
              <td className="px-3 py-1 text-right">{n.files.length}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
