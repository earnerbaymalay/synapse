import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { cn } from './lib/utils'
import { Hint, HINTS_TOGGLE_KEYS, HintsProvider, useHints } from './lib/hints'
import { formatKeys, isMac, useShortcut } from './lib/shortcuts'
import { Intake } from './screens/Intake'
import { Examination } from './screens/Examination'
import { MemoryGraph } from './screens/MemoryGraph'
import { Cockpit } from './screens/Cockpit'
import { Spool } from './screens/Spool'
import { AlertBanner } from './components/AlertBanner'
import { TitleBar } from './components/TitleBar'
import { CommandReference } from './components/CommandReference'
import { HintsCoach } from './components/HintsCoach'
import type { ExamReport, IntakeReport, ScreenId } from './screens/types'

/**
 * The desktop shell.
 *
 * A frameless window: TitleBar across the top, then the sidebar and the
 * working area. Five screens, each backed by a real core call:
 * - Intake: what is installed
 * - Examination: what is wrong
 * - Cockpit: live vital signs
 * - Memory: the memory node graph
 * - Sensory Spool: token reduction diff
 *
 * Data reads are deliberately narrow: `examine` runs once on boot and again
 * only when asked (re-read vitals, after a heal, retry). `intake` runs on boot
 * and each time the Intake screen is entered. Switching screens never re-runs
 * the examination.
 */

interface NavItem {
  id: ScreenId
  label: string
  blurb: string
  keys: string
}

const NAV: NavItem[] = [
  { id: 'intake', label: 'Intake', blurb: 'what is installed', keys: 'mod+1' },
  { id: 'examination', label: 'Examination', blurb: 'what is wrong', keys: 'mod+2' },
  { id: 'cockpit', label: 'Cockpit', blurb: 'live vital signs', keys: 'mod+3' },
  { id: 'memory', label: 'Memory', blurb: 'memory node graph', keys: 'mod+4' },
  { id: 'spool', label: 'Sensory Spool', blurb: 'token reduction', keys: 'mod+5' },
]

const KEYS_REREAD = 'mod+r'
// Shared with HintsCoach so the coach text always names the key that is wired.
const KEYS_HINTS = HINTS_TOGGLE_KEYS

/** True when running inside a Tauri webview — false in a plain browser. */
function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/** ARIA form of a shortcut ("mod+1" -> "Control+1", or "Meta+1" on macOS). */
function ariaKeys(keys: string): string {
  return keys.replace(/\bmod\b/g, isMac() ? 'Meta' : 'Control')
}

function keyLabel(keys: string): string {
  return formatKeys(keys).join(' ')
}

/** The backend reports versions with or without a leading "v"; show one. */
function formatVersion(v: string): string {
  const trimmed = v.trim()
  return trimmed.startsWith('v') ? trimmed : `v${trimmed}`
}

