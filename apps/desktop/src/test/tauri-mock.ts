/**
 * A fake Tauri 2 backend for running the desktop frontend in a plain browser
 * (Playwright e2e, or a manual `vite` session).
 *
 * It installs `window.__TAURI_INTERNALS__` (what `@tauri-apps/api/core`'s
 * `invoke` and `transformCallback` call into) and
 * `window.__TAURI_EVENT_PLUGIN_INTERNALS__` (what `listen`'s unlisten uses),
 * answering every command registered in `src-tauri/src/lib.rs` with fixture
 * data shaped like `src/screens/types.ts` / `src-tauri/src/commands.rs`, plus
 * the `plugin:window|*` and `plugin:event|*` calls a custom title bar makes.
 *
 * Every call is logged to `window.__TAURI_MOCK__.calls`; unknown commands
 * reject and are logged to `window.__TAURI_MOCK__.failures`.
 *
 * This file must stay self-contained (no imports): the e2e runner transpiles
 * it with esbuild to an IIFE and injects it with `page.addInitScript` before
 * the app loads, so no app source changes are needed.
 *
 *   installTauriMock(window, { scenario: 'faulty' })
 */

export type MockScenario = 'clean' | 'faulty'

export interface MockOptions {
  scenario?: MockScenario
  /** commands that should reject (to exercise error paths) */
  failCommands?: string[]
  /** artificial latency in ms for every invoke */
  latencyMs?: number
}

export interface MockCall {
  cmd: string
  args: unknown
  t: number
  ok?: boolean
  error?: string
}

export interface TauriMockState {
  calls: MockCall[]
  failures: MockCall[]
  window: { maximized: boolean; minimized: boolean; closed: boolean }
  /** fire a fake window event (e.g. 'tauri://resize') to registered listeners */
  emit: (event: string, payload?: unknown) => void
  options: MockOptions
}

const SITE = '/home/mara/work/atlas'
const BRAIN = '/home/mara/AIBrain'

function intakeFixture() {
  const findings = [
    { id: 'claude-code', present: true, skills: 12, agents: 4, mcp_servers: 3, error: null },
    { id: 'cursor', present: true, skills: 0, agents: null, mcp_servers: 2, error: null },
    { id: 'cline', present: true, skills: 6, agents: 0, mcp_servers: 0, error: null },
    { id: 'gemini-cli', present: true, skills: null, agents: null, mcp_servers: 1, error: null },
    {
      id: 'github-copilot',
      present: true,
      skills: null,
      agents: null,
      mcp_servers: null,
      error: 'malformed config: not valid UTF-8',
    },
    { id: 'openai-codex', present: false, skills: null, agents: null, mcp_servers: null, error: null },
    { id: 'roo-code', present: false, skills: null, agents: null, mcp_servers: null, error: null },
    { id: 'windsurf', present: false, skills: null, agents: null, mcp_servers: null, error: null },
    { id: 'aider', present: false, skills: null, agents: null, mcp_servers: null, error: null },
    { id: 'zed', present: false, skills: null, agents: null, mcp_servers: null, error: null },
  ]
  return { site: SITE, total: findings.length, present: findings.filter((f) => f.present).length, findings }
}

function examFixture(scenario: MockScenario, healed: boolean) {
  if (scenario === 'clean' || healed) {
    return { brain: BRAIN, tools: SITE, criticals: 0, fixable: 0, findings: [] }
  }
  const findings = [
    {
      rule_id: 'master-prompt-tampered',
      severity: 'critical',
      message: 'Zone 1 checksum does not match the sealed master prompt',
      subject: `${BRAIN}/SYNAPSE.md`,
      auto_fixable: true,
    },
    {
      rule_id: 'master-prompt-missing',
      severity: 'critical',
      message: 'No master prompt found for this tool root',
      subject: `${SITE}/CLAUDE.md`,
      auto_fixable: false,
    },
    {
      rule_id: 'symlink-retargeted',
      severity: 'warning',
      message: 'Projection points somewhere other than the canonical source',
      subject: `${SITE}/.cursor/rules/brain.mdc`,
      auto_fixable: true,
    },
    {
      rule_id: 'brain-not-git-repo',
      severity: 'info',
      message: 'The brain directory is not under version control',
      subject: BRAIN,
      auto_fixable: false,
    },
  ]
  return {
    brain: BRAIN,
    tools: SITE,
    criticals: findings.filter((f) => f.severity === 'critical').length,
    fixable: findings.filter((f) => f.auto_fixable).length,
    findings,
  }
}

