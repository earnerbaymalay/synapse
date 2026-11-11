import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ChartFoot, ChartFrame, ChartHead, Glyph, Readout, plural, type Mark } from '../components/chart'
import { Hint } from '../lib/hints'
import { useShortcut } from '../lib/shortcuts'
import type { CockpitTelemetry } from './types'

/**
 * Cockpit — live vital signs: the local model gateway, sensory gating
 * (prompt compression) and Brain health, re-read on a fixed interval.
 *
 * Rules this screen keeps (IDENTITY.md):
 * - Nothing is displayed that isn't measured. Outside the desktop shell
 *   there is no backend, so the screen says so instead of showing samples.
 * - A field the backend did not send is "not reported", never 0.
 * - The trace only beats while readings are arriving; paused, offline or
 *   failing, it says so and goes flat.
 */

/**
 * Whether the backend measures the gateway and sensory-gating fields.
 *
 * In this build `get_cockpit_telemetry` (src-tauri/src/commands.rs) returns
 * fixed literals for gateway status, upstream, model, latency, request and
 * token counts. Showing them as live would break "nothing displayed that
 * isn't measured", so those blocks say "not measured in this build" until
 * the backend reads a real gateway. Brain path, adapter and Doctor counts
 * are measured and always shown. Flip this when the backend is fixed.
 */
export const GATEWAY_MEASURED = false

/** How often live telemetry is re-read. */
export const POLL_MS = 5000

/** brands/synapse/tokens.json `motion.ekgScroll`: one trace width per 3.4 s, linear. */
const EKG_SCROLL_MS = 3400
/** Width of one P-QRS-T complex on the trace, in CSS pixels. */
const BEAT_PX = 200

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

// ── telemetry normalisation ────────────────────────────────────────────────

/** A telemetry reading where every field may be absent (`null` = not reported). */
export type Reading = { [K in keyof CockpitTelemetry]: CockpitTelemetry[K] | null }

const num = (v: unknown): number | null =>
  typeof v === 'number' && Number.isFinite(v) ? v : null
const str = (v: unknown): string | null =>
  typeof v === 'string' && v.trim() !== '' ? v : null

/**
 * Defensive read of whatever came over IPC. A missing or malformed field
 * becomes `null` rather than crashing the screen (`undefined.toLocaleString()`)
 * or masquerading as zero.
 */
export function normalize(raw: unknown): Reading {
  const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  return {
    gateway_status: str(r.gateway_status),
    upstream_url: str(r.upstream_url),
    model: str(r.model),
    latency_ms: num(r.latency_ms),
    requests_handled: num(r.requests_handled),
    raw_prompt_tokens: num(r.raw_prompt_tokens),
    gated_prompt_tokens: num(r.gated_prompt_tokens),
    tokens_saved: num(r.tokens_saved),
    reduction_pct: num(r.reduction_pct),
    brain_path: str(r.brain_path),
    adapters_present: num(r.adapters_present),
    adapters_total: num(r.adapters_total),
    doctor_criticals: num(r.doctor_criticals),
    doctor_fixable: num(r.doctor_fixable),
  }
}

// ── formatting ─────────────────────────────────────────────────────────────

const INT = new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 })
const PCT = new Intl.NumberFormat('en-US', { minimumFractionDigits: 1, maximumFractionDigits: 1 })

export const fmtInt = (n: number | null): string | null => (n === null ? null : INT.format(n))
export const fmtPct = (n: number | null): string | null => (n === null ? null : `${PCT.format(n)}%`)
export const fmtMs = (n: number | null): string | null => (n === null ? null : `${INT.format(n)} ms`)

/**
 * The backend's error text. Tauri commands reject with a string, but a plugin
 * or IPC failure can reject with an Error or a `{ message }` object; neither
 * should reach the user as "[object Object]" or as an empty reason.
 */
