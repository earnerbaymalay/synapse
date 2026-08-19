import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Intake } from '../Intake'
import { Examination } from '../Examination'
import type { ExamReport, IntakeReport } from '../types'

/**
 * These assert the identity's rules, not just that the components render:
 * absence is reported, unmeasured is distinct from zero, and no surface is a
 * dead end. Those are the properties that stop the UI drifting back into a
 * pretty mock — see IDENTITY.md.
 */

const intakeReport = (over: Partial<IntakeReport> = {}): IntakeReport => ({
  site: '/home/mara/work/atlas',
  total: 3,
  present: 1,
  findings: [
    { id: 'cline', present: true, skills: 6, agents: 0, mcp_servers: 0, error: null },
    { id: 'aider', present: false, skills: null, agents: null, mcp_servers: null, error: null },
    { id: 'zed', present: false, skills: null, agents: null, mcp_servers: null, error: null },
  ],
  ...over,
})

const examReport = (over: Partial<ExamReport> = {}): ExamReport => ({
  brain: '/home/mara/AIBrain',
  tools: '/home/mara/work/atlas',
  criticals: 0,
  fixable: 0,
  findings: [],
  ...over,
})

describe('Intake', () => {
  it('renders a row for tools that are not installed', () => {
    render(<Intake report={intakeReport()} />)
    // Absence is a finding, not an empty table.
    expect(screen.getByText('aider')).toBeInTheDocument()
    expect(screen.getByText('zed')).toBeInTheDocument()
    expect(screen.getAllByText('not present')).toHaveLength(2)
  })

  it('names the next command to run', () => {
    render(<Intake report={intakeReport()} />)
    expect(screen.getByText('neurosurgeon import --dry-run')).toBeInTheDocument()
  })

  it('states that nothing was written', () => {
    render(<Intake report={intakeReport()} />)
    expect(screen.getByText(/Nothing has been written/)).toBeInTheDocument()
  })

  it('surfaces an unreadable config as a critical row rather than dropping it', () => {
    render(
      <Intake
        report={intakeReport({
          present: 1,
          findings: [
            {
              id: 'github-copilot',
              present: true,
              skills: null,
              agents: null,
              mcp_servers: null,
              error: 'malformed config: not valid UTF-8',
            },
          ],
        })}
      />,
    )
    expect(screen.getByText('detected, but could not be read')).toBeInTheDocument()
    expect(screen.getByText(/not valid UTF-8/)).toBeInTheDocument()
  })

  it('does not offer a next step when nothing was detected', () => {
    render(
      <Intake report={intakeReport({ present: 0, total: 0, findings: [] })} />,
    )
    expect(screen.getByText(/No supported tool configs/)).toBeInTheDocument()
    expect(screen.queryByText(/^Next/)).not.toBeInTheDocument()
  })
})

describe('Examination', () => {
  it('reports a clean Brain without inventing findings', () => {
    render(<Examination report={examReport()} />)
    expect(screen.getByText('Clean bill of health.')).toBeInTheDocument()
    expect(screen.getByText('no drift, no faults')).toBeInTheDocument()
  })

  it('agrees in number when exactly one critical finding is open', () => {
    render(
      <Examination
        report={examReport({
          criticals: 1,
          findings: [
            {
              rule_id: 'canonical-source-missing',
              severity: 'critical',
              message: 'Source no longer exists.',
              subject: '/home/mara/AIBrain/skills/gone',
              auto_fixable: false,
            },
          ],
        })}
      />,
    )
    expect(screen.getByText('1 critical finding needs a human.')).toBeInTheDocument()
  })

  it('abbreviates a subject path against the Brain root', () => {
    render(
      <Examination
        report={examReport({
          findings: [
            {
              rule_id: 'mappings-unsorted',
              severity: 'info',
              message: 'Entries are unordered.',
              subject: '/home/mara/AIBrain/.brain/mappings.json',
              auto_fixable: true,
            },
          ],
          fixable: 1,
        })}
      />,
    )
    expect(screen.getByText('brain/.brain/mappings.json')).toBeInTheDocument()
  })

  it('points at --fix only when something is actually auto-fixable', () => {
    const fixable = examReport({
      fixable: 1,
      findings: [
        {
          rule_id: 'r',
          severity: 'warning',
          message: 'm',
          subject: null,
          auto_fixable: true,
        },
      ],
    })
    const { unmount } = render(<Examination report={fixable} />)
    expect(screen.getByText('neurosurgeon doctor --fix')).toBeInTheDocument()
    unmount()

    render(<Examination report={examReport()} />)
    expect(screen.queryByText('neurosurgeon doctor --fix')).not.toBeInTheDocument()
  })
})

describe('identity rules', () => {
  it('renders no emoji in either screen', () => {
    // The single loudest tell of a generated interface. The status vocabulary
    // is geometric shapes (U+25xx), all well below the emoji planes.
    const { container: a } = render(<Intake report={intakeReport()} />)
    const { container: b } = render(
      <Examination
        report={examReport({
          findings: [
            {
              rule_id: 'r',
              severity: 'warning',
              message: 'm',
              subject: null,
              auto_fixable: true,
            },
          ],
        })}
      />,
    )
    const emoji = /\p{Extended_Pictographic}/u
    expect(emoji.test(a.textContent ?? '')).toBe(false)
    expect(emoji.test(b.textContent ?? '')).toBe(false)
  })
})
