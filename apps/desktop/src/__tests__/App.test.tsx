import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}))

// Screens own their own data and (for Memory) WebGL; the shell is under test here.
vi.mock('../screens/Intake', () => ({ Intake: () => <div data-testid="screen-intake">intake screen</div> }))
vi.mock('../screens/Examination', () => ({
  Examination: () => <div data-testid="screen-examination">examination screen</div>,
}))
vi.mock('../screens/Cockpit', () => ({ Cockpit: () => <div data-testid="screen-cockpit">cockpit screen</div> }))
vi.mock('../screens/MemoryGraph', () => ({
  MemoryGraph: () => <div data-testid="screen-memory">memory screen</div>,
}))
vi.mock('../screens/Spool', () => ({ Spool: () => <div data-testid="screen-spool">spool screen</div> }))
// TitleBar drives the real window API (getCurrentWindow) when Tauri is present; it has its own tests.
vi.mock('../components/TitleBar', () => ({ TitleBar: () => <div data-testid="titlebar" /> }))

import App from '../App'

const EXAM = { brain: '/b', tools: '/t', criticals: 0, fixable: 0, findings: [] }
const INTAKE = { site: '/s', total: 0, present: 0, findings: [] }

function backend(overrides: Partial<Record<string, () => unknown>> = {}) {
  invokeMock.mockImplementation(async (cmd: string) => {
    if (overrides[cmd]) return overrides[cmd]!()
    switch (cmd) {
      case 'examine':
        return EXAM
      case 'intake':
        return INTAKE
      case 'get_version':
        return '0.4.2'
      default:
        return null
    }
  })
}

const calls = (cmd: string) => invokeMock.mock.calls.filter(([c]) => c === cmd).length

/** mod = Ctrl in jsdom (navigator.platform is not mac). */
function pressMod(key: string) {
  fireEvent.keyDown(window, { key, ctrlKey: true })
}

function navButton(label: string) {
  return screen.getByRole('button', { name: new RegExp(`^${label}`) })
}

beforeEach(() => {
  localStorage.clear()
  ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
  invokeMock.mockReset()
  backend()
})

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})

async function renderApp() {
  render(<App />)
  await screen.findByTestId('screen-intake')
  await waitFor(() => expect(calls('examine')).toBe(1))
}

describe('App shell', () => {
  it('switches screens from the nav', async () => {
    await renderApp()
    fireEvent.click(navButton('Examination'))
    expect(await screen.findByTestId('screen-examination')).toBeInTheDocument()
    expect(navButton('Examination')).toHaveAttribute('aria-current', 'page')
    fireEvent.click(navButton('Cockpit'))
    expect(screen.getByTestId('screen-cockpit')).toBeInTheDocument()
    expect(navButton('Examination')).not.toHaveAttribute('aria-current')
  })

  it('switches screens with mod+1..mod+5', async () => {
    await renderApp()
    act(() => pressMod('3'))
    expect(screen.getByTestId('screen-cockpit')).toBeInTheDocument()
    act(() => pressMod('4'))
    expect(screen.getByTestId('screen-memory')).toBeInTheDocument()
    act(() => pressMod('5'))
    expect(screen.getByTestId('screen-spool')).toBeInTheDocument()
    act(() => pressMod('2'))
    expect(screen.getByTestId('screen-examination')).toBeInTheDocument()
    act(() => pressMod('1'))
    expect(await screen.findByTestId('screen-intake')).toBeInTheDocument()
  })

  it('shows each nav shortcut in the sidebar', async () => {
    await renderApp()
    expect(navButton('Intake')).toHaveTextContent('Ctrl 1')
    expect(navButton('Sensory Spool')).toHaveTextContent('Ctrl 5')
  })

  it('mod+h toggles hints', async () => {
    await renderApp()
    expect(document.querySelector('[data-hint="shell.nav"]')).not.toBeNull()
    act(() => pressMod('h'))
    expect(document.querySelector('[data-hint^="shell."]')).toBeNull()
    expect(screen.getByRole('button', { name: 'hints off' })).toBeInTheDocument()
    act(() => pressMod('h'))
    expect(document.querySelector('[data-hint="shell.nav"]')).not.toBeNull()
  })

  it('does not re-run the examination when switching screens', async () => {
    await renderApp()
    expect(calls('intake')).toBe(1)
    for (const k of ['2', '3', '4', '5', '2']) act(() => pressMod(k))
    fireEvent.click(navButton('Cockpit'))
    await new Promise((r) => setTimeout(r, 0))
    expect(calls('examine')).toBe(1)
    expect(calls('intake')).toBe(1)
    // Entering Intake re-reads intake only.
    act(() => pressMod('1'))
    await waitFor(() => expect(calls('intake')).toBe(2))
    expect(calls('examine')).toBe(1)
  })

  it('mod+r re-reads vitals', async () => {
    await renderApp()
    act(() => pressMod('r'))
    await waitFor(() => expect(calls('examine')).toBe(2))
  })

  it('offers a retry when the examination fails and clears the error on success', async () => {
    let fail = true
    backend({
      examine: () => {
        if (fail) throw new Error('core unreachable')
        return EXAM
      },
    })
    render(<App />)
    fireEvent.click(navButton('Examination'))
    expect(await screen.findByText('examination halted.')).toBeInTheDocument()
    fail = false
    fireEvent.click(screen.getByRole('button', { name: 'retry' }))
    expect(await screen.findByTestId('screen-examination')).toBeInTheDocument()
    expect(screen.queryByText('examination halted.')).toBeNull()
  })

  it('names the next command when intake fails, and retries', async () => {
    let fail = true
    backend({
      intake: () => {
        if (fail) throw new Error('site unreadable')
        return INTAKE
      },
    })
    render(<App />)
    expect(await screen.findByText('intake halted.')).toBeInTheDocument()
    expect(screen.getByText('synapse doctor')).toBeInTheDocument()
    fail = false
    fireEvent.click(screen.getByRole('button', { name: 'retry' }))
    expect(await screen.findByTestId('screen-intake')).toBeInTheDocument()
  })

  it('never shows an unmeasured version', async () => {
    backend({
      get_version: () => {
        throw new Error('no version')
      },
    })
    await renderApp()
    expect(await screen.findByText('version unknown')).toBeInTheDocument()
    expect(screen.queryByText(/v1\.0\.0/)).toBeNull()
  })

  it('explains why re-read is disabled outside Tauri', async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
    render(<App />)
    const btn = screen.getByRole('button', { name: 're-read vitals' })
    expect(btn).toBeDisabled()
    expect(btn.getAttribute('title')).toMatch(/No backend attached/)
    expect(invokeMock).not.toHaveBeenCalled()
  })
})
