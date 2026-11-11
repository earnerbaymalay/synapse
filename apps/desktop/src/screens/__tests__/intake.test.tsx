import { describe, expect, it, vi, beforeEach } from 'vitest'
import { render, screen, within, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Intake } from '../Intake'
import { HintsProvider } from '../../lib/hints'
import { listShortcuts } from '../../lib/shortcuts'
import type { IntakeReport, ToolFinding } from '../types'

const ADAPTERS = [
  'aider', 'claude-code', 'cline', 'codex', 'continue', 'copilot', 'cursor', 'gemini',
  'goose', 'jetbrains', 'kiro', 'opencode', 'roo', 'windsurf', 'zed',
]

const absent = (id: string): ToolFinding => ({
  id, present: false, skills: null, agents: null, mcp_servers: null, error: null,
})
const read = (id: string, skills: number, agents: number, mcp_servers: number): ToolFinding => ({
  id, present: true, skills, agents, mcp_servers, error: null,
})
const broken = (id: string, error: string): ToolFinding => ({
  id, present: true, skills: null, agents: null, mcp_servers: null, error,
})

function report(findings: ToolFinding[], site = '/home/mara/work/atlas'): IntakeReport {
  return {
    site,
    total: findings.length,
    present: findings.filter((f) => f.present).length,
    findings,
  }
}

const allPresent = report(ADAPTERS.map((id, i) => read(id, i, i % 2, 1)))
const nonePresent = report(ADAPTERS.map(absent))
const mixed = report([
  absent('zed'),
  read('cline', 6, 0, 0),
  broken('copilot', 'malformed config: not valid UTF-8'),
  absent('aider'),
  read('claude-code', 1, 2, 3),
])

const context = () => screen.getByTestId('intake-context').textContent
const nextCommand = () => screen.getByText('Next').parentElement?.textContent?.replace(/^Next\s*/, '')
const rowIds = () =>
  screen.getAllByRole('listitem').map((el) => el.getAttribute('data-tool'))

beforeEach(() => {
  localStorage.clear()
})

