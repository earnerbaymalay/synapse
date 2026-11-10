import { useEffect, useRef } from 'react'

/**
 * Keyboard shortcuts: one registry, so the Command Reference can list every
 * shortcut the app actually responds to (nothing listed that isn't wired).
 *
 * keys format: "mod+1", "mod+/", "?", "r", "escape", "mod+shift+h"
 *   "mod" = Ctrl on Windows/Linux, Cmd on macOS.
 *
 * Usage (in a component):
 *   useShortcut({ id: 'nav.intake', keys: 'mod+1', label: 'Go to Intake', group: 'Navigation' }, () => setScreen('intake'))
 */

export interface ShortcutDef {
  id: string
  keys: string
  label: string
  group: 'Navigation' | 'Actions' | 'View' | 'Help' | (string & {})
  /** only fire when this returns true (e.g. only on a given screen) */
  when?: () => boolean
}

type Entry = ShortcutDef & { handler: (e: KeyboardEvent) => void }

const registry = new Map<string, Entry>()
const listeners = new Set<() => void>()

export const isMac = (): boolean =>
  typeof navigator !== 'undefined' && /mac|iphone|ipad/i.test(navigator.platform || navigator.userAgent)

/** Human label for a key combo, e.g. "mod+1" -> "Ctrl 1" / "⌘ 1". */
export function formatKeys(keys: string): string[] {
  return keys.split('+').map((k) => {
    const key = k.trim().toLowerCase()
    if (key === 'mod') return isMac() ? '⌘' : 'Ctrl'
    if (key === 'shift') return isMac() ? '⇧' : 'Shift'
    if (key === 'alt') return isMac() ? '⌥' : 'Alt'
    if (key === 'escape') return 'Esc'
    return key.length === 1 ? key.toUpperCase() : key[0].toUpperCase() + key.slice(1)
  })
}

function isTypingTarget(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null
  if (!el) return false
  const tag = el.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable
}

export function matches(e: KeyboardEvent, keys: string): boolean {
  const parts = keys.toLowerCase().split('+').map((p) => p.trim())
  const want = {
    mod: parts.includes('mod'),
    shift: parts.includes('shift'),
    alt: parts.includes('alt'),
  }
  const key = parts.filter((p) => !['mod', 'shift', 'alt'].includes(p))[0] ?? ''
  const mod = isMac() ? e.metaKey : e.ctrlKey
  if (mod !== want.mod || e.altKey !== want.alt) return false
  // "?" is typed with shift on most layouts; don't require shift to be declared for it
  if (key !== '?' && e.shiftKey !== want.shift) return false
  return e.key.toLowerCase() === key
}

let installed = false
function install() {
  if (installed || typeof window === 'undefined') return
  installed = true
  window.addEventListener('keydown', (e) => {
    for (const entry of registry.values()) {
      if (!matches(e, entry.keys)) continue
      // bare keys (no modifier) never fire while the user is typing
      if (!entry.keys.includes('mod') && isTypingTarget(e.target)) continue
      if (entry.when && !entry.when()) continue
      e.preventDefault()
      entry.handler(e)
      return
    }
  })
}

export function registerShortcut(def: ShortcutDef, handler: (e: KeyboardEvent) => void): () => void {
  install()
  registry.set(def.id, { ...def, handler })
  listeners.forEach((l) => l())
  return () => {
    registry.delete(def.id)
    listeners.forEach((l) => l())
  }
}

/** Everything currently registered (for the Command Reference). */
export function listShortcuts(): ShortcutDef[] {
  return [...registry.values()].map(({ handler: _h, ...def }) => def)
}

export function subscribeShortcuts(fn: () => void): () => void {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}

/** Register a shortcut for the lifetime of the calling component. */
export function useShortcut(def: ShortcutDef, handler: (e: KeyboardEvent) => void): void {
  const ref = useRef(handler)
  ref.current = handler
  const whenRef = useRef(def.when)
  whenRef.current = def.when
  useEffect(
    () =>
      registerShortcut(
        { ...def, when: () => (whenRef.current ? whenRef.current() : true) },
        (e) => ref.current(e),
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [def.id, def.keys, def.label, def.group],
  )
}
