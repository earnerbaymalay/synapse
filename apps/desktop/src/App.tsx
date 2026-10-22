import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { cn } from './lib/utils'
import { Intake } from './screens/Intake'
import { Examination } from './screens/Examination'
import { MemoryGraph } from './screens/MemoryGraph'
import { Cockpit } from './screens/Cockpit'
import { Spool } from './screens/Spool'
import { AlertBanner } from './components/AlertBanner'
import type { ExamReport, IntakeReport, ScreenId } from './screens/types'

/**
 * The desktop shell.
 *
 * 5 measured clinical screens:
 * - Intake: what is installed & onboarding wizard
 * - Examination: what is wrong & doctor diagnostics
 * - Cockpit: live vital signs telemetry & EKG waveform
 * - Memory: 3D cybernetic synaptic node network
 * - Spool: sensory gating token reduction diff inspector
 */

const NAV: { id: ScreenId; label: string; blurb: string }[] = [
  { id: 'intake', label: 'Intake', blurb: 'what is installed' },
  { id: 'examination', label: 'Examination', blurb: 'what is wrong' },
  { id: 'cockpit', label: 'Cockpit', blurb: 'live vital signs HUD' },
  { id: 'memory', label: 'Memory', blurb: '3D cyber node graph' },
  { id: 'spool', label: 'Sensory Spool', blurb: 'token compression' },
]

/** True when running inside a Tauri webview — false in a plain browser. */
function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function App() {
  const [screen, setScreen] = useState<ScreenId>('intake')
  const [version, setVersion] = useState<string | null>(null)
  const [intake, setIntake] = useState<IntakeReport | null>(null)
  const [exam, setExam] = useState<ExamReport | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const examine = useCallback(async () => {
    if (!inTauri()) return
    setBusy(true)
    setError(null)
    try {
      // Always keep examination vitals fresh for the AlertBanner
      const examReport = await invoke<ExamReport>('examine', { brain: null, tools: null })
      setExam(examReport)

      if (screen === 'intake') {
        setIntake(await invoke<IntakeReport>('intake', { site: null }))
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }, [screen])

  useEffect(() => {
    void examine()
  }, [examine])

  useEffect(() => {
    if (!inTauri()) return
    invoke<string>('get_version')
      .then(setVersion)
      .catch(() => setVersion(null))
  }, [])

  return (
    <div className="flex h-screen bg-ground text-ink">
      {/* Sidebar Navigation */}
      <aside className="flex w-56 shrink-0 flex-col border-r border-rule">
        <div className="border-b border-rule px-4 py-4">
          <p className="font-display text-sm uppercase tracking-[0.04em] text-drape">
            Synapse
          </p>
          <p className="font-mono text-[10px] uppercase tracking-[0.08em] text-ink-soft">
            Clinical Engine for AI Context
          </p>
          <p className="mt-1 font-mono text-xs text-ink-soft">
            {version ? `v${version}` : 'v1.0.0'}
          </p>
        </div>

        <nav className="flex-1 py-2">
          {NAV.map((item) => (
            <button
              key={item.id}
              onClick={() => setScreen(item.id)}
              aria-current={screen === item.id ? 'page' : undefined}
              className={cn(
                'block w-full border-l-2 px-4 py-2.5 text-left transition-colors',
                screen === item.id
                  ? 'border-drape bg-chart text-ink'
                  : 'border-transparent text-ink-soft hover:text-ink',
              )}
            >
              <span className="block font-mono text-sm">{item.label}</span>
              <span className="block text-xs text-ink-soft">{item.blurb}</span>
            </button>
          ))}
        </nav>

        <div className="border-t border-rule px-4 py-3">
          <button
            onClick={() => void examine()}
            disabled={busy || !inTauri()}
            className="font-mono text-xs text-drape hover:text-ink disabled:text-ink-soft"
          >
            {busy ? 'reading…' : 're-read vitals'}
          </button>
        </div>
      </aside>

      {/* Main Operating Workspace */}
      <main className="flex-1 overflow-y-auto bg-chart px-8 py-6">
        <div className="mx-auto max-w-5xl">
          {/* Global Clinical Alert & Action Center */}
          <AlertBanner exam={exam} onHealed={() => void examine()} onOpenOnboard={() => setScreen('intake')} />

          {!inTauri() ? (
            <Disconnected />
          ) : error ? (
            <Halted screen={screen} error={error} />
          ) : screen === 'intake' ? (
            intake ? <Intake report={intake} /> : <p className="font-mono text-xs text-ink-soft">reading intake…</p>
          ) : screen === 'examination' ? (
            exam ? <Examination report={exam} /> : <p className="font-mono text-xs text-ink-soft">examining vitals…</p>
          ) : screen === 'cockpit' ? (
            <Cockpit />
          ) : screen === 'memory' ? (
            <MemoryGraph />
          ) : (
            <Spool />
          )}
        </div>
      </main>
    </div>
  )
}

function Disconnected() {
  return (
    <div>
      <p className="font-mono text-sm">No backend attached.</p>
      <p className="mt-2 max-w-prose text-sm text-ink-soft">
        This window is running outside the desktop shell, so there is nothing to
        read from disk. Nothing is shown in its place.
      </p>
      <p className="mt-3 font-mono text-xs">
        <span className="text-ink-soft">Next </span>
        <span className="text-drape">synapse scan</span>
      </p>
    </div>
  )
}

function Halted({ screen, error }: { screen: ScreenId; error: string }) {
  return (
    <div>
      <p className="font-mono text-sm text-alarm">
        {screen} halted.
      </p>
      <p className="mt-2 max-w-prose break-words font-mono text-xs text-ink-soft">
        {error}
      </p>
    </div>
  )
}

export default App
