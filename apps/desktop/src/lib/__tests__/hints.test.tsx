import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Hint, HintsProvider, HINTS_KEY_DISMISSED, HINTS_KEY_ENABLED, useHints } from '../hints'

function Toggle() {
  const { enabled, setEnabled } = useHints()
  return (
    <button type="button" onClick={() => setEnabled(!enabled)}>
      {enabled ? 'on' : 'off'}
    </button>
  )
}

function App() {
  return (
    <HintsProvider>
      <Toggle />
      <Hint id="a">Tip A</Hint>
      <Hint id="b">Tip B</Hint>
    </HintsProvider>
  )
}

beforeEach(() => localStorage.clear())
afterEach(() => vi.restoreAllMocks())

describe('hints', () => {
  it('is on by default on first boot', () => {
    render(<App />)
    expect(screen.getByText('Tip A')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'on' })).toBeInTheDocument()
  })

  it('persists turning hints off across a remount', () => {
    const { unmount } = render(<App />)
    fireEvent.click(screen.getByRole('button', { name: 'on' }))
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
    expect(localStorage.getItem(HINTS_KEY_ENABLED)).toBe('false')
    unmount()
    render(<App />)
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
  })

  it('persists a single dismissal and leaves other hints alone', () => {
    const { unmount } = render(<App />)
    fireEvent.click(screen.getAllByRole('button', { name: 'Dismiss this hint' })[0])
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
    expect(screen.getByText('Tip B')).toBeInTheDocument()
    unmount()
    render(<App />)
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
    expect(screen.getByText('Tip B')).toBeInTheDocument()
  })

  it('survives malformed stored values', () => {
    localStorage.setItem(HINTS_KEY_ENABLED, 'banana')
    localStorage.setItem(HINTS_KEY_DISMISSED, '{not json')
    render(<App />)
    expect(screen.getByText('Tip A')).toBeInTheDocument()
    expect(screen.getByText('Tip B')).toBeInTheDocument()
  })

  it('ignores non-array and non-string dismissed entries', () => {
    localStorage.setItem(HINTS_KEY_DISMISSED, JSON.stringify(['a', 3, null, 'a']))
    render(<App />)
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
    expect(screen.getByText('Tip B')).toBeInTheDocument()
  })

  it('does not crash when storage throws', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota')
    })
    render(<App />)
    expect(screen.getByText('Tip A')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'on' }))
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
  })

  it('syncs from other windows through the storage event', () => {
    render(<App />)
    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: HINTS_KEY_ENABLED, newValue: 'false' }))
    })
    expect(screen.queryByText('Tip A')).not.toBeInTheDocument()
    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: HINTS_KEY_ENABLED, newValue: 'true' }))
      window.dispatchEvent(new StorageEvent('storage', { key: HINTS_KEY_DISMISSED, newValue: '["b"]' }))
    })
    expect(screen.getByText('Tip A')).toBeInTheDocument()
    expect(screen.queryByText('Tip B')).not.toBeInTheDocument()
  })

  it('renders a keyboard reachable dismiss button', () => {
    render(<App />)
    const btn = screen.getAllByRole('button', { name: 'Dismiss this hint' })[0]
    btn.focus()
    expect(btn).toHaveFocus()
    expect(btn.className).toContain('focus-visible:outline')
  })

  it('behaves as off outside a provider', () => {
    render(<Hint id="x">Lonely</Hint>)
    expect(screen.queryByText('Lonely')).not.toBeInTheDocument()
  })
})