export function errorText(e: unknown): string {
  let s = ''
  if (e instanceof Error) s = e.message
  else if (typeof e === 'string') s = e
  else if (e && typeof e === 'object' && typeof (e as { message?: unknown }).message === 'string')
    s = (e as { message: string }).message
  else if (e !== null && e !== undefined) s = String(e)
  return s.trim() === '' ? 'no reason given by the backend' : s
}

function fmtClock(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

// ── status marks ───────────────────────────────────────────────────────────

function gatewayMark(status: string | null): Mark {
  if (status === null) return 'absent'
  const s = status.toUpperCase()
  if (s === 'ONLINE') return 'present'
  if (s === 'OFFLINE' || s === 'DOWN' || s === 'ERROR') return 'critical'
  return 'warning'
}

function brainMark(r: Reading): Mark {
  if (r.doctor_criticals === null && r.adapters_present === null) return 'absent'
  if ((r.doctor_criticals ?? 0) > 0) return 'critical'
  if ((r.doctor_fixable ?? 0) > 0) return 'warning'
  if (r.adapters_present !== null && r.adapters_total !== null && r.adapters_present < r.adapters_total)
    return 'partial'
  return 'present'
}

function finding(r: Reading, gatewayMeasured: boolean): string {
  if ((r.doctor_criticals ?? 0) > 0) {
    const n = r.doctor_criticals as number
    return `${plural(n, 'critical finding')} ${n === 1 ? 'needs' : 'need'} a human.`
  }
  // "Steady" is a claim; it needs both Doctor counts actually reported.
  const doctorReported = r.doctor_criticals !== null && r.doctor_fixable !== null
  if (!gatewayMeasured) {
    if ((r.doctor_fixable ?? 0) > 0) return `${plural(r.doctor_fixable as number, 'fixable finding')} on the Brain.`
    if (!doctorReported) return 'Doctor counts not reported.'
    return 'Brain steady. Gateway not measured.'
  }
  if (gatewayMark(r.gateway_status) === 'critical') return `Gateway reports ${r.gateway_status}.`
  if (r.gateway_status === null) return 'Gateway status not reported.'
  if ((r.doctor_fixable ?? 0) > 0) return `${plural(r.doctor_fixable as number, 'fixable finding')} on the Brain.`
  if (!doctorReported) return 'Doctor counts not reported.'
  return 'Vitals steady.'
}

function nextCommand(r: Reading, gatewayMeasured: boolean): string {
  if ((r.doctor_fixable ?? 0) > 0) return 'synapse doctor --fix'
  if ((r.doctor_criticals ?? 0) > 0) return 'synapse doctor'
  if (gatewayMeasured && gatewayMark(r.gateway_status) !== 'present') return 'synapse serve'
  return 'synapse hud --once'
}

// ── screen ─────────────────────────────────────────────────────────────────

type Status = 'loading' | 'ok' | 'error' | 'detached'

export function Cockpit({ gatewayMeasured = GATEWAY_MEASURED }: { gatewayMeasured?: boolean } = {}) {
  const [reading, setReading] = useState<Reading | null>(null)
  const [status, setStatus] = useState<Status>(() => (inTauri() ? 'loading' : 'detached'))
  const [error, setError] = useState<string | null>(null)
  const [readAt, setReadAt] = useState<Date | null>(null)
  const [live, setLive] = useState(true)
  const [busy, setBusy] = useState(false)

  const mounted = useRef(true)
  const inFlight = useRef(false)

  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])

  /** One read. Never overlaps itself; never sets state after unmount. */
  const read = useCallback(async () => {
    if (!inTauri() || inFlight.current) return
    inFlight.current = true
    setBusy(true)
    try {
      const res = await invoke<CockpitTelemetry>('get_cockpit_telemetry', { brain: null, tools: null })
      if (!mounted.current) return
      setReading(normalize(res))
      setError(null)
      setStatus('ok')
      setReadAt(new Date())
    } catch (e) {
      if (!mounted.current) return
      // Keep the last good reading on screen, but say it is stale.
      setError(errorText(e))
      setStatus('error')
    } finally {
      inFlight.current = false
      if (mounted.current) setBusy(false)
    }
  }, [])

  // First read on mount, whether or not live updates are on.
  useEffect(() => {
    void read()
  }, [read])

  // Live polling: a setTimeout chain (not setInterval) so a slow read can't
  // stack up behind itself. Skips reads while the window is hidden and reads
  // once immediately when it becomes visible again.
  useEffect(() => {
    if (!live || !inTauri()) return
    let timer: ReturnType<typeof setTimeout> | undefined
    let cancelled = false
    // True while a tick is awaiting its read. Without it, a visibility flip
    // during a slow read saw `timer === undefined`, started a second tick, and
    // both ticks rescheduled: two polling chains, one of them never cleared.
    let ticking = false

    const schedule = () => {
      if (cancelled) return
      if (timer !== undefined) clearTimeout(timer)
      timer = setTimeout(tick, POLL_MS)
    }
    const tick = async () => {
      timer = undefined
      if (cancelled || ticking) return
      if (typeof document !== 'undefined' && document.hidden) return // resumed by onVisible
      ticking = true
      try {
        await read()
      } finally {
        ticking = false
      }
      schedule()
    }
    const onVisible = () => {
      if (cancelled || document.hidden || timer !== undefined || ticking) return
      void tick()
    }

    schedule()
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      cancelled = true
      if (timer !== undefined) clearTimeout(timer)
      document.removeEventListener('visibilitychange', onVisible)
    }
  }, [live, read])

  const toggleLive = useCallback(() => setLive((v) => !v), [])

  useShortcut(
    {
      id: 'cockpit.toggle-live',
      keys: 'p',
      label: 'Pause or resume live telemetry',
      group: 'Actions',
      when: () => inTauri(),
    },
    toggleLive,
  )

  if (status === 'detached') {
    return (
      <ChartFrame>
        <ChartHead procedure="Cockpit" context="no backend" />
        <div className="py-3 font-mono text-xs">
          <p className="flex items-baseline gap-2">
            <Glyph mark="absent" />
            <span>No telemetry. This window is running outside the desktop shell.</span>
          </p>
          <p className="mt-1 pl-6 text-ink-soft">Nothing is shown in its place.</p>
        </div>
        <ChartFoot finding="Vital signs are read by the desktop app." next="synapse hud --once" />
      </ChartFrame>
    )
  }

  // The trace means "readings are arriving". When the gateway is measured it
  // also requires the gateway to be online; otherwise only Brain reads count.
  const beating =
    status === 'ok' &&
    reading !== null &&
    (!gatewayMeasured || gatewayMark(reading.gateway_status) === 'present')
  const trace: TraceMode = !beating ? 'flat' : live ? 'beat' : 'frozen'

  const context =
    status === 'loading'
      ? 'reading…'
      : [
          readAt ? `read ${fmtClock(readAt)}` : 'never read',
          live ? `every ${POLL_MS / 1000} s` : 'paused',
        ].join(' · ')

  return (
    <ChartFrame>
      <ChartHead procedure="Cockpit" context={<span className="tabular-nums">{context}</span>} />

      {/* Controls + monitor state */}
      <div className="flex flex-wrap items-center justify-between gap-3 py-3 font-mono text-xs">
        <p className="flex items-baseline gap-2" aria-live="polite">
          <MonitorState status={status} live={live} beating={beating} reading={reading} />
        </p>
        <div className="flex items-center gap-4">
          <button
            type="button"
            onClick={toggleLive}
            aria-pressed={!live}
            aria-keyshortcuts="P"
            title={live ? 'Pause live telemetry (P)' : 'Resume live telemetry (P)'}
            className="text-drape hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape"
          >
            {live ? 'pause' : 'resume'}{' '}
            <span aria-hidden className="text-ink-soft">
              [P]
            </span>
          </button>
          <ReadButton busy={busy} onRead={read} idle="read now" pending="reading…" />
        </div>
      </div>

      <VitalsTrace mode={trace} />

      {status === 'error' && (
        <div role="alert" className="mt-3 flex items-baseline gap-2 font-mono text-xs text-alarm">
          <Glyph mark="critical" />
          <p className="min-w-0 flex-1 break-words">
            Telemetry read failed: {error}
            {reading && <span className="text-ink-soft"> · last good reading shown below</span>}
          </p>
          <ReadButton busy={busy} onRead={read} idle="retry" pending="retrying…" className="shrink-0" />
        </div>
      )}

      {reading ? (
        <>
          <div className="mt-4 grid grid-cols-1 border-t border-rule md:grid-cols-3 md:divide-x md:divide-rule">
            <section aria-labelledby="ck-gateway" className="py-3 md:pr-4">
              {gatewayMeasured ? (
                <>
                  <PanelHead id="ck-gateway" title="Runtime gateway" mark={gatewayMark(reading.gateway_status)}>
                    {reading.gateway_status ?? 'not reported'}
                  </PanelHead>
                  <dl>
                    <Readout label="Upstream" value={reading.upstream_url} />
                    <Readout label="Model" value={reading.model} />
                    <Readout label="Latency" value={fmtMs(reading.latency_ms)} />
                    <Readout label="Requests" value={fmtInt(reading.requests_handled)} />
                  </dl>
                </>
              ) : (
                <NotMeasured id="ck-gateway" title="Runtime gateway" />
              )}
              <Hint id="cockpit.gateway">
                Latency is the gateway's round trip to the upstream model; requests counts prompts it has relayed
                since it started.
              </Hint>
            </section>

            <section aria-labelledby="ck-gating" className="border-t border-rule py-3 md:border-t-0 md:px-4">
              {gatewayMeasured ? (
                <>
                  <PanelHead
                    id="ck-gating"
                    title="Sensory gating"
                    mark={reading.reduction_pct === null ? 'absent' : 'present'}
                  >
                    {reading.reduction_pct === null ? 'not reported' : `−${fmtPct(reading.reduction_pct)}`}
                  </PanelHead>
                  <dl>
                    <Readout label="Raw context" value={fmtInt(reading.raw_prompt_tokens)} />
                    <Readout label="Gated context" value={fmtInt(reading.gated_prompt_tokens)} />
                    <Readout label="Tokens pruned" value={fmtInt(reading.tokens_saved)} />
                    <Readout label="Reduction" value={fmtPct(reading.reduction_pct)} />
                  </dl>
                </>
              ) : (
                <NotMeasured id="ck-gating" title="Sensory gating" />
              )}
              <Hint id="cockpit.gating">
                Raw is the prompt as assembled, gated is what reaches the model after compression. Reduction is
                pruned over raw.
              </Hint>
            </section>

            <section aria-labelledby="ck-brain" className="border-t border-rule py-3 md:border-t-0 md:pl-4">
              <PanelHead id="ck-brain" title="Brain health" mark={brainMark(reading)}>
                {reading.adapters_present === null || reading.adapters_total === null
                  ? 'grafts not reported'
                  : `${reading.adapters_present}/${reading.adapters_total} grafts`}
              </PanelHead>
              <dl>
                <Readout label="Brain" value={reading.brain_path} />
                <Readout
                  label="Criticals"
                  value={fmtInt(reading.doctor_criticals)}
                  tone={(reading.doctor_criticals ?? 0) > 0 ? 'critical' : undefined}
                />
                <Readout
                  label="Auto-fixable"
                  value={fmtInt(reading.doctor_fixable)}
                  tone={(reading.doctor_fixable ?? 0) > 0 ? 'warning' : undefined}
                />
              </dl>
            </section>
          </div>

          <ChartFoot finding={finding(reading, gatewayMeasured)} next={nextCommand(reading, gatewayMeasured)} />
          <Hint id="cockpit.next">
            The same readings are available in a terminal with synapse hud; press P here to pause live updates.
          </Hint>
        </>
      ) : status === 'error' ? (
        <ChartFoot finding="No reading yet." next="synapse doctor" />
      ) : null}
    </ChartFrame>
  )
}

