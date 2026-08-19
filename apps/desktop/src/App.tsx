import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { cn } from './lib/utils'
import { Intake } from './screens/Intake'
import { Examination } from './screens/Examination'
import type { ExamReport, IntakeReport, ScreenId } from './screens/types'

/**
 * The desktop shell.
 *
 * Two screens, both backed by real `neurosurgeon-core` calls. The eight
 * screens this replaced were transcriptions of an ASCII wireframe rendering
 * hardcoded sample data ("Project A (Active)"), which is what made the app
 * read as generated — see IDENTITY.md.
 *
 * Navigation is text in a ruled column, not icons: the identity's marks are
 * reserved for status, so spending them on chrome would dilute the one signal
 * that carries meaning.
 */

const NAV: { id: ScreenId; label: string; blurb: string }[] = [
  { id: 'intake', label: 'Intake', blurb: 'what is installed' },
  { id: 'examination', label: 'Examination', blurb: 'what is wrong' },
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
      if (screen === 'intake') {
        setIntake(await invoke<IntakeReport>('intake', { site: null }))
      } else {
        setExam(await invoke<ExamReport>('examine', { brain: null, tools: null }))
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

  const report = screen === 'intake' ? intake : exam

  return (
    <div className="flex h-screen bg-ground text-ink">
      <aside className="flex w-52 shrink-0 flex-col border-r border-rule">
        <div className="border-b border-rule px-4 py-4">
          <p className="font-mono text-xs font-semibold uppercase tracking-[0.14em]">
            Neurosurgeon
          </p>
          <p className="mt-0.5 font-mono text-xs text-ink-soft">
            {/* Never invent a version: an unread build reports as unknown. */}
            {version ? `v${version}` : 'version unread'}
          </p>
        </div>

        <nav className="flex-1 py-2">
          {NAV.map((item) => (
            <button
              key={item.id}
              onClick={() => setScreen(item.id)}
              aria-current={screen === item.id ? 'page' : undefined}
              className={cn(
                'block w-full border-l-2 px-4 py-2 text-left transition-colors',
                screen === item.id
                  ? 'border-drape text-ink'
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
            className="font-mono text-xs text-drape disabled:text-ink-soft"
          >
            {busy ? 'reading…' : 're-read'}
          </button>
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto bg-chart px-8 py-6">
        <div className="mx-auto max-w-4xl">
        {!inTauri() ? (
          <Disconnected />
        ) : error ? (
          <Halted screen={screen} error={error} />
        ) : !report ? (
          <p className="font-mono text-xs text-ink-soft">reading…</p>
        ) : screen === 'intake' ? (
          <Intake report={intake!} />
        ) : (
          <Examination report={exam!} />
        )}
        </div>
      </main>
    </div>
  )
}

/**
 * Shown when the page is open outside a Tauri webview (a plain browser, or a
 * screenshot run). It says so plainly rather than rendering sample data —
 * that substitution is exactly what the old screens did.
 */
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
        <span className="text-drape">neurosurgeon scan</span>
      </p>
    </div>
  )
}

function Halted({ screen, error }: { screen: ScreenId; error: string }) {
  return (
    <div>
      <p className="font-mono text-sm text-alarm">
        {screen === 'intake' ? 'Intake' : 'Examination'} halted.
      </p>
      <p className="mt-2 max-w-prose break-words font-mono text-xs text-ink-soft">
        {error}
      </p>
    </div>
  )
}

export default App
