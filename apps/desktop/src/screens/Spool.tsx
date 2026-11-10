import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Hint } from '../lib/hints'
import { formatKeys, useShortcut } from '../lib/shortcuts'

/**
 * Sensory Spool: the raw command output that sensory gating kept back from
 * the agent, read from the spool on disk.
 *
 * Everything shown is measured from what the backend returns. The core's
 * spool index does not record the gated text or a real compressed size, so
 * the gated side is reported as absent rather than estimated.
 */

/**
 * One spooled log as returned by `spool_list`. Mirrors the measured fields of
 * core's `SpoolEntry`; `command` is optional because core currently stores a
 * placeholder there.
 */
export interface SpoolEntryDto {
  id: string
  /** Unix seconds (file mtime). */
  timestamp: number
  raw_bytes: number
  log_path: string
  command?: string | null
}

type ViewMode = 'side-by-side' | 'raw' | 'gated'

type ListState =
  | { kind: 'outside' }
  | { kind: 'loading' }
  | { kind: 'unwired'; message: string }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; entries: SpoolEntryDto[] }

type LogState =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; text: string }

/** Lines rendered for one log; the rest stay in memory and in "Copy full log". */
export const MAX_LINES = 2000
/** Rows rendered in the run list. */
export const MAX_ENTRIES = 200

const VIEWS: { id: ViewMode; label: string }[] = [
  { id: 'side-by-side', label: 'Side by side' },
  { id: 'raw', label: 'Raw' },
  { id: 'gated', label: 'Gated' },
]

const KEYS_VIEW = 'v'
const KEYS_RELOAD = 'shift+r'

const FOCUS =
  'focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-drape'

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function errorText(e: unknown): string {
  if (e instanceof Error) return e.message
  if (typeof e === 'string') return e
  try {
    return JSON.stringify(e)
  } catch {
    return String(e)
  }
}

/** Tauri rejects an unregistered command with "command <name> not found". */
function isUnwired(message: string): boolean {
  return /command\s+\S+\s+not found|unknown command/i.test(message)
}

/** Same rule as core's `estimate_tokens`: characters ÷ 4, rounded up. */
export function estimateTokens(text: string): number {
  if (!text) return 0
  return Math.ceil([...text].length / 4)
}

