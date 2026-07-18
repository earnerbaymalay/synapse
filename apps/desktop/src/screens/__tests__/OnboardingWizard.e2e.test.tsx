import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { invoke } from '@tauri-apps/api/core'
import { OnboardingWizard } from '../OnboardingWizard'
import type { DryRunReport } from '../OnboardingWizard'

// T5.2's "e2e onboarding test": drives the wizard's full 3-step flow the
// way a user would (click through, don't call internals directly),
// mocking only the one seam that genuinely can't run outside a real
// Tauri webview — the `invoke()` bridge to the Rust `scan_dry_run`
// command. Everything else (the wizard's own state machine, the
// step transitions, the rendered report) is exercised for real.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

afterEach(() => {
  cleanup()
  vi.mocked(invoke).mockReset()
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

function stubTauriBridge() {
  ;(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {}
}

const SAMPLE_REPORT: DryRunReport = {
  root: '/home/user/my-project',
  detected: [
    { tool_id: 'claude-code', skills: 7, agents: 6, mcp_servers: 0, error: null },
    { tool_id: 'cursor', skills: 2, agents: 0, mcp_servers: 0, error: null },
  ],
}

describe('OnboardingWizard — dry-run flow (T5.2)', () => {
  it('walks all 3 steps and shows the real scan_dry_run report', async () => {
    stubTauriBridge()
    vi.mocked(invoke).mockResolvedValue(SAMPLE_REPORT)
    const user = userEvent.setup()

    render(<OnboardingWizard />)

    // Step 1: environment selection.
    expect(screen.getByText(/Step 1\/3: Select Dev Environment/)).toBeInTheDocument()
    expect(screen.getByLabelText('Tauri + React (Default)')).toBeChecked()

    await user.click(screen.getByRole('button', { name: 'Next' }))

    // Step 2: fires the real dry-run invoke() call and renders its result.
    expect(invoke).toHaveBeenCalledWith('scan_dry_run')
    await waitFor(() =>
      expect(screen.getByText(/Step 2\/3: Scan for AI Tool Configs/)).toBeInTheDocument(),
    )
    await waitFor(() => expect(screen.getByText('claude-code')).toBeInTheDocument())
    expect(screen.getByText(/7 skill\(s\), 6 agent\(s\), 0 mcp server\(s\)/)).toBeInTheDocument()
    expect(screen.getByText('cursor')).toBeInTheDocument()
    expect(screen.getByText(/my-project/)).toBeInTheDocument()
    expect(screen.getByText(/nothing was written/i)).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Next' }))

    // Step 3: completion, explicit about not having written anything yet.
    expect(screen.getByText(/Step 3\/3: Ready/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Finish' })).toBeInTheDocument()
  })

  it('shows an empty state when no tools are detected', async () => {
    stubTauriBridge()
    vi.mocked(invoke).mockResolvedValue({ root: '/empty/dir', detected: [] } satisfies DryRunReport)
    const user = userEvent.setup()

    render(<OnboardingWizard />)
    await user.click(screen.getByRole('button', { name: 'Next' }))

    await waitFor(() =>
      expect(screen.getByText(/No supported AI tool configs detected/)).toBeInTheDocument(),
    )
  })

  it('degrades gracefully instead of crashing when no Tauri backend is present', async () => {
    // Deliberately do NOT stub __TAURI_INTERNALS__ — this is exactly the
    // plain-browser-preview situation hit while screenshotting T5.1.
    const user = userEvent.setup()

    render(<OnboardingWizard />)
    await user.click(screen.getByRole('button', { name: 'Next' }))

    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Tauri backend is not available in this preview.',
      ),
    )
    expect(invoke).not.toHaveBeenCalled()
  })

  it('lets the user go back from the scan step without losing the wizard', async () => {
    stubTauriBridge()
    vi.mocked(invoke).mockResolvedValue(SAMPLE_REPORT)
    const user = userEvent.setup()

    render(<OnboardingWizard />)
    await user.click(screen.getByRole('button', { name: 'Next' }))
    await waitFor(() => expect(screen.getByText('claude-code')).toBeInTheDocument())

    await user.click(screen.getByRole('button', { name: '← Back' }))

    expect(screen.getByText(/Step 1\/3: Select Dev Environment/)).toBeInTheDocument()
  })
})
