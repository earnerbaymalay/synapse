import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ExamReport, Finding } from '../../screens/types'

const invokeMock = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }))

import { AlertBanner } from '../AlertBanner'

const finding = (over: Partial<Finding> = {}): Finding => ({
  rule_id: 'projection-missing',
  severity: 'critical',
  message: 'projection missing',
  subject: '.claude/CLAUDE.md',
  auto_fixable: true,
  ...over,
})

const exam = (findings: Finding[]): ExamReport => ({
  brain: '/home/mara/AIBrain',
  tools: '/home/mara',
  criticals: findings.filter((f) => f.severity === 'critical').length,
  fixable: findings.filter((f) => f.auto_fixable).length,
  findings,
})

function deferred<T>() {
  let resolve!: (v: T) => void
  let reject!: (e: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

beforeEach(() => {
  invokeMock.mockReset()
})

describe('AlertBanner states', () => {
  it('exam null reports vitals as not read, never as healthy', () => {
    const { container } = render(<AlertBanner exam={null} onHealed={() => {}} />)
    expect(screen.getByRole('status')).toHaveTextContent(/vitals not read/i)
    expect(container.textContent).not.toMatch(/healthy|synced|signed|clean/i)
    expect(screen.queryByRole('button')).toBeNull()
    expect(container.querySelector('[data-level]')).toHaveAttribute('data-level', 'unread')
    expect(container.textContent).toContain('○')
  })

  it('healthy exam shows the glyph, the words and no hard-coded counts', () => {
    const { container } = render(<AlertBanner exam={exam([])} onHealed={() => {}} />)
    expect(screen.getByRole('status')).toHaveTextContent(/no faults/i)
    expect(container.textContent).toContain('●')
    expect(container.textContent).not.toMatch(/15 grafts|Core Harness|Doctor: Clean/)
    expect(screen.getByText('/home/mara/AIBrain')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /heal/i })).toBeNull()
  })

  it('warnings only is a warning state, not a critical one', () => {
    const { container } = render(
      <AlertBanner exam={exam([finding({ severity: 'warning', auto_fixable: false })])} onHealed={() => {}} />,
    )
    expect(container.querySelector('[data-level]')).toHaveAttribute('data-level', 'warning')
    expect(screen.getByRole('status')).toHaveTextContent(/1 warning/i)
    expect(container.textContent).toContain('▲')
    expect(container.textContent).not.toMatch(/critical finding/i)
    // nothing fixable: no heal offered
    expect(screen.queryByRole('button', { name: /heal/i })).toBeNull()
  })

  it('critical exam shows the count, glyph and the heal action with an honest snapshot note', () => {
    const { container } = render(<AlertBanner exam={exam([finding(), finding({ subject: 'b' })])} onHealed={() => {}} />)
    expect(container.querySelector('[data-level]')).toHaveAttribute('data-level', 'critical')
    expect(screen.getByRole('status')).toHaveTextContent(/2 critical findings/i)
    expect(container.textContent).toContain('■')
    expect(screen.getByRole('button', { name: /heal 2 findings/i })).toBeEnabled()
    expect(screen.getByText(/a snapshot of the brain is taken first/i)).toBeInTheDocument()
  })

  it('names Zone 1 tamper in the headline', () => {
    render(<AlertBanner exam={exam([finding({ rule_id: 'master-prompt-tampered' })])} onHealed={() => {}} />)
    expect(screen.getByRole('status')).toHaveTextContent(/zone 1 altered/i)
  })
})

describe('AlertBanner actions', () => {
  it('heal shows progress, blocks double submit, reports the real count, then calls onHealed', async () => {
    const user = userEvent.setup()
    const d = deferred<number>()
    invokeMock.mockReturnValue(d.promise)
    const onHealed = vi.fn()
    render(<AlertBanner exam={exam([finding(), finding({ subject: 'b' })])} onHealed={onHealed} />)

    const btn = screen.getByRole('button', { name: /heal 2 findings/i })
    await user.dblClick(btn)
    await user.click(btn)
    expect(invokeMock).toHaveBeenCalledTimes(1)
    expect(invokeMock).toHaveBeenCalledWith('heal_vitals', { brain: null, tools: null })
    expect(btn).toHaveAttribute('aria-disabled', 'true')
    expect(btn).toHaveFocus() // focus is not thrown to the page while healing
    expect(btn).toHaveTextContent(/healing/i)
    expect(onHealed).not.toHaveBeenCalled()

    d.resolve(1)
    await waitFor(() => expect(screen.getByText(/healed 1 of 2 auto-fixable findings/i)).toBeInTheDocument())
    expect(onHealed).toHaveBeenCalledTimes(1)
    expect(btn).not.toHaveAttribute('aria-disabled')
  })

  it('heal error shows the backend error and does not call onHealed', async () => {
    const user = userEvent.setup()
    invokeMock.mockRejectedValue('permission denied: /home/mara/AIBrain/SYNAPSE.md')
    const onHealed = vi.fn()
    render(<AlertBanner exam={exam([finding()])} onHealed={onHealed} />)
    await user.click(screen.getByRole('button', { name: /heal 1 finding/i }))
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Heal failed: permission denied: /home/mara/AIBrain/SYNAPSE.md')
    expect(onHealed).not.toHaveBeenCalled()

    // the result can be dismissed from the keyboard
    screen.getByRole('button', { name: /dismiss heal result/i }).focus()
    await user.keyboard('{Enter}')
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('a missing master prompt is healed, not routed to a dead-end screen', () => {
    // doctor marks master-prompt-missing auto_fixable and heal regenerates SYNAPSE.md,
    // so the banner offers Heal and no separate "configure" button that went nowhere.
    const onOpenOnboard = vi.fn()
    render(
      <AlertBanner
        exam={exam([finding({ rule_id: 'master-prompt-missing', auto_fixable: true })])}
        onHealed={() => {}}
        onOpenOnboard={onOpenOnboard}
      />,
    )
    expect(screen.queryByRole('button', { name: /configure master prompt/i })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /heal 1 finding/i })).toBeInTheDocument()
    expect(onOpenOnboard).not.toHaveBeenCalled()
  })

  it('findings disclosure toggles with the keyboard and reports aria-expanded', async () => {
    const user = userEvent.setup()
    render(<AlertBanner exam={exam([finding({ message: 'symlink detached' })])} onHealed={() => {}} />)
    const toggle = screen.getByRole('button', { name: /show 1 finding/i })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    toggle.focus()
    await user.keyboard('{Enter}')
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText('symlink detached')).toBeInTheDocument()
    await user.keyboard(' ')
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByText('symlink detached')).toBeNull()
  })
})