/**
 * A read trigger. While a read is in flight it is `aria-disabled` rather than
 * `disabled`: a natively disabled button drops keyboard focus to <body> the
 * moment it is pressed. The click is still ignored (read() never overlaps).
 */
function ReadButton({
  busy,
  onRead,
  idle,
  pending,
  className,
}: {
  busy: boolean
  onRead: () => Promise<void>
  idle: string
  pending: string
  className?: string
}) {
  return (
    <button
      type="button"
      onClick={() => {
        if (!busy) void onRead()
      }}
      aria-disabled={busy}
      title={busy ? 'A telemetry read is already in progress' : undefined}
      className={`${className ?? ''} text-drape hover:text-ink aria-disabled:cursor-default aria-disabled:text-ink-soft focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape`}
    >
      {busy ? pending : idle}
    </button>
  )
}

const MARK_WORD: Record<Mark, string> = {
  present: 'ok',
  partial: 'partial',
  warning: 'warning',
  critical: 'critical',
  absent: 'not reported',
  na: 'not applicable',
}

function PanelHead({
  id,
  title,
  mark,
  children,
}: {
  id: string
  title: string
  mark: Mark
  children: ReactNode
}) {
  return (
    <div className="mb-1 flex items-baseline justify-between gap-2 border-b border-rule pb-1.5 font-mono text-xs">
      <h2 id={id} className="font-semibold uppercase tracking-[0.1em] text-ink">
        {title}
      </h2>
      <span className="flex items-baseline gap-1 tabular-nums">
        <Glyph mark={mark} />
        {mark !== 'absent' && <span className="sr-only">Status: {MARK_WORD[mark]}.</span>}
        <span className={mark === 'absent' ? 'text-ink-soft' : 'text-ink'}>{children}</span>
      </span>
    </div>
  )
}

