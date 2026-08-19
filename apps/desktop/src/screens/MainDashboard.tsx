import { Cpu, GitBranch, ShieldCheck, Store, Wrench, RefreshCw } from 'lucide-react'
import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'
import type { ScreenProps } from './types'

const PROJECTS = [
  { name: 'llm-neuro-surgeon', state: 'Active', status: 'SYNAPSE IN SYNC', tone: 'ok' as const },
  { name: 'anthropics-skills-bundle', state: 'Marketplace', status: '13 SKILLS LOADED', tone: 'ok' as const },
  { name: 'mcp-server-registry', state: 'Daemon', status: 'WATCHING', tone: 'ok' as const },
]

export function MainDashboard({ onNavigate }: ScreenProps) {
  return (
    <div>
      <PageHeader title="SYNAPSE DASHBOARD" subtitle="One Brain. All Models." />

      <div className="mb-6 grid grid-cols-3 gap-4">
        <Card title="Adapters Registered">
          <div className="flex items-baseline justify-between">
            <p className="font-display text-3xl font-bold text-ink-100">13</p>
            <Cpu className="h-5 w-5 text-accent-500" />
          </div>
          <p className="mt-2 font-mono text-xs text-ink-400">13 / 13 Verified Tool Briefs</p>
        </Card>

        <Card title="Time Machine">
          <div className="flex items-baseline justify-between">
            <p className="font-mono text-sm font-semibold text-accent-300">~/AIBrain</p>
            <GitBranch className="h-5 w-5 text-accent-500" />
          </div>
          <p className="mt-2 font-mono text-xs text-ink-400">Git Commit History Active</p>
        </Card>

        <Card title="System Vitals">
          <div className="flex items-center justify-between">
            <StatusPill tone="ok">Tauri v2.0 Ready</StatusPill>
            <ShieldCheck className="h-5 w-5 text-semantic-success" />
          </div>
          <p className="mt-2 font-mono text-xs text-ink-400">0 Faults Diagnosed</p>
        </Card>
      </div>

      <div className="mb-6 grid grid-cols-3 gap-4 font-mono text-xs">
        <button
          onClick={() => onNavigate('adapters')}
          className="flex items-center gap-3 border border-ink-800 bg-ink-900/60 p-4 uppercase tracking-wider text-ink-200 transition-colors hover:border-accent-500 hover:text-accent-300"
        >
          <Cpu className="h-4 w-4 text-accent-500" />
          Inspect Adapters
        </button>
        <button
          onClick={() => onNavigate('marketplace')}
          className="flex items-center gap-3 border border-ink-800 bg-ink-900/60 p-4 uppercase tracking-wider text-ink-200 transition-colors hover:border-accent-500 hover:text-accent-300"
        >
          <Store className="h-4 w-4 text-accent-500" />
          Marketplace Hub
        </button>
        <button
          onClick={() => onNavigate('config')}
          className="flex items-center gap-3 border border-ink-800 bg-ink-900/60 p-4 uppercase tracking-wider text-ink-200 transition-colors hover:border-accent-500 hover:text-accent-300"
        >
          <Wrench className="h-4 w-4 text-accent-500" />
          Manage Configuration
        </button>
      </div>

      <Card title="Active Brain Targets" className="mb-6">
        <ul className="divide-y divide-ink-800/80 font-mono text-xs">
          {PROJECTS.map((p) => (
            <li key={p.name} className="flex items-center justify-between py-3">
              <span className="text-ink-200">
                {p.name} <span className="text-ink-400">[{p.state}]</span>
              </span>
              <StatusPill tone={p.tone}>{p.status}</StatusPill>
            </li>
          ))}
        </ul>
      </Card>

      <Toolbar>
        <ToolbarButton onClick={() => onNavigate('debug')}>
          <span className="flex items-center gap-1.5">
            <RefreshCw className="h-3.5 w-3.5" />
            Trigger Scan
          </span>
        </ToolbarButton>
        <ToolbarButton onClick={() => onNavigate('config')}>Project Configs</ToolbarButton>
        <ToolbarButton primary onClick={() => onNavigate('status')}>
          Sync Daemon Status
        </ToolbarButton>
      </Toolbar>
    </div>
  )
}
