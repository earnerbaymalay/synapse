import { describe, expect, it, beforeEach } from 'vitest'
import type { ReactElement } from 'react'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Examination, bySeverity, subject, tally } from '../Examination'
import { HintsProvider } from '../../lib/hints'
import { listShortcuts } from '../../lib/shortcuts'
import type { ExamReport, Finding } from '../types'

const BRAIN = '/home/mara/AIBrain'
const TOOLS = '/home/mara/work/atlas'

const f = (over: Partial<Finding> = {}): Finding => ({
  rule_id: 'r',
  severity: 'info',
  message: 'm',
  subject: null,
  auto_fixable: false,
  ...over,
})

const report = (over: Partial<ExamReport> = {}): ExamReport => ({
  brain: BRAIN,
  tools: TOOLS,
  criticals: 0,
  fixable: 0,
  findings: [],
  ...over,
})

// Deliberately out of severity order, as a backend might send them.
const mixed = report({
  criticals: 1,
  fixable: 2,
  findings: [
    f({ rule_id: 'mappings-unsorted', severity: 'info', message: 'Entries are unordered.', subject: `${BRAIN}/.brain/mappings.json`, auto_fixable: true }),
    f({ rule_id: 'canonical-source-missing', severity: 'critical', message: 'Source no longer exists.', subject: `${BRAIN}/skills/gone` }),
    f({ rule_id: 'projection-missing', severity: 'warning', message: 'Projection was deleted.', subject: `${TOOLS}/.cursor/rules/a.md`, auto_fixable: true }),
  ],
})

const LONG_PATH = `${TOOLS}/${'very-deep-directory/'.repeat(20)}file-with-a-long-name.md`
const LONG_MSG = 'The projection differs from its canonical source '.repeat(12).trim()

function withHints(ui: ReactElement) {
  return render(<HintsProvider>{ui}</HintsProvider>)
}

beforeEach(() => {
  localStorage.clear()
})

describe('Examination — zero findings', () => {
  it('states absence, a zero tally and the next command', () => {
    render(<Examination report={report()} />)
    expect(screen.getByText('no findings')).toBeInTheDocument()
    expect(screen.getByTestId('exam-tally')).toHaveTextContent('0 critical · 0 warning · 0 info · 0 fixable')
    expect(screen.getByText('no drift, no faults')).toBeInTheDocument()
    expect(screen.getByText('Clean bill of health.')).toBeInTheDocument()
    expect(screen.getByText('synapse sync --once')).toBeInTheDocument()
    expect(screen.queryByRole('list', { name: /findings/i })).not.toBeInTheDocument()
  })

  it('shows no severity or --fix hints when clean', () => {
    withHints(<Examination report={report()} />)
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
  })
})

describe('Examination — mixed severities', () => {
  it('orders rows critical, warning, info', () => {
    render(<Examination report={mixed} />)
    const rows = within(screen.getByRole('list', { name: /findings/i })).getAllByRole('listitem')
    expect(rows.map((r) => r.getAttribute('data-severity'))).toEqual(['critical', 'warning', 'info'])
  })

  it('pairs every glyph with its severity in words', () => {
    render(<Examination report={mixed} />)
    const rows = screen.getAllByRole('listitem')
    expect(rows[0]).toHaveTextContent('■')
    expect(rows[0]).toHaveTextContent('critical')
    expect(rows[1]).toHaveTextContent('▲')
    expect(rows[1]).toHaveTextContent('warning')
    expect(rows[2]).toHaveTextContent('◐')
    expect(rows[2]).toHaveTextContent('info')
  })

  it('counts what it lists, even when the report totals disagree', () => {
    // Report claims 5 criticals / 9 fixable; only the listed rows count.
    render(<Examination report={{ ...mixed, criticals: 5, fixable: 9 }} />)
    expect(screen.getByText('3 findings')).toBeInTheDocument()
    expect(screen.getAllByRole('listitem')).toHaveLength(3)
    expect(screen.getByTestId('exam-tally')).toHaveTextContent('1 critical · 1 warning · 1 info · 2 fixable')
    expect(screen.getByText('1 critical finding needs a human. 1 warning can wait.')).toBeInTheDocument()
  })

  it('names --fix when anything is fixable, per row and in the footer', () => {
    render(<Examination report={mixed} />)
    expect(screen.getByText('synapse doctor --fix')).toBeInTheDocument()
    expect(screen.getAllByText('fixable with --fix')).toHaveLength(2)
    expect(screen.getAllByText('fix by hand')).toHaveLength(1)
    expect(screen.getByText('rule canonical-source-missing')).toBeInTheDocument()
  })

  it('still names a next command when criticals are not fixable', () => {
    render(<Examination report={report({ criticals: 2, findings: [f({ severity: 'critical' }), f({ severity: 'critical' })] })} />)
    expect(screen.getByText('2 critical findings need a human.')).toBeInTheDocument()
    expect(screen.getByText('synapse doctor')).toBeInTheDocument()
  })

  it('reports warnings-only as observations and points at sync', () => {
    render(<Examination report={report({ findings: [f({ severity: 'warning' })] })} />)
    expect(screen.getByText('No critical findings. 1 observation noted.')).toBeInTheDocument()
    expect(screen.getByText('synapse sync --once')).toBeInTheDocument()
  })
})

