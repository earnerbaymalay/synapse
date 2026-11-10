import { useEffect, useRef } from 'react'

/**
 * Keyboard shortcuts: one registry, so the Command Reference can list every
 * shortcut the app actually responds to (nothing listed that isn't wired).
 *
 * keys format: "mod+1", "mod+/", "?", "r", "escape", "mod+shift+h"
 *   "mod" = Ctrl on Windows/Linux, Cmd on macOS.
 *   "ctrl" / "meta" may be named explicitly when you really mean that key.
 *   A literal plus is written "mod++" or "+".
 *
 * Usage (in a component):
 *   useShortcut({ id: 'nav.intake', keys: 'mod+1', label: 'Go to Intake', group: 'Navigation' }, () => setScreen('intake'))
 *
 * Rules the dispatcher enforces:
 * - Bare keys (no mod/ctrl/meta/alt) never fire while the user is typing in an
 *   input, textarea, select or contenteditable, unless the def sets
 *   `allowWhileTyping` (e.g. Escape to close a dialog from its search box).
 * - Extra modifiers reject a match: "r" does not fire on Ctrl+R or Cmd+R.
 * - Punctuation keys ("?", "/") ignore Shift and AltGr unless Shift is declared,
 *   because which modifiers produce them depends on the keyboard layout.
 * - Letter/digit keys fall back to the physical key (e.code) when the layout
 *   produces a non-Latin character, so "mod+k" works on a Cyrillic layout.
 * - Several registrations may share an id; the newest wins and unmounting it
 *   restores the previous one. Newer registrations are tried first.
 * - IME composition keystrokes are ignored.
 */

export interface ShortcutDef {
  id: string
  keys: string
  label: string
  group: 'Navigation' | 'Actions' | 'View' | 'Help' | (string & {})
  /** only fire when this returns true (e.g. only on a given screen) */
  when?: () => boolean
  /** fire even when focus is in a text field (bare keys are blocked otherwise) */
  allowWhileTyping?: boolean
}

type Entry = ShortcutDef & { handler: (e: KeyboardEvent) => void; token: number }

/** id -> stack of registrations (last = active). */
const registry = new Map<string, Entry[]>()
const listeners = new Set<() => void>()
let nextToken = 0

export const isMac = (): boolean => {
  if (typeof navigator === 'undefined') return false
  const uaData = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData
  const platform = uaData?.platform || navigator.platform || navigator.userAgent || ''
  return /mac|iphone|ipad|ipod/i.test(platform)
}

interface Parsed {
  mod: boolean
  ctrl: boolean
  meta: boolean
  shift: boolean
  alt: boolean
  key: string
}

const MODIFIERS = new Set(['mod', 'ctrl', 'control', 'meta', 'cmd', 'shift', 'alt', 'option'])
const KEY_ALIASES: Record<string, string> = {
  esc: 'escape',
  return: 'enter',
  space: ' ',
  spacebar: ' ',
  up: 'arrowup',
  down: 'arrowdown',
  left: 'arrowleft',
  right: 'arrowright',
  del: 'delete',
  plus: '+',
}

function splitKeys(keys: string): string[] {
  const raw = keys.trim().toLowerCase()
  if (raw === '+') return ['+']
  const parts = raw.split('+').map((p) => p.trim())
  // "mod++" splits to [..., '', ''] : the trailing empty pair is a literal plus
  if (parts.length >= 2 && parts[parts.length - 1] === '' && parts[parts.length - 2] === '') {
    parts.splice(parts.length - 2, 2, '+')
  }
  return parts.filter((p) => p !== '')
}

function parse(keys: string): Parsed {
  const parts = splitKeys(keys)
  const has = (...names: string[]) => names.some((n) => parts.includes(n))
  const rawKey = parts.filter((p) => !MODIFIERS.has(p))[0] ?? ''
  return {
    mod: has('mod'),
    ctrl: has('ctrl', 'control'),
    meta: has('meta', 'cmd'),
    shift: has('shift'),
    alt: has('alt', 'option'),
    key: KEY_ALIASES[rawKey] ?? rawKey,
  }
}

const isPunctuation = (k: string) => k.length === 1 && !/[a-z0-9 ]/i.test(k)

/** Human label for a key combo, e.g. "mod+1" -> ["Ctrl", "1"] / ["⌘", "1"]. */
export function formatKeys(keys: string): string[] {
  const mac = isMac()
  const names: Record<string, string> = {
    mod: mac ? '⌘' : 'Ctrl',
    ctrl: mac ? '⌃' : 'Ctrl',
    control: mac ? '⌃' : 'Ctrl',
    meta: mac ? '⌘' : 'Win',
    cmd: mac ? '⌘' : 'Win',
    shift: mac ? '⇧' : 'Shift',
    alt: mac ? '⌥' : 'Alt',
    option: mac ? '⌥' : 'Alt',
    escape: 'Esc',
    esc: 'Esc',
    enter: 'Enter',
    return: 'Enter',
    ' ': 'Space',
    space: 'Space',
    arrowup: '↑',
    up: '↑',
    arrowdown: '↓',
    down: '↓',
    arrowleft: '←',
    left: '←',
    arrowright: '→',
    right: '→',
    delete: 'Del',
    del: 'Del',
    backspace: 'Backspace',
    tab: 'Tab',
    plus: '+',
  }
  return splitKeys(keys).map((key) => {
    if (key in names) return names[key]
    if (key.length === 1) return key.toUpperCase()
    return key[0].toUpperCase() + key.slice(1)
  })
}