/** A panel whose fields this build does not measure: said once, no numbers shown. */
function NotMeasured({ id, title }: { id: string; title: string }) {
  return (
    <>
      <PanelHead id={id} title={title} mark="absent">
        not measured
      </PanelHead>
      <p className="py-1 font-mono text-xs text-ink-soft">
        Not measured in this build. The backend returns fixed values here, so none are shown.
      </p>
    </>
  )
}

function MonitorState({
  status,
  live,
  beating,
  reading,
}: {
  status: Status
  live: boolean
  beating: boolean
  reading: Reading | null
}) {
  if (status === 'loading')
    return (
      <>
        <Glyph mark="na" />
        <span className="text-ink-soft">Reading vital signs…</span>
      </>
    )
  if (status === 'error')
    return (
      <>
        <Glyph mark="critical" />
        <span className="text-alarm">Signal lost</span>
      </>
    )
  if (!beating)
    return (
      <>
        <Glyph mark={reading?.gateway_status == null ? 'absent' : 'critical'} />
        <span className="text-ink-soft">Flat trace: gateway not online</span>
      </>
    )
  if (!live)
    return (
      <>
        <Glyph mark="partial" />
        <span className="text-ink-soft">Paused · showing last reading</span>
      </>
    )
  return (
    <>
      <span className="motion-safe:animate-pulse">
        <Glyph mark="present" />
      </span>
      <span className="text-success">Live</span>
    </>
  )
}