function Shell() {
  const tauri = inTauri()
  const hints = useHints()

  const [screen, setScreen] = useState<ScreenId>('intake')
  const [version, setVersion] = useState<string | null>(null)
  const [versionRead, setVersionRead] = useState(false)

  const [exam, setExam] = useState<ExamReport | null>(null)
  const [examError, setExamError] = useState<string | null>(null)
  const [examBusy, setExamBusy] = useState(false)

  const [intake, setIntake] = useState<IntakeReport | null>(null)
  const [intakeError, setIntakeError] = useState<string | null>(null)
  const [intakeBusy, setIntakeBusy] = useState(false)

  // Monotonic request ids: a slow, older response must never overwrite a
  // newer one, and nothing is set after unmount.
  const examSeq = useRef(0)
  const intakeSeq = useRef(0)
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])

  const readExam = useCallback(async () => {
    if (!inTauri()) return
    const seq = ++examSeq.current
    setExamBusy(true)
    try {
      const report = await invoke<ExamReport>('examine', { brain: null, tools: null })
      if (!mounted.current || seq !== examSeq.current) return
      setExam(report)
      setExamError(null)
    } catch (e) {
      if (!mounted.current || seq !== examSeq.current) return
      setExamError(String(e))
    } finally {
      if (mounted.current && seq === examSeq.current) setExamBusy(false)
    }
  }, [])

  const readIntake = useCallback(async () => {
    if (!inTauri()) return
    const seq = ++intakeSeq.current
    setIntakeBusy(true)
    try {
      const report = await invoke<IntakeReport>('intake', { site: null })
      if (!mounted.current || seq !== intakeSeq.current) return
      setIntake(report)
      setIntakeError(null)
    } catch (e) {
      if (!mounted.current || seq !== intakeSeq.current) return
      setIntakeError(String(e))
    } finally {
      if (mounted.current && seq === intakeSeq.current) setIntakeBusy(false)
    }
  }, [])

  // Examination: once on boot. Never tied to the current screen.
  useEffect(() => {
    void readExam()
  }, [readExam])

  // Intake: each time the Intake screen is entered (including boot).
  useEffect(() => {
    if (screen === 'intake') void readIntake()
  }, [screen, readIntake])

  useEffect(() => {
    if (!inTauri()) return
    invoke<string>('get_version')
      .then((v) => {
        if (mounted.current) setVersion(v)
      })
      .catch(() => {
        if (mounted.current) setVersion(null)
      })
      .finally(() => {
        if (mounted.current) setVersionRead(true)
      })
  }, [])

  // The current screen, readable from stable callbacks.
  const screenRef = useRef(screen)
  screenRef.current = screen

  /** re-read vitals: the examination, plus intake when it is on screen. */
  const reread = useCallback(() => {
    void readExam()
    if (screenRef.current === 'intake') void readIntake()
  }, [readExam, readIntake])

  const busy = examBusy || intakeBusy

  // A retry or re-read that succeeds removes the control that held keyboard
  // focus (the retry button goes away with its error). Without this the focus
  // falls to <body> and the next Tab starts from the top of the window. Only
  // on an error clearing, so boot never moves focus.
  const mainRef = useRef<HTMLElement>(null)
  const hadError = useRef(false)
  const anyError = Boolean(examError || intakeError)
  useEffect(() => {
    const cleared = hadError.current && !anyError
    hadError.current = anyError
    if (!cleared) return
    const active = document.activeElement
    if (!active || active === document.body || !active.isConnected) {
      mainRef.current?.focus({ preventScroll: true })
    }
  }, [anyError])

  // Shortcuts. The registry calls preventDefault, so mod+r never reloads.
  useShortcut({ id: 'nav.intake', keys: NAV[0].keys, label: 'Go to Intake', group: 'Navigation' }, () =>
    setScreen('intake'),
  )
  useShortcut({ id: 'nav.examination', keys: NAV[1].keys, label: 'Go to Examination', group: 'Navigation' }, () =>
    setScreen('examination'),
  )
  useShortcut({ id: 'nav.cockpit', keys: NAV[2].keys, label: 'Go to Cockpit', group: 'Navigation' }, () =>
    setScreen('cockpit'),
  )
  useShortcut({ id: 'nav.memory', keys: NAV[3].keys, label: 'Go to Memory', group: 'Navigation' }, () =>
    setScreen('memory'),
  )
  useShortcut({ id: 'nav.spool', keys: NAV[4].keys, label: 'Go to Sensory Spool', group: 'Navigation' }, () =>
    setScreen('spool'),
  )
  useShortcut({ id: 'shell.reread', keys: KEYS_REREAD, label: 'Re-read vitals', group: 'Actions' }, () => {
    if (inTauri()) reread()
  })
  useShortcut({ id: 'shell.hints', keys: KEYS_HINTS, label: 'Turn hints on or off', group: 'View' }, () =>
    hints.toggle(),
  )

  const versionText = version ? formatVersion(version) : !tauri || versionRead ? 'version unknown' : 'reading version…'

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-ground text-ink">
      <TitleBar />

      <div className="flex min-h-0 flex-1">
        {/* Sidebar */}
        <aside className="flex w-60 shrink-0 flex-col overflow-y-auto border-r border-rule">
          <div className="border-b border-rule px-4 py-4">
            <p className="font-display text-sm uppercase tracking-[0.04em] text-drape">Synapse</p>
            <p className="font-mono text-[10px] uppercase tracking-[0.08em] text-ink-soft">
              Clinical Engine for AI Context
            </p>
            <p className="mt-1 font-mono text-xs text-ink-soft" data-testid="version">
              {versionText}
            </p>
          </div>

          <nav aria-label="Screens" className="flex-1 py-2">
            <ul>
              {NAV.map((item) => {
                const active = screen === item.id
                return (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => setScreen(item.id)}
                      aria-current={active ? 'page' : undefined}
                      aria-keyshortcuts={ariaKeys(item.keys)}
                      className={cn(
                        'flex w-full items-baseline gap-2 border-l-2 px-4 py-2.5 text-left transition-colors',
                        'focus-visible:outline focus-visible:outline-1 focus-visible:-outline-offset-1 focus-visible:outline-drape',
                        active ? 'border-drape bg-chart text-ink' : 'border-transparent text-ink-soft hover:text-ink',
                      )}
                    >
                      <span className="min-w-0 flex-1">
                        <span className="block font-mono text-sm">{item.label}</span>
                        <span className="block text-xs text-ink-soft">{item.blurb}</span>
                      </span>
                      <kbd aria-hidden="true" className="shrink-0 font-mono text-[10px] text-ink-soft">{keyLabel(item.keys)}</kbd>
                    </button>
                  </li>
                )
              })}
            </ul>
            <div className="px-4">
              <Hint id="shell.nav">{formatKeys('mod')[0]} 1-5 switches screens.</Hint>
            </div>
          </nav>

          <div className="border-t border-rule px-4 py-3">
            <div className="flex items-baseline justify-between gap-2">
              {/* While a read runs the button is aria-disabled, not disabled:
                  a disabled button drops keyboard focus mid-press. */}
              <button
                type="button"
                onClick={() => {
                  if (!busy) reread()
                }}
                disabled={!tauri}
                aria-disabled={busy || undefined}
                aria-keyshortcuts={ariaKeys(KEYS_REREAD)}
                title={
                  !tauri
                    ? 'No backend attached: this window is running outside the desktop shell, so there are no vitals to read.'
                    : busy
                      ? 'A read is already in progress.'
                      : 'Re-run the examination'
                }
                className="font-mono text-xs text-drape hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape disabled:text-ink-soft aria-disabled:cursor-default aria-disabled:text-ink-soft aria-disabled:hover:text-ink-soft"
              >
                {busy ? 'reading…' : 're-read vitals'}
              </button>
              <kbd className="font-mono text-[10px] text-ink-soft">{keyLabel(KEYS_REREAD)}</kbd>
            </div>
            <Hint id="shell.reread">{keyLabel(KEYS_REREAD)} re-reads vitals without reloading the window.</Hint>

            <div className="mt-2 flex items-baseline justify-between gap-2">
              <button
                type="button"
                onClick={hints.toggle}
                aria-pressed={hints.enabled}
                aria-keyshortcuts={ariaKeys(KEYS_HINTS)}
                className="font-mono text-xs text-ink-soft hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape"
              >
                {hints.enabled ? 'hints on' : 'hints off'}
              </button>
              <kbd className="font-mono text-[10px] text-ink-soft">{keyLabel(KEYS_HINTS)}</kbd>
            </div>
            <Hint id="shell.reference">Press ? for every shortcut.</Hint>
          </div>
          <HintsCoach />
        </aside>

        {/* Working area */}
        <main
          ref={mainRef}
          tabIndex={-1}
          className="min-h-0 min-w-0 flex-1 overflow-y-auto bg-chart px-8 py-6 focus-visible:outline-none"
        >
          <div className="mx-auto max-w-5xl">
            {!tauri ? (
              <Disconnected />
            ) : (
              <>
                {/* Only shown once vitals have actually been read, and hidden
                    while the latest read failed: a stale "Healthy" next to
                    "vitals unread" would contradict itself. */}
                {exam && !examError && (
                  <AlertBanner exam={exam} onHealed={() => void readExam()} onOpenOnboard={() => setScreen('intake')} />
                )}
                {examError && screen !== 'examination' && (
                  <VitalsUnread error={examError} busy={examBusy} onRetry={() => void readExam()} />
                )}
                {screen === 'intake' ? (
                  intakeError ? (
                    <Halted
                      screen="intake"
                      error={intakeError}
                      next="synapse doctor"
                      busy={intakeBusy}
                      onRetry={() => void readIntake()}
                    />
                  ) : intake ? (
                    <Intake report={intake} onRescan={() => void readIntake()} busy={intakeBusy} />
                  ) : (
                    <p className="font-mono text-xs text-ink-soft">reading intake…</p>
                  )
                ) : screen === 'examination' ? (
                  examError ? (
                    <Halted screen="examination" error={examError} busy={examBusy} onRetry={() => void readExam()} />
                  ) : exam ? (
                    <Examination report={exam} />
                  ) : (
                    <p className="font-mono text-xs text-ink-soft">examining vitals…</p>
                  )
                ) : screen === 'cockpit' ? (
                  <Cockpit />
                ) : screen === 'memory' ? (
                  <MemoryGraph />
                ) : (
                  <Spool />
                )}
              </>
            )}
          </div>
        </main>
      </div>

      <CommandReference />
    </div>
  )
}

