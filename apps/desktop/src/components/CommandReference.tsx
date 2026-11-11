import { useCallback, useEffect, useId, useMemo, useRef, useState, useSyncExternalStore } from 'react'
import { useHints } from '../lib/hints'
import { formatKeys, isMac, listShortcuts, subscribeShortcuts, useShortcut, type ShortcutDef } from '../lib/shortcuts'

/**
 * Quick command reference: every registered keyboard shortcut plus the
 * synapse CLI commands. "?", mod+/ and mod+k open and close it; so does
 * <CommandReferenceButton /> (or openCommandReference()). Self-contained:
 * the shell mounts <CommandReference /> once and it manages its own state.
 */

/** Window event the visible opener uses; CommandReference listens for it. */
export const COMMAND_REFERENCE_OPEN_EVENT = 'synapse:command-reference-open'

/** Open the command reference from anywhere (focus returns to the caller's focused element). */
export function openCommandReference(): void {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent(COMMAND_REFERENCE_OPEN_EVENT))
}

interface CliCommand {
  command: string
  description: string
}

// Descriptions follow apps/cli/src/main.rs (clap doc comments) and docs/USER_GUIDE.md.
export const CLI_COMMANDS: CliCommand[] = [
  { command: 'synapse scan', description: 'Detect installed AI tools and the config files they own.' },
  { command: 'synapse import [--dry-run]', description: 'Import detected configs into the Brain; --dry-run previews with no writes.' },
  { command: 'synapse project [--dry-run]', description: 'Project the Brain back out to every linked tool; --dry-run shows what would be written.' },
  { command: 'synapse sync [--once]', description: 'Run one import + project pass and resolve or queue conflicts. --daemon watcher is planned, not yet shipped.' },
  { command: 'synapse doctor [--fix]', description: 'Diagnose Brain/tool drift and explain fixes; --fix applies them.' },
  { command: 'synapse snapshot "<message>"', description: 'Record a git snapshot of the current Brain state.' },
  { command: 'synapse rollback <hash>', description: 'Restore the Brain to a prior snapshot (id or git ref).' },
  { command: 'synapse mem <search|timeline|get>', description: 'Agent memory: search the index, view context around an id, or get full detail.' },
  { command: 'synapse hud [--once]', description: 'Open the live Vital Signs HUD; --once prints one snapshot and exits.' },
  { command: 'synapse serve [--host] [--port]', description: 'Start the local runtime gateway proxy (default 127.0.0.1:8088).' },
]

const HELP_GROUP = 'Help'
/** Groups render in this order; any other group follows, in registration order. */
const GROUP_ORDER = ['Navigation', 'Actions', 'View', HELP_GROUP]

const SPOKEN: Record<string, string> = {
  '⌘': 'Command',
  '⌃': 'Control',
  '⌥': 'Option',
  '⇧': 'Shift',
  Ctrl: 'Control',
  Win: 'Windows',
  Esc: 'Escape',
  Del: 'Delete',
  '↑': 'Up arrow',
  '↓': 'Down arrow',
  '←': 'Left arrow',
  '→': 'Right arrow',
  '?': 'Question mark',
  '/': 'Slash',
  '+': 'Plus',
  '-': 'Minus',
}

/** What a screen reader should say for a combo: "Command+K", not "place of interest sign K". */
export function spokenKeys(keys: string): string {
  return formatKeys(keys)
    .map((k) => SPOKEN[k] ?? k)
    .join('+')
}

function available(s: ShortcutDef): boolean {
  try {
    return s.when ? s.when() !== false : true
  } catch {
    return false
  }
}

interface Row {
  label: string
  group: string
  combos: ShortcutDef[]
  available: boolean
}

function useRegisteredShortcuts(): ShortcutDef[] {
  // listShortcuts() returns a fresh array each call; cache per notification.
  const cache = useRef<ShortcutDef[] | null>(null)
  const subscribe = useCallback(
    (cb: () => void) =>
      subscribeShortcuts(() => {
        cache.current = null
        cb()
      }),
    [],
  )
  const get = useCallback(() => (cache.current ??= listShortcuts()), [])
  return useSyncExternalStore(subscribe, get, get)
}

function Kbd({ children }: { children: string }) {
  return (
    <kbd className="inline-block min-w-[1.5rem] border border-rule bg-chart px-1.5 py-0.5 text-center font-mono text-[11px] leading-none text-ink">
      {children}
    </kbd>
  )
}

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