function memoryFixture() {
  const now = 1_794_380_000_000
  const n = (id: string, label: string, kind: string, narrative: string, files: string[], val: number, i: number) => ({
    id,
    label,
    kind,
    narrative,
    files,
    ts: now - i * 3_600_000,
    val,
  })
  return {
    nodes: [
      n('core', 'AIBrain', 'core', 'Canonical brain root.', ['SYNAPSE.md'], 1.0, 0),
      n('proj-atlas', 'atlas', 'project', 'Project memory for atlas.', ['atlas/'], 0.8, 1),
      n('obs-1', 'Prefers pnpm', 'observation', 'User consistently uses pnpm workspaces.', [], 0.62, 2),
      n('obs-2', 'Strict TDD', 'observation', 'Tests are written before implementation.', ['tests/'], 0.71, 3),
      n('file-1', 'commands.rs', 'file', 'Tauri command surface.', ['src-tauri/src/commands.rs'], 0.55, 4),
      n('chg-1', 'Renamed to Plexus', 'change', 'Product rename recorded.', ['README.md'], 0.4, 5),
      n('disc-1', 'Doctor rule 15', 'discovery', 'Master prompt integrity check.', ['doctor.rs'], 0.66, 6),
    ],
    edges: [
      { source: 'core', target: 'proj-atlas', label: 'contains' },
      { source: 'proj-atlas', target: 'obs-1', label: 'observed' },
      { source: 'proj-atlas', target: 'obs-2', label: 'observed' },
      { source: 'proj-atlas', target: 'file-1', label: 'touches' },
      { source: 'file-1', target: 'chg-1', label: 'changed' },
      { source: 'core', target: 'disc-1', label: 'discovered' },
    ],
    total_observations: 2,
  }
}

function spoolFixture() {
  const base = 1_794_380_000 // unix seconds (2026-11-11)
  const longLog = [
    '$ cargo test --workspace',
    '   Compiling synapse-core v1.0.0 (/home/mara/work/atlas/packages/core)',
    ...Array.from({ length: 140 }, (_, i) =>
      i % 23 === 7
        ? `test doctor::rules::r${String(i).padStart(3, '0')} ... FAILED`
        : `test doctor::rules::r${String(i).padStart(3, '0')} ... ok`,
    ),
    '',
    'failures:',
    '---- doctor::rules::r007 stdout ----',
    "thread 'doctor::rules::r007' panicked at packages/core/src/doctor.rs:412:9:",
    'assertion `left == right` failed',
    '  left: 2',
    ' right: 3',
    '',
    'test result: FAILED. 133 passed; 7 failed; 0 ignored; finished in 4.21s',
  ].join('\n')
  const entries = [
    { id: '20261111-052104-cargo-test', timestamp: base + 900, command: 'cargo test --workspace', text: longLog },
    {
      id: '20261111-050512-pnpm-build',
      timestamp: base + 300,
      command: 'pnpm -C apps/desktop build',
      text: '$ pnpm -C apps/desktop build\nvite v6.4.3 building for production...\n✓ 1532 modules transformed.\ndist/index.html  0.46 kB\ndist/assets/index.js  812.40 kB │ gzip: 231.10 kB\n✓ built in 6.12s',
    },
    {
      id: '20261111-044830-unknown',
      timestamp: base,
      command: null,
      text: 'npm WARN deprecated inflight@1.0.6\nadded 412 packages in 9s',
    },
  ]
  return entries.map((e) => ({
    ...e,
    raw_bytes: new TextEncoder().encode(e.text).length,
    log_path: `/home/mara/AIBrain/.brain/spool/${e.id}.log`,
  }))
}

function cockpitFixture(scenario: MockScenario, healed: boolean) {
  const exam = examFixture(scenario, healed)
  const intake = intakeFixture()
  return {
    gateway_status: 'ONLINE',
    upstream_url: 'http://127.0.0.1:11434 (Ollama)',
    model: 'qwen2.5-coder:7b',
    latency_ms: 12,
    requests_handled: 18,
    raw_prompt_tokens: 184_600,
    gated_prompt_tokens: 11_200,
    tokens_saved: 173_400,
    reduction_pct: 93.9,
    brain_path: BRAIN,
    adapters_present: intake.present,
    adapters_total: intake.total,
    doctor_criticals: exam.criticals,
    doctor_fixable: exam.fixable,
  }
}