function Disconnected() {
  return (
    <div>
      <p className="font-mono text-sm">No backend attached.</p>
      <p className="mt-2 max-w-prose text-sm text-ink-soft">
        This window is running outside the desktop shell, so there is nothing to read from disk. Nothing is shown in
        its place.
      </p>
      <p className="mt-3 font-mono text-xs">
        <span className="text-ink-soft">Next </span>
        <span className="text-drape">synapse scan</span>
      </p>
    </div>
  )
}

function RetryButton({ busy, onRetry }: { busy: boolean; onRetry: () => void }) {
  return (
    <button
      type="button"
      onClick={() => {
        if (!busy) onRetry()
      }}
      // aria-disabled, not disabled: a disabled button drops keyboard focus.
      aria-disabled={busy || undefined}
      className="font-mono text-xs text-drape hover:text-ink focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-drape aria-disabled:cursor-default aria-disabled:text-ink-soft aria-disabled:hover:text-ink-soft"
    >
      {busy ? 'retrying…' : 'retry'}
    </button>
  )
}

function Halted({
  screen,
  error,
  next,
  busy,
  onRetry,
}: {
  screen: ScreenId
  error: string
  /** The CLI command to run next, named per IDENTITY rule 1. */
  next?: string
  busy: boolean
  onRetry: () => void
}) {
  return (
    <div role="alert">
      <p className="font-mono text-sm text-alarm">{screen} halted.</p>
      <p className="mt-2 max-w-prose break-words font-mono text-xs text-ink-soft">{error}</p>
      <p className="mt-3 font-mono text-xs">
        <span className="text-ink-soft">Next </span>
        {next ? (
          <>
            <span className="text-drape">{next}</span>
            <span className="text-ink-soft"> or </span>
          </>
        ) : null}
        <RetryButton busy={busy} onRetry={onRetry} />
      </p>
    </div>
  )
}

/** The examination failed but this screen has its own data: say so in one ruled line. */
function VitalsUnread({ error, busy, onRetry }: { error: string; busy: boolean; onRetry: () => void }) {
  return (
    <div role="alert" className="mb-4 flex items-baseline gap-3 border-b border-rule pb-2 font-mono text-xs">
      <span className="shrink-0 text-alarm">vitals unread</span>
      <span className="min-w-0 flex-1 truncate text-ink-soft" title={error}>
        {error}
      </span>
      <span className="shrink-0">
        <span className="text-ink-soft">Next </span>
        <RetryButton busy={busy} onRetry={onRetry} />
      </span>
    </div>
  )
}

/** The app, with its hints context. Rendered once from main.tsx. */
function App() {
  return (
    <HintsProvider>
      <Shell />
    </HintsProvider>
  )
}

export default App
