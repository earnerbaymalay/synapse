import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'
import type { ScreenProps } from './types'

const PROJECTS = [
  { name: 'Project A', state: 'Active', status: 'OK', tone: 'ok' as const },
  { name: 'Project B', state: 'Syncing', status: 'Syncing...', tone: 'warn' as const },
  { name: 'Project C', state: 'Ready', status: 'Ready', tone: 'ok' as const },
]

export function MainDashboard({ onNavigate }: ScreenProps) {
  return (
    <div>
      <PageHeader title="LLM Neurosurgeon" subtitle="Main Dashboard" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="📊 Adapters">
          <p className="text-2xl font-semibold text-white">12</p>
          <p className="text-sm text-slate-400">💾 3 GB Used</p>
        </Card>
        <Card title="🛡️ Gate Status">
          <StatusPill tone="ok">✅ Tauri v2.0</StatusPill>
        </Card>
      </div>

      <div className="mb-4 grid grid-cols-3 gap-4">
        <button
          onClick={() => onNavigate('adapters')}
          className="rounded-lg border border-slate-800 bg-slate-900/60 p-4 text-left transition-colors hover:border-primary-500"
        >
          🔍 Projects
        </button>
        <button
          onClick={() => onNavigate('marketplace')}
          className="rounded-lg border border-slate-800 bg-slate-900/60 p-4 text-left transition-colors hover:border-primary-500"
        >
          🏪 Marketplace
        </button>
        <button
          onClick={() => onNavigate('config')}
          className="rounded-lg border border-slate-800 bg-slate-900/60 p-4 text-left transition-colors hover:border-primary-500"
        >
          📁 Tools
        </button>
      </div>

      <Card className="mb-4">
        <ul className="divide-y divide-slate-800">
          {PROJECTS.map((p) => (
            <li key={p.name} className="flex items-center justify-between py-2 text-sm">
              <span>
                {p.name} <span className="text-slate-500">({p.state})</span>
              </span>
              <StatusPill tone={p.tone}>{p.status}</StatusPill>
            </li>
          ))}
        </ul>
      </Card>

      <Toolbar>
        <ToolbarButton>← ↑ →</ToolbarButton>
        <ToolbarButton>Help</ToolbarButton>
        <ToolbarButton>Apply</ToolbarButton>
        <ToolbarButton primary>Save</ToolbarButton>
      </Toolbar>
    </div>
  )
}