export function installTauriMock(w: Window & typeof globalThis, options: MockOptions = {}): TauriMockState {
  const scenario: MockScenario = options.scenario ?? 'faulty'
  const fail = new Set(options.failCommands ?? [])
  let healed = false
  let nextCb = 1
  let nextEvent = 1
  const callbacks = new Map<number, (payload: unknown) => void>()
  const listeners = new Map<number, { event: string; handler: number }>()

  const state: TauriMockState = {
    calls: [],
    failures: [],
    window: { maximized: false, minimized: false, closed: false },
    options,
    emit(event, payload) {
      for (const [id, l] of listeners) {
        if (l.event === event) callbacks.get(l.handler)?.({ event, id, payload })
      }
    },
  }

  const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
    get_version: () => '1.0.0',
    intake: () => intakeFixture(),
    examine: () => examFixture(scenario, healed),
    heal_vitals: () => {
      const fixed = examFixture(scenario, healed).fixable
      healed = true
      return fixed
    },
    get_memory_graph: () => memoryFixture(),
    get_cockpit_telemetry: () => cockpitFixture(scenario, healed),
    check_for_update: () => ({
      decision: { status: 'up_to_date', current: '1.0.0' },
      report: 'synapse 1.0.0 is the newest build on the stable channel.',
      signing: 'not_configured',
    }),
    // Spool: SpoolEntryDto[] (src/screens/Spool.tsx), newest first
    spool_list: () => spoolFixture().map(({ text: _t, ...e }) => e),
    spool_show: (args) => {
      const hit = spoolFixture().find((e) => e.id === args.id)
      if (!hit) throw `no spool entry ${String(args.id)}`
      return hit.text
    },

    // DryRunReport { root, detected: DetectedTool[] } — counts are plain usize, not Option
    scan_dry_run: () => ({
      root: SITE,
      detected: intakeFixture()
        .findings.filter((f) => f.present)
        .map((f) => ({
          tool_id: f.id,
          skills: f.skills ?? 0,
          agents: f.agents ?? 0,
          mcp_servers: f.mcp_servers ?? 0,
          error: f.error,
        })),
    }),

    // @tauri-apps/api/window
    'plugin:window|minimize': () => {
      state.window.minimized = true
      return null
    },
    'plugin:window|unminimize': () => {
      state.window.minimized = false
      return null
    },
    'plugin:window|maximize': () => {
      state.window.maximized = true
      state.emit('tauri://resize', { width: 1600, height: 1000 })
      return null
    },
    'plugin:window|unmaximize': () => {
      state.window.maximized = false
      state.emit('tauri://resize', { width: 1280, height: 800 })
      return null
    },
    'plugin:window|toggle_maximize': () => {
      state.window.maximized = !state.window.maximized
      state.emit('tauri://resize', state.window.maximized ? { width: 1600, height: 1000 } : { width: 1280, height: 800 })
      return null
    },
    'plugin:window|is_maximized': () => state.window.maximized,
    'plugin:window|is_minimized': () => state.window.minimized,
    'plugin:window|is_fullscreen': () => false,
    'plugin:window|is_focused': () => true,
    'plugin:window|is_decorated': () => false,
    'plugin:window|close': () => {
      state.window.closed = true
      return null
    },
    'plugin:window|destroy': () => {
      state.window.closed = true
      return null
    },
    'plugin:window|start_dragging': () => null,
    'plugin:window|set_title': () => null,
    'plugin:window|set_focus': () => null,
    'plugin:window|theme': () => 'dark',
    'plugin:window|scale_factor': () => 1,
    'plugin:window|inner_size': () => ({ width: 1280, height: 800 }),
    'plugin:window|outer_size': () => ({ width: 1280, height: 800 }),
    'plugin:window|title': () => 'Synapse',

    // @tauri-apps/api/event
    'plugin:event|listen': (args) => {
      const id = nextEvent++
      listeners.set(id, { event: String(args.event), handler: Number(args.handler) })
      return id
    },
    'plugin:event|unlisten': (args) => {
      listeners.delete(Number(args.eventId))
      return null
    },
    'plugin:event|emit': () => null,
    'plugin:event|emit_to': () => null,
  }

  const internals = {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { windowLabel: 'main', label: 'main' },
    },
    plugins: {},
    transformCallback(cb: (payload: unknown) => void, once = false): number {
      const id = nextCb++
      callbacks.set(id, (p) => {
        if (once) callbacks.delete(id)
        cb(p)
      })
      return id
    },
    unregisterCallback(id: number) {
      callbacks.delete(id)
    },
    convertFileSrc(path: string, protocol = 'asset') {
      return `${protocol}://localhost/${encodeURIComponent(path)}`
    },
    async invoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
      const call: MockCall = { cmd, args: JSON.parse(JSON.stringify(args ?? {})), t: Date.now() }
      state.calls.push(call)
      if (options.latencyMs) await new Promise((r) => setTimeout(r, options.latencyMs))
      const h: ((args: Record<string, unknown>) => unknown) | undefined = Object.prototype.hasOwnProperty.call(handlers, cmd)
        ? handlers[cmd]
        : undefined
      if (!h || fail.has(cmd)) {
        call.ok = false
        call.error = h ? `mock: ${cmd} forced to fail` : `mock: unknown command ${cmd}`
        state.failures.push(call)
        throw call.error
      }
      call.ok = true
      return h(args ?? {})
    },
  }

  const eventInternals = {
    unregisterListener(_event: string, id: number) {
      listeners.delete(id)
    },
  }

  Object.defineProperty(w, '__TAURI_INTERNALS__', { value: internals, configurable: true, writable: true })
  Object.defineProperty(w, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
    value: eventInternals,
    configurable: true,
    writable: true,
  })
  ;(w as unknown as { __TAURI_MOCK__: TauriMockState }).__TAURI_MOCK__ = state
  return state
}
