import { act, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { HintsProvider } from '../../lib/hints'
import { registerShortcut } from '../../lib/shortcuts'
import { CommandReference } from '../CommandReference'

function setup() {
  const utils = render(
    <HintsProvider>
      <button type="button">before</button>
      <CommandReference />
    </HintsProvider>,
  )
  return { ...utils, user: userEvent.setup() }
}

beforeEach(() => {
  vi.spyOn(navigator, 'platform', 'get').mockReturnValue('Linux x86_64')
  localStorage.clear()
})
afterEach(() => vi.restoreAllMocks())

describe('CommandReference', () => {
  it('is closed by default and opens with "?"', async () => {
    const { user } = setup()
    expect(screen.queryByRole('dialog')).toBeNull()
    await user.keyboard('?')
    const dialog = screen.getByRole('dialog')
    expect(dialog).toHaveAttribute('aria-modal', 'true')
    expect(dialog).toHaveAccessibleName('Command reference')
    expect(screen.getByRole('searchbox')).toHaveFocus()
  })

  it('opens with mod+k and mod+/', async () => {
    const { user } = setup()
    await user.keyboard('{Control>}k{/Control}')
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog')).toBeNull()
    await user.keyboard('{Control>}/{/Control}')
    expect(screen.getByRole('dialog')).toBeInTheDocument()
  })

  it('"?" does not open while typing in a text field', async () => {
    const user = userEvent.setup()
    render(
      <>
        <input aria-label="name" />
        <CommandReference />
      </>,
    )
    await user.click(screen.getByLabelText('name'))
    await user.keyboard('?')
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(screen.getByLabelText('name')).toHaveValue('?')
  })

  it('Esc closes from the search box and restores focus', async () => {
    const { user } = setup()
    const before = screen.getByRole('button', { name: 'before' })
    before.focus()
    await user.keyboard('?')
    expect(screen.getByRole('searchbox')).toHaveFocus()
    // bare keys typed into the search do not fire shortcuts
    await user.keyboard('?')
    expect(screen.getByRole('searchbox')).toHaveValue('?')
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(before).toHaveFocus()
  })

  it('closes via the close button and the backdrop', async () => {
    const { user } = setup()
    await user.keyboard('?')
    await user.click(screen.getByRole('button', { name: 'Close command reference' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    await user.keyboard('?')
    fireEvent.mouseDown(screen.getByRole('dialog'))
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    fireEvent.mouseDown(screen.getByTestId('command-reference-backdrop'))
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('lists registered shortcuts with kbd chips and the CLI section', async () => {
    const { user } = setup()
    await user.keyboard('?')
    const dialog = screen.getByRole('dialog')
    expect(within(dialog).getByText('Help')).toBeInTheDocument()
    expect(within(dialog).getAllByText('Open command reference').length).toBe(3)
    expect(within(dialog).getByText('Close command reference')).toBeInTheDocument()
    const row = dialog.querySelector('[data-shortcut-id="help.reference.k"]')!
    expect([...row.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['Ctrl', 'K'])
    expect(within(dialog).getByText('CLI')).toBeInTheDocument()
    expect(within(dialog).getByText('synapse doctor [--fix]')).toBeInTheDocument()
    expect(within(dialog).getByText('synapse serve [--host] [--port]')).toBeInTheDocument()
  })

  it('search filters both sections', async () => {
    const { user } = setup()
    await user.keyboard('?')
    await user.type(screen.getByRole('searchbox'), 'rollback')
    const dialog = screen.getByRole('dialog')
    expect(within(dialog).getByText('synapse rollback <hash>')).toBeInTheDocument()
    expect(within(dialog).queryByText('synapse scan')).toBeNull()
    expect(within(dialog).queryByText('Open command reference')).toBeNull()
    await user.clear(screen.getByRole('searchbox'))
    await user.type(screen.getByRole('searchbox'), 'close command')
    expect(within(dialog).getByText('Close command reference')).toBeInTheDocument()
    expect(within(dialog).queryByText('CLI')).toBeNull()
    await user.clear(screen.getByRole('searchbox'))
    await user.type(screen.getByRole('searchbox'), 'zzzz-nothing')
    expect(within(dialog).getByText(/No matches/)).toBeInTheDocument()
  })

  it('hints switch toggles useHints().enabled', async () => {
    const { user } = setup()
    await user.keyboard('?')
    const sw = screen.getByRole('switch', { name: 'Show hints' })
    expect(sw).toHaveAttribute('aria-checked', 'true')
    await user.click(sw)
    expect(sw).toHaveAttribute('aria-checked', 'false')
    expect(localStorage.getItem('synapse.hints.enabled')).toBe('false')
    await user.click(sw)
    expect(sw).toHaveAttribute('aria-checked', 'true')
    expect(screen.getByText(/turned back on here/)).toBeInTheDocument()
  })

  it('shows a newly registered shortcut live', async () => {
    const { user } = setup()
    await user.keyboard('?')
    expect(screen.queryByText('Re-scan tools')).toBeNull()
    let off: () => void = () => {}
    act(() => {
      off = registerShortcut({ id: 'test.rescan', keys: 'r', label: 'Re-scan tools', group: 'Actions' }, () => {})
    })
    expect(screen.getByText('Re-scan tools')).toBeInTheDocument()
    expect(screen.getByText('Actions')).toBeInTheDocument()
    act(() => off())
    expect(screen.queryByText('Re-scan tools')).toBeNull()
  })

  it('traps focus inside the dialog', async () => {
    const { user } = setup()
    await user.keyboard('?')
    const dialog = screen.getByRole('dialog')
    for (let i = 0; i < 6; i++) {
      await user.tab()
      expect(dialog.contains(document.activeElement)).toBe(true)
    }
    await user.tab({ shift: true })
    expect(dialog.contains(document.activeElement)).toBe(true)
  })
})
