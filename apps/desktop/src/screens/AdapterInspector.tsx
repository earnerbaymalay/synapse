import { useState } from 'react'
import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'

const ADAPTERS = [
  { name: 'GitHub Copilot', status: 'Active', detail: 'Version: 1.2.3', tone: 'ok' as const },
  { name: 'Zed', status: 'Active', detail: 'Version: 0.4.1', tone: 'ok' as const },
  { name: 'Continue', status: 'Inactive', detail: 'License: MIT', tone: 'idle' as const },
  { name: 'Gemini CLI', status: 'Inactive', detail: 'Version: 1.0.0', tone: 'idle' as const },
]

export function AdapterInspector() {
  const [selected, setSelected] = useState(ADAPTERS[0].name)

  return (
    <div>
      <PageHeader title="Adapter Inspector" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="🔌 Adapter List">
          <ul className="space-y-1">
            {ADAPTERS.map((a) => (
              <li key={a.name}>
                <button
                  onClick={() => setSelected(a.name)}
                  className={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
                    selected === a.name ? 'bg-primary-500/15 text-primary-400' : 'hover:bg-slate-800'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span>{a.name}</span>
                    <StatusPill tone={a.tone}>{a.status}</StatusPill>
                  </div>
                  <p className="mt-0.5 text-xs text-slate-500">{a.detail}</p>
                </button>
              </li>
            ))}
          </ul>
        </Card>
        <Card title="📊 Adapter Status">
          <ul className="space-y-3 text-sm">
            {ADAPTERS.map((a) => (
              <li key={a.name} className="flex items-center justify-between">
                <span>{a.name}</span>
                <StatusPill tone={a.tone}>{a.status}</StatusPill>
              </li>
            ))}
          </ul>
        </Card>
      </div>

      <Card className="mb-4">
        <p className="text-sm">
          Selected: <span className="font-medium text-white">{selected}</span>
        </p>
        <p className="mt-1 text-sm text-slate-400">
          Config Path: <code className="font-mono text-slate-300">.github/copilot-instructions.md</code>
        </p>
        <p className="mt-1 text-sm text-slate-400">Discovered: 2/3 adapters missing configs</p>
      </Card>

      <Toolbar>
        <ToolbarButton>← Inspect</ToolbarButton>
        <ToolbarButton>Import</ToolbarButton>
        <ToolbarButton primary>Export</ToolbarButton>
      </Toolbar>
    </div>
  )
}