// ── EKG trace ──────────────────────────────────────────────────────────────

type TraceMode = 'beat' | 'frozen' | 'flat'

function cssVar(name: string, fallback: string): string {
  if (typeof window === 'undefined') return fallback
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

/** Height offset (CSS px, positive = up) of the P-QRS-T complex at `phase` px into a beat. */
export function beatAt(phase: number): number {
  const c = ((phase % BEAT_PX) + BEAT_PX) % BEAT_PX
  if (c > 60 && c < 75) return Math.sin(((c - 60) / 15) * Math.PI) * 12 // P
  if (c >= 75 && c < 85) return -8 // Q
  if (c >= 85 && c < 95) return 45 // R
  if (c >= 95 && c < 105) return -18 // S
  if (c >= 115 && c < 145) return Math.sin(((c - 115) / 30) * Math.PI) * 16 // T
  return 0
}

function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false
}

/**
 * The scrolling vitals trace. It is a status display, not a measurement: it
 * beats only while live readings arrive from an online gateway.
 *
 * - Time-based scroll (tokens `ekgScroll`), so speed is the same at 60 or 144 Hz.
 * - rAF stops while the document is hidden, when paused, and under
 *   prefers-reduced-motion (a single static frame is drawn instead).
 * - Backing store tracks the element's size and devicePixelRatio.
 */
