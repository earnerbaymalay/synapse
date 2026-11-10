import { useCallback, useEffect, useId, useMemo, useRef, useState, useSyncExternalStore } from 'react'
import { useHints } from '../lib/hints'
import { formatKeys, listShortcuts, subscribeShortcuts, useShortcut, type ShortcutDef } from '../lib/shortcuts'

/**
 * Quick command reference: every registered keyboard shortcut plus the
 * synapse CLI commands. Opens with "?", mod+/ or mod+k. Self-contained:
 * the shell mounts <CommandReference /> once and it manages its own state.
 */

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

  useShortcut({ id: 'help.reference.question', keys: '?', label: 'Open command reference', group: HELP_GROUP }, show)
  useShortcut({ id: 'help.reference.slash', keys: 'mod+/', label: 'Open command reference', group: HELP_GROUP }, show)
  useShortcut({ id: 'help.reference.k', keys: 'mod+k', label: 'Open command reference', group: HELP_GROUP }, show)
  useShortcut(
    {
      id: 'help.reference.close',
      keys: 'escape',
      label: 'Close command reference',
      group: HELP_GROUP,
      allowWhileTyping: true,
      when: () => openRef.current,
    },
    close,
  )

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
    const byGroup = new Map<string, ShortcutDef[]>()
    for (const s of shortcuts) {
      const hay = [s.label, s.keys, s.group, formatKeys(s.keys).join(' ')].join(' ').toLowerCase()
      if (q && !hay.includes(q)) continue
      const list = byGroup.get(s.group) ?? []
      list.push(s)
      byGroup.set(s.group, list)
    }
    return [...byGroup.entries()]
  }, [shortcuts, q])

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

        <div className="flex-1 overflow-y-auto">
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
                      {items.map((s) => (
                        <li
                          key={s.id}
                          data-shortcut-id={s.id}
                          className="flex items-center justify-between gap-4 border-b border-rule px-4 py-1.5 text-sm last:border-b-0"
                        >
                          <span>{s.label}</span>
                          <span className="flex shrink-0 gap-1" aria-label={formatKeys(s.keys).join(' + ')}>
                            {formatKeys(s.keys).map((k, i) => (
                              <Kbd key={i}>{k}</Kbd>
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
          <p id={noteId} className="text-xs text-ink-soft">
            Hints can be turned back on here at any time.
          </p>
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