const TEXT_INPUT_TYPES = new Set([
  'text', 'search', 'email', 'url', 'tel', 'password', 'number',
  'date', 'datetime-local', 'month', 'time', 'week', '',
])

/** True when keystrokes on this target are text entry. Exported for tests. */
export function isTypingTarget(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null
  if (!el || typeof el.tagName !== 'string') return false
  const tag = el.tagName
  if (tag === 'INPUT') {
    const type = ((el as HTMLInputElement).getAttribute('type') || '').toLowerCase()
    return TEXT_INPUT_TYPES.has(type)
  }
  if (tag === 'TEXTAREA' || tag === 'SELECT') return true
  if (el.isContentEditable) return true
  // jsdom lacks isContentEditable; also honour ARIA text widgets
  const ce = el.getAttribute?.('contenteditable')
  if (ce !== null && ce !== undefined && ce !== 'false') return true
  const role = el.getAttribute?.('role')
  return role === 'textbox' || role === 'searchbox' || role === 'combobox'
}

function eventKeyMatches(e: KeyboardEvent, key: string): boolean {
  const k = (e.key || '').toLowerCase()
  if (k === key) return true
  if (key === 'escape' && k === 'esc') return true // old Edge/IE
  // Non-Latin layouts: match letters/digits by physical position.
  if (/^[a-z]$/.test(key) && e.code === `Key${key.toUpperCase()}` && !/^[a-z]$/.test(k)) return true
  if (/^[0-9]$/.test(key) && (e.code === `Digit${key}` || e.code === `Numpad${key}`) && !/^[0-9]$/.test(k)) return true
  return false
}

export function matches(e: KeyboardEvent, keys: string): boolean {
  const want = parse(keys)
  if (!want.key) return false
  const mac = isMac()
  const punct = isPunctuation(want.key)
  const altGraph = typeof e.getModifierState === 'function' && e.getModifierState('AltGraph')

  // Expected state of the physical ctrl / meta keys.
  const wantCtrl = want.ctrl || (want.mod && !mac)
  const wantMeta = want.meta || (want.mod && mac)
  let ctrl = e.ctrlKey
  let alt = e.altKey
  // Windows reports AltGr as Ctrl+Alt; layouts use it to type punctuation.
  if (punct && !want.alt && !wantCtrl && (altGraph || (ctrl && alt && !mac))) {
    ctrl = false
    alt = false
  }
  if (ctrl !== wantCtrl || e.metaKey !== wantMeta) return false
  // Punctuation may need Option on macOS layouts: tolerate undeclared alt there.
  if (alt !== want.alt && !(punct && !want.alt && mac)) return false
  // Punctuation like "?" or "/" needs Shift on some layouts and not others.
  if (!(punct && !want.shift) && e.shiftKey !== want.shift) return false
  return eventKeyMatches(e, want.key)
}

/** True when a combo is a "bare" key (no command modifier). Shift alone is still bare. */
function isBare(keys: string): boolean {
  const p = parse(keys)
  return !p.mod && !p.ctrl && !p.meta && !p.alt
}

function activeEntries(): Entry[] {
  const out: Entry[] = []
  for (const stack of registry.values()) {
    const top = stack[stack.length - 1]
    if (top) out.push(top)
  }
  // newest registration first, so a freshly opened dialog's keys take priority
  return out.sort((a, b) => b.token - a.token)
}

function onKeyDown(e: KeyboardEvent) {
  if (e.isComposing || e.keyCode === 229) return
  const target = (typeof e.composedPath === 'function' && e.composedPath()[0]) || e.target
  const typing = isTypingTarget(target)
  for (const entry of activeEntries()) {
    if (!matches(e, entry.keys)) continue
    if (typing && isBare(entry.keys) && !entry.allowWhileTyping) continue
    if (entry.when && !entry.when()) continue
    e.preventDefault()
    entry.handler(e)
    return
  }
}

let installed = false
function install() {
  if (installed || typeof window === 'undefined') return
  installed = true
  window.addEventListener('keydown', onKeyDown)
}

function notify() {
  ;[...listeners].forEach((l) => l())
}

export function registerShortcut(def: ShortcutDef, handler: (e: KeyboardEvent) => void): () => void {
  install()
  const entry: Entry = { ...def, handler, token: ++nextToken }
  const stack = registry.get(def.id) ?? []
  stack.push(entry)
  registry.set(def.id, stack)
  notify()
  let done = false
  return () => {
    if (done) return
    done = true
    const s = registry.get(def.id)
    if (!s) return
    const i = s.indexOf(entry)
    if (i !== -1) s.splice(i, 1)
    if (s.length === 0) registry.delete(def.id)
    notify()
  }
}

/** Everything currently registered (for the Command Reference). One row per id. */
export function listShortcuts(): ShortcutDef[] {
  return activeEntries()
    .sort((a, b) => a.token - b.token)
    .map(({ handler: _h, token: _t, ...def }) => def)
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
    [def.id, def.keys, def.label, def.group, def.allowWhileTyping],
  )
}