function fmtInt(n: number): string {
  return n.toLocaleString('en-US')
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${fmtInt(n)} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`
  return `${(n / (1024 * 1024)).toFixed(1)} MiB`
}

function fmtTime(secs: number): string {
  if (!Number.isFinite(secs) || secs <= 0) return 'time not recorded'
  const d = new Date(secs * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

/** Last `max` lines of `text`, with the total line count. */
export function tailLines(text: string, max: number): { shown: string; shownCount: number; total: number } {
  const lines = text.split('\n')
  const total = lines.length
  if (total <= max) return { shown: text, shownCount: total, total }
  return { shown: lines.slice(total - max).join('\n'), shownCount: max, total }
}

async function copyText(text: string): Promise<void> {
  const clip = typeof navigator !== 'undefined' ? navigator.clipboard : undefined
  if (!clip || typeof clip.writeText !== 'function') {
    throw new Error('clipboard is not available in this window')
  }
  await clip.writeText(text)
}

export function Spool() {
  const [list, setList] = useState<ListState>(() => (inTauri() ? { kind: 'loading' } : { kind: 'outside' }))
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [log, setLog] = useState<LogState>({ kind: 'idle' })
  const [view, setView] = useState<ViewMode>('side-by-side')
  const [copyNote, setCopyNote] = useState<{ ok: boolean; text: string } | null>(null)
  const listSeq = useRef(0)
  const logSeq = useRef(0)
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])

  const loadList = useCallback(async () => {
    if (!inTauri()) {
      setList({ kind: 'outside' })
      return
    }
    const seq = ++listSeq.current
    setList({ kind: 'loading' })
    try {
      const entries = await invoke<SpoolEntryDto[]>('spool_list')
      if (!mounted.current || seq !== listSeq.current) return
      const safe = Array.isArray(entries) ? entries : []
      setList({ kind: 'ready', entries: safe })
      setSelectedId((cur) => (cur && safe.some((e) => e.id === cur) ? cur : (safe[0]?.id ?? null)))
    } catch (e) {
      if (!mounted.current || seq !== listSeq.current) return
      const message = errorText(e)
      setList(isUnwired(message) ? { kind: 'unwired', message } : { kind: 'error', message })
    }
  }, [])

  useEffect(() => {
    void loadList()
  }, [loadList])

  useEffect(() => {
    setCopyNote(null)
    if (!selectedId) {
      setLog({ kind: 'idle' })
      return
    }
    const seq = ++logSeq.current
    setLog({ kind: 'loading' })
    invoke<string>('spool_show', { id: selectedId })
      .then((text) => {
        if (!mounted.current || seq !== logSeq.current) return
        setLog({ kind: 'ready', text: typeof text === 'string' ? text : '' })
      })
      .catch((e) => {
        if (!mounted.current || seq !== logSeq.current) return
        setLog({ kind: 'error', message: errorText(e) })
      })
  }, [selectedId])

  const cycleView = useCallback(() => {
    setView((v) => VIEWS[(VIEWS.findIndex((x) => x.id === v) + 1) % VIEWS.length].id)
  }, [])

  // Scoped: registered only while this screen is mounted.
  useShortcut({ id: 'spool.view', keys: KEYS_VIEW, label: 'Cycle raw / gated view', group: 'View' }, cycleView)
  useShortcut({ id: 'spool.reload', keys: KEYS_RELOAD, label: 'Re-read the spool', group: 'Actions' }, () => {
    void loadList()
  })

  const doCopy = async (what: string, text: string) => {
    try {
      await copyText(text)
      if (mounted.current) setCopyNote({ ok: true, text: `Copied ${what}.` })
    } catch (e) {
      if (mounted.current) setCopyNote({ ok: false, text: `Copy failed: ${errorText(e)}` })
    }
  }

  const entries = list.kind === 'ready' ? list.entries : []
  const selected = entries.find((e) => e.id === selectedId) ?? null
  const shownEntries = entries.slice(0, MAX_ENTRIES)

  return (
    <div className="space-y-6">
      <header className="border-b border-rule pb-3">
        <h2 className="font-display text-base uppercase tracking-wider text-drape">Sensory Spool</h2>
        <p className="font-mono text-xs text-ink-soft">Raw command output kept back by sensory gating, read from disk.</p>
        <Hint id="spool.gating">
          Sensory gating hands the agent a short summary of long command output and keeps the full output here, so
          nothing is lost.
        </Hint>
      </header>

      {list.kind === 'outside' && (
        <Absent
          title="No backend attached."
          body="This window is running outside the desktop shell, so there is no spool to read. Nothing is shown in its place."
          next="synapse spool list"
        />
      )}

      {list.kind === 'loading' && <p className="font-mono text-xs text-ink-soft">reading spool…</p>}

      {list.kind === 'unwired' && (
        <Absent
          title="Spool not readable from the desktop yet."
          body={`This build has no spool reader (${list.message}). The CLI can read it.`}
          next="synapse spool list"
        />
      )}

      {list.kind === 'error' && (
        <div role="alert">
          <p className="font-mono text-sm text-alarm">
            <span aria-hidden="true">▲ </span>Spool could not be read.
          </p>
          <p className="mt-2 break-words font-mono text-xs text-ink-soft">{list.message}</p>
          <RetryLine onRetry={() => void loadList()} />
        </div>
      )}

      {list.kind === 'ready' && entries.length === 0 && (
        <Absent
          title="Spool is empty."
          body="No command output has been spooled yet. Output is spooled when a command runs through sensory gating."
          next="synapse squash <command>"
        />
      )}

      {list.kind === 'ready' && entries.length > 0 && (
        <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
          <nav aria-label="Spooled runs" className="border border-rule bg-ground">
            <div className="flex items-center justify-between border-b border-rule px-3 py-2">
              <p className="font-mono text-[11px] font-bold uppercase tracking-wider text-drape">Spooled runs</p>
              <p className="font-mono text-[11px] tabular-nums text-ink-soft">{fmtInt(entries.length)}</p>
            </div>
            <ul className="max-h-[32rem] divide-y divide-rule overflow-y-auto">
              {shownEntries.map((run) => {
                const active = run.id === selectedId
                return (
                  <li key={run.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(run.id)}
                      aria-current={active ? 'true' : undefined}
                      className={`block w-full border-l-2 px-3 py-2 text-left font-mono text-xs ${FOCUS} ${
                        active ? 'border-drape bg-chart text-ink' : 'border-transparent text-ink-soft hover:text-ink'
                      }`}
                    >
                      <span className="block truncate text-ink">{run.command?.trim() || run.id}</span>
                      <span className="mt-0.5 flex justify-between gap-2 text-[11px] tabular-nums text-ink-soft">
                        <span>{run.id}</span>
                        <span>{fmtBytes(run.raw_bytes)}</span>
                      </span>
                      <span className="block text-[11px] tabular-nums text-ink-soft">{fmtTime(run.timestamp)}</span>
                    </button>
                  </li>
                )
              })}
            </ul>
            {entries.length > MAX_ENTRIES && (
              <p className="border-t border-rule px-3 py-2 font-mono text-[11px] tabular-nums text-ink-soft">
                showing {fmtInt(MAX_ENTRIES)} of {fmtInt(entries.length)} runs, newest first
              </p>
            )}
          </nav>

          <section aria-label="Spool inspector" className="space-y-4 border border-rule bg-chart p-4 md:col-span-2">
            {selected ? (
              <Inspector
                entry={selected}
                log={log}
                view={view}
                setView={setView}
                onCopy={doCopy}
                copyNote={copyNote}
              />
            ) : (
              <p className="font-mono text-xs text-ink-soft">Select a run to inspect it.</p>
            )}
          </section>
        </div>
      )}
    </div>
  )
}

function Inspector({
  entry,
  log,
  view,
  setView,
  onCopy,
  copyNote,
}: {
  entry: SpoolEntryDto
  log: LogState
  view: ViewMode
  setView: (v: ViewMode) => void
  onCopy: (what: string, text: string) => Promise<void>
  copyNote: { ok: boolean; text: string } | null
}) {
  const text = log.kind === 'ready' ? log.text : null
  const tokens = text !== null ? estimateTokens(text) : null
  const tail = text !== null ? tailLines(text, MAX_LINES) : null

  return (
    <>
      <div className="flex flex-wrap items-start justify-between gap-3 border-b border-rule pb-3">
        <div className="min-w-0">
          <h3 className="truncate font-mono text-sm font-semibold text-ink">{entry.command?.trim() || entry.id}</h3>
          <p className="font-mono text-[11px] tabular-nums text-ink-soft">
            {entry.id} · {fmtTime(entry.timestamp)} · {fmtBytes(entry.raw_bytes)} raw
            {tokens !== null && <> · ≈ {fmtInt(tokens)} tokens</>}
          </p>
          <p className="mt-1 break-all font-mono text-[11px] text-ink-soft">{entry.log_path}</p>
        </div>

        <div role="group" aria-label="View" className="flex items-center">
          {VIEWS.map((v) => (
            <button
              key={v.id}
              type="button"
              aria-pressed={view === v.id}
              onClick={() => setView(v.id)}
              className={`-ml-px border px-2.5 py-1 font-mono text-xs first:ml-0 ${FOCUS} ${
                view === v.id ? 'relative border-drape text-drape' : 'border-rule text-ink-soft hover:text-ink'
              }`}
            >
              {v.label}
            </button>
          ))}
          <kbd className="ml-2 font-mono text-[10px] text-ink-soft">{formatKeys(KEYS_VIEW).join(' ')}</kbd>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2 font-mono text-xs">
        <button
          type="button"
          onClick={() => void onCopy('log path', entry.log_path)}
          className={`border border-rule px-2.5 py-1 text-ink-soft hover:text-ink ${FOCUS}`}
        >
          Copy path
        </button>
        <button
          type="button"
          disabled={text === null}
          onClick={() => text !== null && void onCopy('full log', text)}
          className={`border border-rule px-2.5 py-1 text-ink-soft hover:text-ink disabled:opacity-50 ${FOCUS}`}
        >
          Copy full log
        </button>
        <button
          type="button"
          onClick={() => void onCopy('command', `synapse spool show ${entry.id}`)}
          className={`border border-rule px-2.5 py-1 text-ink-soft hover:text-ink ${FOCUS}`}
        >
          Copy <span className="text-drape">synapse spool show {entry.id}</span>
        </button>
        {copyNote && (
          <span role={copyNote.ok ? 'status' : 'alert'} className={copyNote.ok ? 'text-ink-soft' : 'text-alarm'}>
            <span aria-hidden="true">{copyNote.ok ? '● ' : '▲ '}</span>
            {copyNote.text}
          </span>
        )}
      </div>

      <div className={`grid grid-cols-1 gap-4 ${view === 'side-by-side' ? 'md:grid-cols-2' : ''}`}>
        {(view === 'side-by-side' || view === 'raw') && (
          <div className="border border-rule bg-ground">
            <p className="flex justify-between border-b border-rule px-3 py-1.5 font-mono text-[11px] font-bold uppercase tracking-wider text-ink">
              <span>Raw output</span>
              {tokens !== null && <span className="font-normal tabular-nums text-ink-soft">≈ {fmtInt(tokens)} tokens</span>}
            </p>
            {log.kind === 'loading' && <p className="p-3 font-mono text-xs text-ink-soft">reading log…</p>}
            {log.kind === 'error' && (
              <div role="alert" className="p-3 font-mono text-xs">
                <p className="text-alarm">
                  <span aria-hidden="true">▲ </span>Log could not be read.
                </p>
                <p className="mt-1 break-words text-ink-soft">{log.message}</p>
              </div>
            )}
            {tail && tail.total > MAX_LINES && (
              <p className="border-b border-rule px-3 py-1 font-mono text-[11px] tabular-nums text-ink-soft">
                showing last {fmtInt(tail.shownCount)} of {fmtInt(tail.total)} lines · copy the full log for the rest
              </p>
            )}
            {text !== null &&
              (text.length === 0 ? (
                <p className="p-3 font-mono text-xs text-ink-soft">
                  <span aria-hidden="true">○ </span>Log is empty: the command produced no output.
                </p>
              ) : (
                <pre
                  tabIndex={0}
                  aria-label="Raw output"
                  className={`max-h-96 overflow-auto p-3 font-mono text-[11px] leading-relaxed text-ink-soft ${FOCUS}`}
                >
                  {tail?.shown}
                </pre>
              ))}
          </div>
        )}

        {(view === 'side-by-side' || view === 'gated') && (
          <div className="border border-rule bg-ground">
            <p className="border-b border-rule px-3 py-1.5 font-mono text-[11px] font-bold uppercase tracking-wider text-ink">
              Gated output
            </p>
            <div className="p-3 font-mono text-xs">
              <p className="text-ink-soft">
                <span aria-hidden="true">○ </span>Not recorded. The spool keeps only the raw log, so the gated text
                and its token count are not available here.
              </p>
              <p className="mt-2">
                <span className="text-ink-soft">Next </span>
                <span className="text-drape">synapse squash &lt;command&gt;</span>
              </p>
            </div>
          </div>
        )}
      </div>

      <Hint id="spool.tokens">
        Token counts are estimates: characters ÷ 4, the same rule the CLI uses.
      </Hint>
    </>
  )
}

function Absent({ title, body, next }: { title: string; body: string; next: string }) {
  return (
    <div>
      <p className="font-mono text-sm">
        <span aria-hidden="true" className="text-ink-soft">
          ○{' '}
        </span>
        {title}
      </p>
      <p className="mt-2 max-w-prose text-sm text-ink-soft">{body}</p>
      <p className="mt-3 font-mono text-xs">
        <span className="text-ink-soft">Next </span>
        <span className="text-drape">{next}</span>
      </p>
    </div>
  )
}

function RetryLine({ onRetry }: { onRetry: () => void }) {
  return (
    <p className="mt-3 font-mono text-xs">
      <button type="button" onClick={onRetry} className={`text-drape hover:text-ink ${FOCUS}`}>
        Re-read the spool
      </button>
      <span className="text-ink-soft"> · or run </span>
      <span className="text-drape">synapse spool list</span>
    </p>
  )
}
