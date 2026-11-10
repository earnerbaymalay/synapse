import { afterEach, describe, expect, it, vi } from 'vitest'
import { formatKeys, isTypingTarget, listShortcuts, matches, registerShortcut, subscribeShortcuts } from '../shortcuts'

function setPlatform(p: string) {
  vi.spyOn(navigator, 'platform', 'get').mockReturnValue(p)
}

function key(init: KeyboardEventInit & { altGraph?: boolean }): KeyboardEvent {
  const e = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init })
  if (init.altGraph) Object.defineProperty(e, 'getModifierState', { value: (m: string) => m === 'AltGraph' })
  return e
}

afterEach(() => vi.restoreAllMocks())

describe('matches()', () => {
  it('maps mod to Ctrl off macOS and Cmd on macOS', () => {
    setPlatform('Linux x86_64')
    expect(matches(key({ key: 'k', ctrlKey: true }), 'mod+k')).toBe(true)
    expect(matches(key({ key: 'k', metaKey: true }), 'mod+k')).toBe(false)
    setPlatform('MacIntel')
    expect(matches(key({ key: 'k', metaKey: true }), 'mod+k')).toBe(true)
    expect(matches(key({ key: 'k', ctrlKey: true }), 'mod+k')).toBe(false)
  })

  it('rejects undeclared command modifiers on bare keys', () => {
    setPlatform('MacIntel')
    expect(matches(key({ key: 'r' }), 'r')).toBe(true)
    expect(matches(key({ key: 'r', ctrlKey: true }), 'r')).toBe(false)
    expect(matches(key({ key: 'r', metaKey: true }), 'r')).toBe(false)
    setPlatform('Win32')
    expect(matches(key({ key: 'r', metaKey: true }), 'r')).toBe(false)
  })

  it('requires shift exactly for letters', () => {
    setPlatform('Linux x86_64')
    expect(matches(key({ key: 'H', ctrlKey: true, shiftKey: true }), 'mod+shift+h')).toBe(true)
    expect(matches(key({ key: 'h', ctrlKey: true }), 'mod+shift+h')).toBe(false)
    expect(matches(key({ key: 'H', shiftKey: true }), 'h')).toBe(false)
  })

  it('matches "?" with or without shift and through AltGr (non-US layouts)', () => {
    setPlatform('Win32')
    expect(matches(key({ key: '?', shiftKey: true }), '?')).toBe(true)
    expect(matches(key({ key: '?' }), '?')).toBe(true)
    // Windows reports AltGr as Ctrl+Alt
    expect(matches(key({ key: '?', ctrlKey: true, altKey: true }), '?')).toBe(true)
    expect(matches(key({ key: '?', altGraph: true }), '?')).toBe(true)
    // but a real Ctrl+? is not a bare "?"
    expect(matches(key({ key: '?', ctrlKey: true }), '?')).toBe(false)
  })

  it('matches mod+/ when the layout needs shift for "/" (e.g. German Shift+7)', () => {
    setPlatform('Linux x86_64')
    expect(matches(key({ key: '/', ctrlKey: true, shiftKey: true }), 'mod+/')).toBe(true)
    expect(matches(key({ key: '/', ctrlKey: true }), 'mod+/')).toBe(true)
  })

  it('falls back to the physical key on non-Latin layouts', () => {
    setPlatform('Linux x86_64')
    expect(matches(key({ key: 'л', code: 'KeyK', ctrlKey: true }), 'mod+k')).toBe(true)
    // a Latin key at a different position does not match by code
    expect(matches(key({ key: 'z', code: 'KeyY', ctrlKey: true }), 'mod+y')).toBe(false)
  })

  it('handles escape and aliases', () => {
    expect(matches(key({ key: 'Escape' }), 'escape')).toBe(true)
    expect(matches(key({ key: 'Escape' }), 'esc')).toBe(true)
    expect(matches(key({ key: 'Escape', shiftKey: true }), 'escape')).toBe(false)
  })
})