function VitalsTrace({ mode }: { mode: TraceMode }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const offset = useRef(0)
  const [reduced, setReduced] = useState(prefersReducedMotion)

  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)')
    const onChange = () => setReduced(mq.matches)
    mq.addEventListener?.('change', onChange)
    return () => mq.removeEventListener?.('change', onChange)
  }, [])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const colours = {
      bg: cssVar('--ground', '#0b0e13'),
      grid: cssVar('--rule', '#262c37'),
      beat: cssVar('--success', '#3fb950'),
      flat: cssVar('--ink-soft', '#8b949e'),
    }
    let w = 0
    let h = 0

    const fit = () => {
      const dpr = window.devicePixelRatio || 1
      w = canvas.clientWidth || canvas.width
      h = canvas.clientHeight || canvas.height
      const bw = Math.max(1, Math.round(w * dpr))
      const bh = Math.max(1, Math.round(h * dpr))
      if (canvas.width !== bw || canvas.height !== bh) {
        canvas.width = bw
        canvas.height = bh
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    }

    const draw = () => {
      if (w <= 0 || h <= 0) return
      ctx.fillStyle = colours.bg
      ctx.fillRect(0, 0, w, h)

      ctx.strokeStyle = colours.grid
      ctx.lineWidth = 1
      ctx.beginPath()
      for (let x = 0.5; x < w; x += 20) {
        ctx.moveTo(x, 0)
        ctx.lineTo(x, h)
      }
      for (let y = 0.5; y < h; y += 20) {
        ctx.moveTo(0, y)
        ctx.lineTo(w, y)
      }
      ctx.stroke()

      const mid = h / 2
      const scale = Math.min(1, h / 120)
      ctx.strokeStyle = mode === 'flat' ? colours.flat : colours.beat
      ctx.lineWidth = 1.5
      ctx.beginPath()
      for (let x = 0; x <= w; x++) {
        const y = mode === 'flat' ? mid : mid - beatAt(x + offset.current) * scale
        if (x === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      ctx.stroke()
    }

    fit()
    draw()

    let ro: ResizeObserver | undefined
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(() => {
        fit()
        draw()
      })
      ro.observe(canvas)
    }

    const animate = mode === 'beat' && !reduced
    let raf: number | undefined
    let last: number | undefined
    const pxPerMs = () => Math.max(w, BEAT_PX) / EKG_SCROLL_MS

    const frame = (t: number) => {
      if (last !== undefined) offset.current = (offset.current + (t - last) * pxPerMs()) % (BEAT_PX * 1000)
      last = t
      draw()
      raf = requestAnimationFrame(frame)
    }
    const start = () => {
      if (!animate || raf !== undefined || document.hidden) return
      last = undefined
      raf = requestAnimationFrame(frame)
    }
    const stop = () => {
      if (raf !== undefined) cancelAnimationFrame(raf)
      raf = undefined
    }
    const onVisibility = () => (document.hidden ? stop() : start())

    start()
    document.addEventListener('visibilitychange', onVisibility)
    return () => {
      stop()
      ro?.disconnect()
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [mode, reduced])

  const label =
    mode === 'beat'
      ? 'Vitals trace, beating: live readings arriving'
      : mode === 'frozen'
        ? 'Vitals trace, paused'
        : 'Vitals trace, flat: no live reading'

  return (
    <canvas
      ref={canvasRef}
      role="img"
      aria-label={label}
      data-mode={mode}
      width={800}
      height={112}
      className="block h-28 w-full border border-rule bg-ground"
    />
  )
}
