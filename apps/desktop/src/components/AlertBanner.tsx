import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { ExamReport } from '../screens/types'

interface AlertBannerProps {
  exam: ExamReport | null
  onHealed: () => void
  onOpenOnboard?: () => void
}

/**
 * Clinical Alert & Action Center.
 * Always visible at the top of the operating console:
 * - Surfaces critical faults (Zone 1 harness tamper, config drift, broken symlinks)
 * - Prompts for user action when needed
 * - Provides one-click clinical remediation (`heal_vitals`)
 */
export function AlertBanner({ exam, onHealed, onOpenOnboard }: AlertBannerProps) {
  const [healing, setHealing] = useState(false)
  const [healResult, setHealResult] = useState<string | null>(null)

  const criticals = exam?.criticals ?? 0
  const fixable = exam?.fixable ?? 0
  const hasTamper = exam?.findings.some((f) => f.rule_id === 'master-prompt-tampered')
  const hasMissingPrompt = exam?.findings.some((f) => f.rule_id === 'master-prompt-missing')

  const handleHeal = async () => {
    setHealing(true)
    setHealResult(null)
    try {
      const fixedCount = await invoke<number>('heal_vitals', { brain: null, tools: null })
      setHealResult(`Successfully healed ${fixedCount} fault(s)`)
      onHealed()
    } catch (e) {
      setHealResult(`Healing error: ${e}`)
    } finally {
      setHealing(false)
    }
  }

  // Clean state
  if (criticals === 0 && fixable === 0 && !hasTamper) {
    return (
      <div className="mb-4 flex items-center justify-between border border-semantic-success/30 bg-semantic-success/5 px-4 py-2 font-mono text-xs text-semantic-success">
        <div className="flex items-center gap-2">
          <span className="inline-block h-2 w-2 rounded-full bg-semantic-success animate-pulse" />
          <span>● LOCKSTEP SYNCED — All 15 grafts healthy (0 faults)</span>
        </div>
        <div className="flex items-center gap-3 text-ink-soft">
          <span>Doctor: Clean</span>
          <span>Core Harness: Signed</span>
        </div>
      </div>
    )
  }

  // Critical fault state (e.g. Master prompt Zone 1 tampered or drift)
  return (
    <div className="mb-4 border border-semantic-error/40 bg-semantic-error/10 p-3 font-mono text-xs text-semantic-error">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="text-sm font-bold">▲</span>
          <div>
            <span className="font-semibold uppercase tracking-wider">
              {hasTamper
                ? 'CRITICAL FAULT: Master Prompt Zone 1 Clinical Core Tampered'
                : `${criticals} Critical Finding(s) Detected`}
            </span>
            <p className="text-[11px] text-ink-soft mt-0.5">
              {hasTamper
                ? 'Non-negotiable clinical operating core checksum invalid. Doctor remediation required.'
                : `${fixable} auto-fixable issue(s) in canonical brain mappings or tool projections.`}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {hasMissingPrompt && onOpenOnboard && (
            <button
              onClick={onOpenOnboard}
              className="border border-drape/60 bg-drape/20 px-3 py-1 font-mono text-xs text-drape transition hover:bg-drape/30"
            >
              Configure Master Prompt
            </button>
          )}

          <button
            onClick={handleHeal}
            disabled={healing}
            className="border border-semantic-error bg-semantic-error/20 px-3 py-1 font-mono text-xs font-semibold text-ink uppercase tracking-wider transition hover:bg-semantic-error/30 disabled:opacity-50"
          >
            {healing ? 'Healing Vitals…' : 'Heal Vitals (Doctor --fix)'}
          </button>
        </div>
      </div>

      {healResult && (
        <div className="mt-2 border-t border-semantic-error/20 pt-2 text-[11px] text-ink">
          {healResult}
        </div>
      )}
    </div>
  )
}