describe('Intake counts', () => {
  it('all present: header, rows and totals match the report', () => {
    render(<Intake report={allPresent} />)
    expect(context()).toBe('15 of 15 present')
    expect(screen.getAllByRole('listitem')).toHaveLength(15)
    expect(screen.queryByText('not present')).not.toBeInTheDocument()
    // skills 0..14 = 105, agents = 7 odd indices, servers = 15
    expect(
      screen.getByText(/105 skills, 7 agents and 15 mcp servers would enter the Brain from 15 tools\./),
    ).toBeInTheDocument()
    expect(nextCommand()).toBe('synapse import --dry-run')
  })

  it('none present: every tool still gets a row and a next command is named', () => {
    render(<Intake report={nonePresent} />)
    expect(context()).toBe('0 of 15 present')
    expect(screen.getAllByText('not present')).toHaveLength(15)
    expect(screen.getByText(/None of the 15 supported tools has a config/)).toBeInTheDocument()
    expect(nextCommand()).toBe('synapse onboard')
  })

  it('mixed: counts only readable tools and reports the unreadable one', () => {
    render(<Intake report={mixed} />)
    expect(context()).toBe('3 of 5 present')
    expect(
      screen.getByText(
        '7 skills, 2 agents and 3 mcp servers would enter the Brain from 2 tools. ' +
          '1 tool could not be read. Nothing has been written.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByText('detected, but could not be read')).toBeInTheDocument()
    expect(screen.getByTitle('malformed config: not valid UTF-8')).toBeInTheDocument()
    expect(nextCommand()).toBe('synapse doctor')
  })

  it('sorts present first, then absent, alphabetical within each group', () => {
    render(<Intake report={mixed} />)
    expect(rowIds()).toEqual(['claude-code', 'cline', 'copilot', 'aider', 'zed'])
  })

  it('derives the header from the rows, not from stale summary fields', () => {
    render(<Intake report={{ ...mixed, present: 9, total: 99 }} />)
    expect(context()).toBe('3 of 5 present')
  })

  it('empty findings array: says nothing was scanned and still names a next command', () => {
    render(<Intake report={report([])} />)
    expect(context()).toBe('0 of 0 present')
    expect(screen.queryAllByRole('listitem')).toHaveLength(0)
    expect(screen.getByText(/No tool adapters are registered/)).toBeInTheDocument()
    expect(nextCommand()).toBe('synapse doctor')
  })

  it('every present tool unreadable: no claim that anything would enter the Brain', () => {
    render(<Intake report={report([broken('cline', 'boom'), absent('zed')])} />)
    expect(screen.queryByText(/would enter the Brain/)).not.toBeInTheDocument()
    expect(screen.getByText('1 tool could not be read. Nothing has been written.')).toBeInTheDocument()
    expect(nextCommand()).toBe('synapse doctor')
  })

  it('a present row with null counts shows "not measured", never zero', () => {
    const f: ToolFinding = { id: 'cursor', present: true, skills: 4, agents: null, mcp_servers: 0, error: null }
    render(<Intake report={report([f])} />)
    const row = screen.getByRole('listitem')
    expect(within(row).getByText('agents not measured')).toBeInTheDocument()
    expect(within(row).queryByText('0 agents')).not.toBeInTheDocument()
    expect(within(row).getByText('◐')).toBeInTheDocument()
    expect(screen.getByText(/1 tool could not be fully measured/)).toBeInTheDocument()
    // the partly measured tool does not contribute to the import total
    expect(screen.queryByText(/would enter the Brain/)).not.toBeInTheDocument()
  })

  it('uses only the house glyph set', () => {
    const { container } = render(<Intake report={mixed} />)
    expect(container.textContent).toMatch(/●/)
    expect(container.textContent).toMatch(/■/)
    expect(container.textContent).toMatch(/○/)
    // no emoji
    expect(container.textContent).not.toMatch(/\p{Extended_Pictographic}/u)
  })
})

describe('Intake long values', () => {
  const longSite = '/home/mara/' + 'very-deep-directory/'.repeat(20) + 'atlas'
  const longError = 'could not parse ' + '/x/'.repeat(80) + 'settings.json'

  it('truncates the site with the full path in its title', () => {
    render(<Intake report={report([read('cline', 1, 1, 1)], longSite)} />)
    const site = screen.getByTestId('intake-site')
    expect(site).toHaveAttribute('title', longSite)
    expect(site).toHaveTextContent(longSite)
    expect(site.className).toMatch(/\btruncate\b/)
  })

  it('keeps a long error whole and wraps it', () => {
    render(<Intake report={report([broken('cline', longError)])} />)
    const el = screen.getByTitle(longError)
    expect(el).toHaveTextContent(longError)
    expect(el.className).toMatch(/break-words/)
  })
})

describe('Intake re-scan', () => {
  it('shows no re-scan control or shortcut when none is wired', () => {
    render(<Intake report={mixed} />)
    expect(screen.queryByRole('button', { name: /re-scan/ })).not.toBeInTheDocument()
    expect(listShortcuts().find((s) => s.id === 'intake.rescan')).toBeUndefined()
  })

  it('clicking re-scan calls onRescan', async () => {
    const onRescan = vi.fn()
    render(<Intake report={mixed} onRescan={onRescan} />)
    await userEvent.click(screen.getByRole('button', { name: 're-scan' }))
    expect(onRescan).toHaveBeenCalledTimes(1)
  })

  it('is keyboard reachable and fires on Enter', async () => {
    const user = userEvent.setup()
    const onRescan = vi.fn()
    render(<Intake report={mixed} onRescan={onRescan} />)
    await user.tab()
    expect(screen.getByRole('button', { name: 're-scan' })).toHaveFocus()
    await user.keyboard('{Enter}')
    expect(onRescan).toHaveBeenCalledTimes(1)
  })

  it('R registers under Actions and triggers a re-scan; unregisters on unmount', async () => {
    const onRescan = vi.fn()
    const { unmount } = render(<Intake report={mixed} onRescan={onRescan} />)
    const def = listShortcuts().find((s) => s.id === 'intake.rescan')
    expect(def).toMatchObject({ keys: 'r', group: 'Actions' })
    await userEvent.keyboard('r')
    expect(onRescan).toHaveBeenCalledTimes(1)
    unmount()
    expect(listShortcuts().find((s) => s.id === 'intake.rescan')).toBeUndefined()
    fireEvent.keyDown(window, { key: 'r' })
    expect(onRescan).toHaveBeenCalledTimes(1)
  })

  it('while busy: button disabled, label changes, R does nothing', async () => {
    const onRescan = vi.fn()
    render(<Intake report={mixed} onRescan={onRescan} busy />)
    const btn = screen.getByRole('button', { name: 're-scanning…' })
    expect(btn).toBeDisabled()
    await userEvent.click(btn)
    await userEvent.keyboard('r')
    expect(onRescan).not.toHaveBeenCalled()
  })
})

describe('Intake hints', () => {
  it('renders no hints outside a provider', () => {
    render(<Intake report={mixed} />)
    expect(screen.queryAllByRole('note')).toHaveLength(0)
  })

  it('shows the intake hints on first run and each can be dismissed', async () => {
    const user = userEvent.setup()
    render(
      <HintsProvider>
        <Intake report={mixed} />
      </HintsProvider>,
    )
    const notes = screen.getAllByRole('note')
    expect(notes.map((n) => n.getAttribute('data-hint'))).toEqual(['intake.present', 'intake.next'])
    for (const n of notes) {
      expect(n.textContent).not.toMatch(/!/)
    }
    await user.click(within(notes[0]).getByRole('button', { name: 'Dismiss this hint' }))
    expect(screen.getAllByRole('note').map((n) => n.getAttribute('data-hint'))).toEqual(['intake.next'])
    await user.click(screen.getByRole('button', { name: 'Dismiss this hint' }))
    expect(screen.queryAllByRole('note')).toHaveLength(0)
  })

  it('omits the "present" hint when there are no rows to explain', () => {
    render(
      <HintsProvider>
        <Intake report={report([])} />
      </HintsProvider>,
    )
    expect(screen.getAllByRole('note').map((n) => n.getAttribute('data-hint'))).toEqual(['intake.next'])
  })
})