describe('formatKeys()', () => {
  it('renders platform labels', () => {
    setPlatform('Linux x86_64')
    expect(formatKeys('mod+shift+h')).toEqual(['Ctrl', 'Shift', 'H'])
    expect(formatKeys('mod+/')).toEqual(['Ctrl', '/'])
    expect(formatKeys('escape')).toEqual(['Esc'])
    expect(formatKeys('?')).toEqual(['?'])
    setPlatform('MacIntel')
    expect(formatKeys('mod+alt+1')).toEqual(['⌘', '⌥', '1'])
  })

  it('handles named keys, a literal plus, and does not crash on empty parts', () => {
    setPlatform('Linux x86_64')
    expect(formatKeys('arrowup')).toEqual(['↑'])
    expect(formatKeys('mod++')).toEqual(['Ctrl', '+'])
    expect(formatKeys('pageDown')).toEqual(['Pagedown'])
    expect(() => formatKeys('mod+ ')).not.toThrow()
  })
})

describe('registry', () => {
  it('keeps one row per id and restores the earlier registration on unmount', () => {
    const a = vi.fn()
    const b = vi.fn()
    const offA = registerShortcut({ id: 't.dup', keys: 'x', label: 'A', group: 'Test' }, a)
    const offB = registerShortcut({ id: 't.dup', keys: 'x', label: 'B', group: 'Test' }, b)
    expect(listShortcuts().filter((s) => s.id === 't.dup').map((s) => s.label)).toEqual(['B'])
    window.dispatchEvent(key({ key: 'x' }))
    expect(b).toHaveBeenCalledTimes(1)
    expect(a).not.toHaveBeenCalled()
    offA() // unmounting the older one must not remove the newer
    expect(listShortcuts().some((s) => s.id === 't.dup')).toBe(true)
    offB()
    expect(listShortcuts().some((s) => s.id === 't.dup')).toBe(false)
  })

  it('notifies subscribers', () => {
    const fn = vi.fn()
    const unsub = subscribeShortcuts(fn)
    const off = registerShortcut({ id: 't.sub', keys: 'y', label: 'Y', group: 'Test' }, () => {})
    off()
    unsub()
    expect(fn).toHaveBeenCalledTimes(2)
  })

  it('blocks bare keys while typing unless allowWhileTyping, but lets mod keys through', () => {
    setPlatform('Linux x86_64')
    const bare = vi.fn()
    const esc = vi.fn()
    const mod = vi.fn()
    const offs = [
      registerShortcut({ id: 't.bare', keys: 'g', label: 'g', group: 'Test' }, bare),
      registerShortcut({ id: 't.esc', keys: 'escape', label: 'esc', group: 'Test', allowWhileTyping: true }, esc),
      registerShortcut({ id: 't.mod', keys: 'mod+g', label: 'mod g', group: 'Test' }, mod),
    ]
    const input = document.createElement('input')
    document.body.appendChild(input)
    input.dispatchEvent(key({ key: 'g' }))
    input.dispatchEvent(key({ key: 'Escape' }))
    input.dispatchEvent(key({ key: 'g', ctrlKey: true }))
    expect(bare).not.toHaveBeenCalled()
    expect(esc).toHaveBeenCalledTimes(1)
    expect(mod).toHaveBeenCalledTimes(1)
    // non-text inputs are not typing targets
    const box = document.createElement('input')
    box.type = 'checkbox'
    document.body.appendChild(box)
    box.dispatchEvent(key({ key: 'g' }))
    expect(bare).toHaveBeenCalledTimes(1)
    offs.forEach((o) => o())
    input.remove()
    box.remove()
  })

  it('ignores IME composition keystrokes', () => {
    const fn = vi.fn()
    const off = registerShortcut({ id: 't.ime', keys: 'q', label: 'q', group: 'Test' }, fn)
    window.dispatchEvent(key({ key: 'q', isComposing: true }))
    expect(fn).not.toHaveBeenCalled()
    off()
  })

  it('isTypingTarget recognises contenteditable and textbox roles', () => {
    const div = document.createElement('div')
    div.setAttribute('contenteditable', 'true')
    expect(isTypingTarget(div)).toBe(true)
    const r = document.createElement('div')
    r.setAttribute('role', 'textbox')
    expect(isTypingTarget(r)).toBe(true)
    expect(isTypingTarget(document.createElement('button'))).toBe(false)
    expect(isTypingTarget(null)).toBe(false)
  })
})