describe('Examination — long values', () => {
  it('truncates the subject but keeps the full path in title, and keeps the full message', () => {
    render(<Examination report={report({ findings: [f({ subject: LONG_PATH, message: LONG_MSG })] })} />)
    const short = LONG_PATH.replace(TOOLS, 'tools')
    const cell = screen.getByText(short)
    expect(cell).toHaveAttribute('title', LONG_PATH)
    expect(cell.className).toMatch(/truncate/)
    expect(screen.getByText(LONG_MSG)).toBeInTheDocument()
  })
})

describe('Examination — interactions', () => {
  it('copy button writes the next command to the clipboard and confirms', async () => {
    const user = userEvent.setup()
    render(<Examination report={mixed} />)
    await user.click(screen.getByRole('button', { name: 'Copy synapse doctor --fix' }))
    expect(await navigator.clipboard.readText()).toBe('synapse doctor --fix')
    expect(screen.getByRole('status')).toHaveTextContent('copied')
  })

  it('reports a failed copy instead of claiming success', async () => {
    const user = userEvent.setup()
    render(<Examination report={report()} />)
    const original = navigator.clipboard.writeText
    navigator.clipboard.writeText = () => Promise.reject(new Error('denied'))
    try {
      await user.click(screen.getByRole('button', { name: 'Copy synapse sync --once' }))
      expect(await screen.findByRole('alert')).toHaveTextContent('clipboard unavailable')
      expect(screen.getByRole('status')).toHaveTextContent('')
    } finally {
      navigator.clipboard.writeText = original
    }
  })

  it('C copies the next command, and the shortcut lives only while the screen is mounted', async () => {
    const user = userEvent.setup()
    const { unmount } = render(<Examination report={mixed} />)
    expect(listShortcuts().find((s) => s.id === 'examination.copy-next')).toMatchObject({ keys: 'c', group: 'Actions' })
    await user.keyboard('c')
    expect(await navigator.clipboard.readText()).toBe('synapse doctor --fix')
    unmount()
    expect(listShortcuts().some((s) => s.id === 'examination.copy-next')).toBe(false)
  })

  it('keyboard order reaches hint dismissals then copy', async () => {
    const user = userEvent.setup()
    withHints(<Examination report={mixed} />)
    await user.tab()
    expect(document.activeElement).toHaveAccessibleName('Dismiss this hint')
    await user.tab()
    expect(document.activeElement).toHaveAccessibleName('Copy synapse doctor --fix')
    await user.tab()
    expect(document.activeElement).toHaveAccessibleName('Dismiss this hint')
  })

  it('hints explain the glyphs and --fix, and each dismiss removes only its own hint', async () => {
    const user = userEvent.setup()
    withHints(<Examination report={mixed} />)
    const notes = screen.getAllByRole('note')
    expect(notes).toHaveLength(2)
    expect(notes[0]).toHaveAttribute('data-hint', 'examination.glyphs')
    expect(notes[1]).toHaveAttribute('data-hint', 'examination.fix')
    expect(notes[1]).toHaveTextContent('does not take a snapshot first')
    for (const n of notes) expect(n.textContent).not.toMatch(/!/)

    await user.click(within(notes[0]).getByRole('button', { name: 'Dismiss this hint' }))
    expect(document.querySelector('[data-hint="examination.glyphs"]')).toBeNull()
    expect(document.querySelector('[data-hint="examination.fix"]')).not.toBeNull()

    await user.click(screen.getByRole('button', { name: 'Dismiss this hint' }))
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
  })

  it('has no emoji in any state', () => {
    const emoji = /\p{Extended_Pictographic}/u
    for (const r of [report(), mixed]) {
      const { container, unmount } = withHints(<Examination report={r} />)
      expect(emoji.test(container.textContent ?? '')).toBe(false)
      unmount()
    }
  })
})

describe('helpers', () => {
  it('subject requires a path boundary after the root', () => {
    const roots = { brain: BRAIN, tools: TOOLS }
    expect(subject(f({ subject: `${BRAIN}2/skills/x` }), roots)).toBe(`${BRAIN}2/skills/x`)
    expect(subject(f({ subject: BRAIN }), roots)).toBe('brain')
    expect(subject(f({ subject: `${BRAIN}/skills` }), { brain: `${BRAIN}/`, tools: TOOLS })).toBe('brain/skills')
    expect(subject(f({ subject: null }), roots)).toBe('brain')
    expect(subject(f({ subject: '/etc/hosts' }), roots)).toBe('/etc/hosts')
  })

  it('brain wins when roots nest, like the CLI', () => {
    expect(subject(f({ subject: '/home/u/AIBrain/skills' }), { brain: '/home/u/AIBrain', tools: '/home/u' })).toBe('brain/skills')
  })

  it('bySeverity is stable and tally matches', () => {
    const list = [f({ rule_id: 'a' }), f({ rule_id: 'b', severity: 'critical' }), f({ rule_id: 'c' })]
    expect(bySeverity(list).map((x) => x.rule_id)).toEqual(['b', 'a', 'c'])
    expect(tally(list)).toEqual({ total: 3, critical: 1, warning: 0, info: 2, fixable: 0 })
  })
})
