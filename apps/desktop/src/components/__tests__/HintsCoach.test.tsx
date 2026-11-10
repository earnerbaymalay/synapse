import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { HintsProvider, HINTS_KEY_ENABLED } from '../../lib/hints'
import { COACH_KEY_DISMISSED, HintsCoach } from '../HintsCoach'

function App() {
  return (
    <HintsProvider>
      <button type="button">elsewhere</button>
      <HintsCoach />
    </HintsProvider>
  )
}

beforeEach(() => localStorage.clear())

describe('HintsCoach', () => {
  it('shows on first boot as a polite status without taking focus', () => {
    render(<App />)
    const panel = screen.getByRole('status')
    expect(panel).toHaveTextContent('Hints are on.')
    expect(panel).toHaveTextContent('lists every shortcut.')
    expect(panel).toHaveAttribute('aria-live', 'polite')
    expect(document.activeElement).toBe(document.body)
  })

  it('hides and turns hints off with "Turn hints off"', () => {
    render(<App />)
    fireEvent.click(screen.getByRole('button', { name: 'Turn hints off' }))
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    expect(localStorage.getItem(HINTS_KEY_ENABLED)).toBe('false')
  })

  it('does not render when hints are already off', () => {
    localStorage.setItem(HINTS_KEY_ENABLED, 'false')
    render(<App />)
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('"Got it" dismisses only the coach, and it stays dismissed', () => {
    const { unmount } = render(<App />)
    fireEvent.click(screen.getByRole('button', { name: 'Got it' }))
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    expect(localStorage.getItem(COACH_KEY_DISMISSED)).toBe('true')
    expect(localStorage.getItem(HINTS_KEY_ENABLED)).toBe('true')
    unmount()
    render(<App />)
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('renders nothing outside a provider', () => {
    render(<HintsCoach />)
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('survives a malformed coach value', () => {
    localStorage.setItem(COACH_KEY_DISMISSED, '{{')
    render(<App />)
    expect(screen.getByRole('status')).toBeInTheDocument()
  })
})