export function CommandReference(): JSX.Element | null {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [activeGroup, setActiveGroup] = useState<string | null>(null)
  const openRef = useRef(open)
  openRef.current = open
  const restoreRef = useRef<HTMLElement | null>(null)
  const dialogRef = useRef<HTMLDivElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)
  const titleId = useId()
  const noteId = useId()
  const hints = useHints()
  const shortcuts = useRegisteredShortcuts()

  const show = useCallback(() => {
    if (openRef.current) return
    const active = document.activeElement
    restoreRef.current = active instanceof HTMLElement && active !== document.body ? active : null
    setQuery('')
    setOpen(true)
  }, [])

  const close = useCallback(() => {
    if (!openRef.current) return
    setOpen(false)
  }, [])

  const toggleOpen = useCallback(() => (openRef.current ? close() : show()), [close, show])
  const OPEN_LABEL = 'Open or close command reference'

  // inModal: these keep working while the dialog is open; every other shortcut
  // (including a screen's own Escape) is inert behind it.
  useShortcut({ id: 'help.reference.question', keys: '?', label: OPEN_LABEL, group: HELP_GROUP, inModal: true }, toggleOpen)
  useShortcut({ id: 'help.reference.slash', keys: 'mod+/', label: OPEN_LABEL, group: HELP_GROUP, inModal: true }, toggleOpen)
  useShortcut({ id: 'help.reference.k', keys: 'mod+k', label: OPEN_LABEL, group: HELP_GROUP, inModal: true }, toggleOpen)
  useShortcut(
    {
      id: 'help.reference.close',
      keys: 'escape',
      label: 'Close command reference',
      group: HELP_GROUP,
      allowWhileTyping: true,
      inModal: true,
      when: () => openRef.current,
    },
    close,
  )

  // The visible opener (CommandReferenceButton) talks to us through a window event.
  useEffect(() => {
    if (typeof window === 'undefined') return
    window.addEventListener(COMMAND_REFERENCE_OPEN_EVENT, show)
    return () => window.removeEventListener(COMMAND_REFERENCE_OPEN_EVENT, show)
  }, [show])

  // Focus the search box on open; restore focus on close.
  useEffect(() => {
    if (open) {
      searchRef.current?.focus()
      return
    }
    const el = restoreRef.current
    restoreRef.current = null
    if (el && el.isConnected) el.focus()
  }, [open])

  const q = query.trim().toLowerCase()

  const groups = useMemo(() => {
    // One row per action: several combos for the same label share a row.
    const byGroup = new Map<string, Map<string, Row>>()
    for (const s of shortcuts) {
      const hay = [s.label, s.keys, s.group, formatKeys(s.keys).join(' '), spokenKeys(s.keys)].join(' ').toLowerCase()
      if (q && !hay.includes(q)) continue
      const rows = byGroup.get(s.group) ?? new Map<string, Row>()
      const row = rows.get(s.label) ?? { label: s.label, group: s.group, combos: [], available: false }
      row.combos.push(s)
      row.available ||= available(s)
      rows.set(s.label, row)
      byGroup.set(s.group, rows)
    }
    const rank = (g: string) => {
      const i = GROUP_ORDER.indexOf(g)
      return i === -1 ? GROUP_ORDER.length : i
    }
    return [...byGroup.entries()]
      .map(([g, rows]) => [g, [...rows.values()]] as const)
      .sort((a, b) => rank(a[0]) - rank(b[0]))
    // `open` is a dependency so availability is re-read each time the dialog opens.
  }, [shortcuts, q, open])

  const cli = useMemo(
    () => CLI_COMMANDS.filter((c) => !q || `${c.command} ${c.description}`.toLowerCase().includes(q)),
    [q],
  )

  const currentGroup = activeGroup && groups.some(([g]) => g === activeGroup) ? activeGroup : (groups[0]?.[0] ?? null)

  const trapFocus = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'Tab' || !dialogRef.current) return
    const nodes = [...dialogRef.current.querySelectorAll<HTMLElement>(FOCUSABLE)]
    if (nodes.length === 0) {
      e.preventDefault()
      return
    }
    const first = nodes[0]
    const last = nodes[nodes.length - 1]
    const active = document.activeElement
    if (e.shiftKey && (active === first || !dialogRef.current.contains(active))) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && (active === last || !dialogRef.current.contains(active))) {
      e.preventDefault()
      first.focus()
    }
  }

  if (!open) return null

  const empty = groups.length === 0 && cli.length === 0
  const shortcutCount = groups.reduce((n, [, rows]) => n + rows.length, 0)

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center px-4 pt-[10vh]"
      data-testid="command-reference-backdrop"
      style={{ background: 'color-mix(in srgb, var(--ground) 72%, transparent)' }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onKeyDown={trapFocus}
        className="flex max-h-[75vh] w-full max-w-2xl flex-col border border-rule bg-ground text-ink"
      >
        <header className="flex items-center justify-between border-b border-rule px-4 py-3">
          <h2 id={titleId} className="font-mono text-xs uppercase tracking-[0.18em] text-ink">
            Command reference
          </h2>
          <button
            type="button"
            onClick={close}
            aria-label="Close command reference"
            className="border border-transparent px-2 font-mono text-xs text-ink-soft hover:border-rule hover:text-ink focus-visible:border-drape focus-visible:text-ink focus-visible:outline-none"
          >
            Esc
          </button>
        </header>

        <div className="border-b border-rule px-4 py-2">
          <label className="sr-only" htmlFor={`${titleId}-search`}>
            Filter commands
          </label>
          <input
            id={`${titleId}-search`}
            ref={searchRef}
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Filter shortcuts and commands"
            autoComplete="off"
            spellCheck={false}
            className="w-full bg-transparent py-1 font-mono text-sm text-ink placeholder:text-ink-soft focus:outline-none"
          />
        </div>

        <p role="status" aria-live="polite" className="sr-only">
          {q ? (empty ? 'No matches.' : `${shortcutCount} shortcuts, ${cli.length} commands.`) : ''}
        </p>

        <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
          {groups.length > 0 && (
            <section aria-label="Keyboard shortcuts" className="border-b border-rule">
              {groups.map(([group, items]) => {
                const active = group === currentGroup
                return (
                  <div key={group} onMouseEnter={() => setActiveGroup(group)} onFocus={() => setActiveGroup(group)}>
                    <h3
                      className={
                        'border-b border-rule px-4 pb-1 pt-3 font-mono text-[11px] uppercase tracking-[0.16em] ' +
                        (active ? 'text-drape' : 'text-ink-soft')
                      }
                      data-active={active || undefined}
                    >
                      {group}
                    </h3>
                    <ul>
                      {items.map((row) => (
                        <li
                          key={row.label}
                          data-available={row.available ? undefined : 'false'}
                          title={row.available ? undefined : 'Not available on this screen right now'}
                          className={
                            'flex items-center justify-between gap-4 border-b border-rule px-4 py-1.5 text-sm last:border-b-0 ' +
                            (row.available ? '' : 'text-ink-soft')
                          }
                        >
                          <span>
                            {row.label}
                            {!row.available && <span className="sr-only"> (not available right now)</span>}
                          </span>
                          <span className="flex shrink-0 flex-wrap items-center justify-end gap-1">
                            {row.combos.map((s, ci) => (
                              <span key={s.id} data-shortcut-id={s.id} className="flex items-center gap-1">
                                {ci > 0 && (
                                  <span aria-hidden="true" className="font-mono text-[11px] text-ink-soft">
                                    or
                                  </span>
                                )}
                                <span className="sr-only">{(ci > 0 ? 'or ' : '') + spokenKeys(s.keys)}</span>
                                <span aria-hidden="true" className="flex gap-1">
                                  {formatKeys(s.keys).map((k, i) => (
                                    <Kbd key={i}>{k}</Kbd>
                                  ))}
                                </span>
                              </span>
                            ))}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )
              })}
            </section>
          )}

          {cli.length > 0 && (
            <section aria-labelledby={`${titleId}-cli`}>
              <h3
                id={`${titleId}-cli`}
                className="border-b border-rule px-4 pb-1 pt-3 font-mono text-[11px] uppercase tracking-[0.16em] text-ink-soft"
              >
                CLI
              </h3>
              <ul>
                {cli.map((c) => (
                  <li key={c.command} className="grid gap-0.5 border-b border-rule px-4 py-1.5 last:border-b-0 sm:grid-cols-[15rem_1fr] sm:gap-4">
                    <code className="font-mono text-xs text-ink">{c.command}</code>
                    <span className="text-xs text-ink-soft">{c.description}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {empty && <p className="px-4 py-6 font-mono text-xs text-ink-soft">No matches for "{query}".</p>}
        </div>

        <footer className="flex items-center justify-between gap-4 border-t border-rule px-4 py-3">
          <div className="flex min-w-0 flex-col gap-1">
            <p id={noteId} className="text-xs text-ink-soft">
              Hints can be turned back on here at any time.
            </p>
            {hints.dismissedCount > 0 && (
              <button
                type="button"
                onClick={hints.resetDismissed}
                className="self-start font-mono text-[11px] text-ink-soft underline-offset-2 hover:text-ink hover:underline focus-visible:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape"
              >
                {hints.dismissedCount === 1
                  ? 'Show 1 dismissed hint again'
                  : `Show ${hints.dismissedCount} dismissed hints again`}
              </button>
            )}
          </div>
          <label className="flex shrink-0 items-center gap-2 text-xs text-ink">
            <span id={`${titleId}-hints`}>Show hints</span>
            <button
              type="button"
              role="switch"
              aria-checked={hints.enabled}
              aria-labelledby={`${titleId}-hints`}
              aria-describedby={noteId}
              onClick={() => hints.setEnabled(!hints.enabled)}
              className={
                'relative h-4 w-8 border focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape ' +
                (hints.enabled ? 'border-drape bg-drape' : 'border-rule bg-chart')
              }
            >
              <span
                aria-hidden="true"
                className={
                  'absolute top-0.5 h-2.5 w-2.5 transition-[left] ' +
                  (hints.enabled ? 'left-[1.1rem] bg-ground' : 'left-0.5 bg-ink-soft')
                }
              />
            </button>
          </label>
        </footer>
      </div>
    </div>
  )
}

/**
 * Visible opener for the command reference, styled like the sidebar's quiet
 * mono controls. Focus returns here when the reference closes.
 */
export function CommandReferenceButton({ className }: { className?: string }): JSX.Element {
  return (
    <button
      type="button"
      onClick={openCommandReference}
      aria-haspopup="dialog"
      aria-keyshortcuts={isMac() ? 'Shift+? Meta+K Meta+/' : 'Shift+? Control+K Control+/'}
      className={
        'font-mono text-xs text-ink-soft hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape ' +
        (className ?? '')
      }
    >
      shortcuts
    </button>
  )
}
