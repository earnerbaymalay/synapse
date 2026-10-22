import { useEffect, useRef, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { CockpitTelemetry } from './types'

export function Cockpit() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const [telemetry, setTelemetry] = useState<CockpitTelemetry | null>(null)
  const [loading, setLoading] = useState(true)

  const loadTelemetry = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const res = await invoke<CockpitTelemetry>('get_cockpit_telemetry', {
          brain: null,
          tools: null,
        })
        setTelemetry(res)
      } else {
        // Fallback for browser preview
        setTelemetry({
          gateway_status: 'ONLINE',
          upstream_url: 'http://127.0.0.1:11434 (Ollama)',
          model: 'qwen2.5-coder:7b',
          latency_ms: 12,
          requests_handled: 18,
          raw_prompt_tokens: 184600,
          gated_prompt_tokens: 11200,
          tokens_saved: 173400,
          reduction_pct: 93.9,
          brain_path: '~/Desktop/AI BRAIN',
          adapters_present: 15,
          adapters_total: 15,
          doctor_criticals: 0,
          doctor_fixable: 0,
        })
      }
    } catch {
      // Graceful fallback
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadTelemetry()
  }, [loadTelemetry])

  // Canvas EKG Waveform Animation
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let animId: number
    let xOffset = 0

    const renderEkg = () => {
      const width = canvas.width
      const height = canvas.height
      const midY = height / 2

      ctx.fillStyle = '#06080b'
      ctx.fillRect(0, 0, width, height)

      // Grid background
      ctx.strokeStyle = 'rgba(63, 185, 80, 0.08)'
      ctx.lineWidth = 1
      for (let x = 0; x < width; x += 20) {
        ctx.beginPath()
        ctx.moveTo(x, 0)
        ctx.lineTo(x, height)
        ctx.stroke()
      }
      for (let y = 0; y < height; y += 20) {
        ctx.beginPath()
        ctx.moveTo(0, y)
        ctx.lineTo(width, y)
      }

      // EKG Signal Trace
      ctx.strokeStyle = '#3fb950'
      ctx.lineWidth = 2
      ctx.shadowColor = '#3fb950'
      ctx.shadowBlur = 8
      ctx.beginPath()

      xOffset = (xOffset + 2) % 200

      for (let x = 0; x < width; x++) {
        const cycleX = (x + xOffset) % 200
        let y = midY

        if (cycleX > 60 && cycleX < 75) {
          y = midY - Math.sin(((cycleX - 60) / 15) * Math.PI) * 12 // P wave
        } else if (cycleX >= 75 && cycleX < 85) {
          y = midY + 8 // Q dip
        } else if (cycleX >= 85 && cycleX < 95) {
          y = midY - 45 // R peak
        } else if (cycleX >= 95 && cycleX < 105) {
          y = midY + 18 // S dip
        } else if (cycleX >= 115 && cycleX < 145) {
          y = midY - Math.sin(((cycleX - 115) / 30) * Math.PI) * 16 // T wave
        }

        if (x === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }

      ctx.stroke()
      ctx.shadowBlur = 0

      animId = requestAnimationFrame(renderEkg)
    }

    animId = requestAnimationFrame(renderEkg)
    return () => cancelAnimationFrame(animId)
  }, [])

  if (loading || !telemetry) {
    return (
      <div className="flex h-64 items-center justify-center font-mono text-xs text-ink-soft">
        Polling vital signs…
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Screen Header */}
      <div className="border-b border-rule pb-3">
        <h2 className="font-display text-base tracking-wider uppercase text-drape">
          Vital Signs Cockpit // Runtime Telemetry
        </h2>
        <p className="font-mono text-xs text-ink-soft">
          Continuous telemetry for local model gateway, sensory gating compression, and Brain lockstep.
        </p>
      </div>

      {/* EKG Live Waveform Monitor */}
      <div className="border border-rule bg-ground p-4">
        <div className="mb-2 flex items-center justify-between font-mono text-xs">
          <span className="flex items-center gap-2 text-semantic-success">
            <span className="inline-block h-2 w-2 rounded-full bg-semantic-success animate-ping" />
            Vitals Monitor: Steady Rhythm (60 bpm)
          </span>
          <span className="text-ink-soft">Latency: {telemetry.latency_ms}ms · 127.0.0.1:8088</span>
        </div>
        <canvas ref={canvasRef} width={800} height={120} className="w-full h-28 border border-rule/60 bg-chart" />
      </div>

      {/* Primary Metrics Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Runtime Gateway Card */}
        <div className="border border-rule bg-chart p-4">
          <div className="flex items-center justify-between border-b border-rule pb-2">
            <span className="font-mono text-xs font-semibold uppercase text-drape">Runtime Gateway</span>
            <span className="font-mono text-[11px] text-semantic-success">● {telemetry.gateway_status}</span>
          </div>
          <div className="mt-3 space-y-2 font-mono text-xs text-ink-soft">
            <div>Upstream: <span className="text-ink">{telemetry.upstream_url}</span></div>
            <div>Model: <span className="text-ink">{telemetry.model}</span></div>
            <div>Requests Handled: <span className="text-drape">{telemetry.requests_handled}</span></div>
          </div>
        </div>

        {/* Sensory Gating Card */}
        <div className="border border-rule bg-chart p-4">
          <div className="flex items-center justify-between border-b border-rule pb-2">
            <span className="font-mono text-xs font-semibold uppercase text-drape">Sensory Gating</span>
            <span className="font-mono text-[11px] text-semantic-success">-{telemetry.reduction_pct}% TOKENS</span>
          </div>
          <div className="mt-3 space-y-2 font-mono text-xs text-ink-soft">
            <div>Raw Context: <span className="text-ink">{telemetry.raw_prompt_tokens.toLocaleString()}</span></div>
            <div>Gated Context: <span className="text-semantic-success font-semibold">{telemetry.gated_prompt_tokens.toLocaleString()}</span></div>
            <div>Tokens Pruned: <span className="text-drape font-bold">{telemetry.tokens_saved.toLocaleString()}</span></div>
          </div>
        </div>

        {/* Brain Lockstep Card */}
        <div className="border border-rule bg-chart p-4">
          <div className="flex items-center justify-between border-b border-rule pb-2">
            <span className="font-mono text-xs font-semibold uppercase text-drape">Brain Health</span>
            <span className="font-mono text-[11px] text-semantic-success">● {telemetry.adapters_present}/{telemetry.adapters_total} Grafts</span>
          </div>
          <div className="mt-3 space-y-2 font-mono text-xs text-ink-soft">
            <div className="truncate">Organ: <span className="text-ink">{telemetry.brain_path}</span></div>
            <div>Doctor Criticals: <span className={telemetry.doctor_criticals > 0 ? 'text-semantic-error font-bold' : 'text-ink'}>{telemetry.doctor_criticals}</span></div>
            <div>Auto-Fixable: <span className="text-drape">{telemetry.doctor_fixable}</span></div>
          </div>
        </div>
      </div>
    </div>
  )
}
