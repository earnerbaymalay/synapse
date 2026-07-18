import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Card, StatusPill, Toolbar, ToolbarButton } from '../components/ui'

const ENVIRONMENTS = ['Tauri + React (Default)', 'Plain CLI Only', 'Cross-Platform Only']

export interface DetectedTool {
  tool_id: string
  skills: number
  agents: number
  mcp_servers: number
  error: string | null
}

export interface DryRunReport {
  root: string
  detected: DetectedTool[]
}

type ScanState =
  | { status: 'idle' }
  | { status: 'scanning' }
  | { status: 'done'; report: DryRunReport }
  | { status: 'error'; message: string }

/** T5.2: wraps the real `cli import --dry-run`-equivalent flow — same
 * `detect()` + `import()` calls against `packages/core`, same
 * nothing-is-written guarantee. Throws in a plain-browser preview, where
 * there's no Tauri backend to call. */
export async function runDryRunScan(): Promise<DryRunReport> {
  if (!('__TAURI_INTERNALS__' in window)) {
    throw new Error('Tauri backend is not available in this preview.')
  }
  return invoke<DryRunReport>('scan_dry_run')
}

export function OnboardingWizard() {
  const [step, setStep] = useState(1)
  const [selected, setSelected] = useState(0)
  const [scan, setScan] = useState<ScanState>({ status: 'idle' })

  const startScan = async () => {
    setStep(2)
    setScan({ status: 'scanning' })
    try {
      const report = await runDryRunScan()
      setScan({ status: 'done', report })
    } catch (e) {
      setScan({ status: 'error', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <div>
      <Card>
        <h2 className="mb-4 text-base font-semibold text-white">
          🎯 Welcome to LLM Neurosurgeon
        </h2>

        {step === 1 && (
          <>
            <p className="mb-2 text-sm text-slate-400">Step 1/3: Select Dev Environment</p>
            <div className="rounded-md border border-slate-800 p-3">
              {ENVIRONMENTS.map((env, i) => (
                <label
                  key={env}
                  className="flex cursor-pointer items-center gap-2 py-1.5 text-sm text-slate-300"
                >
                  <input
                    type="radio"
                    name="env"
                    checked={selected === i}
                    onChange={() => setSelected(i)}
                    className="accent-primary-500"
                  />
                  {env}
                </label>
              ))}
            </div>
          </>
        )}

        {step === 2 && (
          <>
            <p className="mb-2 text-sm text-slate-400">
              Step 2/3: Scan for AI Tool Configs (Dry Run)
            </p>
            {scan.status === 'scanning' && (
              <p className="text-sm text-slate-400">Scanning current directory...</p>
            )}
            {scan.status === 'error' && (
              <p className="text-sm text-red-400" role="alert">
                {scan.message}
              </p>
            )}
            {scan.status === 'done' && (
              <div>
                <p className="mb-3 text-xs text-slate-500">
                  Dry run — nothing was written. Root:{' '}
                  <code className="font-mono">{scan.report.root}</code>
                </p>
                {scan.report.detected.length === 0 ? (
                  <p className="text-sm text-slate-400">
                    No supported AI tool configs detected.
                  </p>
                ) : (
                  <ul className="space-y-1.5">
                    {scan.report.detected.map((t) => (
                      <li
                        key={t.tool_id}
                        className="flex items-center justify-between rounded-md bg-slate-800/60 px-3 py-2 text-sm"
                      >
                        <span>{t.tool_id}</span>
                        {t.error ? (
                          <StatusPill tone="error">{t.error}</StatusPill>
                        ) : (
                          <span className="text-xs text-slate-400">
                            {t.skills} skill(s), {t.agents} agent(s), {t.mcp_servers} mcp
                            server(s)
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </>
        )}

        {step === 3 && (
          <>
            <p className="mb-2 text-sm text-slate-400">Step 3/3: Ready</p>
            <p className="text-sm text-slate-300">
              Setup complete. Nothing has been written to disk yet — writing detected
              configs into the Brain is a future release.
            </p>
          </>
        )}
      </Card>

      <div className="mt-4">
        <Toolbar>
          <ToolbarButton onClick={() => setStep((s) => Math.max(1, s - 1))}>
            ← Back
          </ToolbarButton>
          {step === 1 && (
            <ToolbarButton primary onClick={startScan}>
              Next
            </ToolbarButton>
          )}
          {step === 2 && (
            <ToolbarButton primary onClick={() => setStep(3)}>
              Next
            </ToolbarButton>
          )}
          {step === 3 && <ToolbarButton primary>Finish</ToolbarButton>}
          {step === 1 && <ToolbarButton>Skip</ToolbarButton>}
        </Toolbar>
      </div>
    </div>
  )
}
