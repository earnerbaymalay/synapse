import { Card, PageHeader, Toolbar, ToolbarButton } from '../components/ui'

const LOG_LINES = [
  {
    level: 'error',
    message: 'Adapter mismatch',
    at: '/workspace/llm-neurosurgeon/adapter-smith/...',
    time: '2026-07-07 14:23:12',
  },
  {
    level: 'warn',
    message: 'Deprecated config format detected',
    at: '~/.config/zed/AGENTS.md',
    time: '2026-07-07 14:22:45',
  },
]

const LEVEL_COLOR: Record<string, string> = {
  error: 'text-red-400',
  warn: 'text-amber-400',
}

export function DebugConsole() {
  return (
    <div>
      <PageHeader title="Debug Console" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="🔧 Log Level: INFO">
          <p className="text-sm text-slate-400">Filter: Adapter</p>
          <p className="text-sm text-slate-400">Show: Errors Only</p>
        </Card>
        <Card title="📅 Timestamp: 2026-07-07 14:23">
          <p className="text-sm text-slate-400">Limit: Last 100</p>
          <p className="text-sm text-slate-400">Format: JSON</p>
        </Card>
      </div>

      <div className="mb-4 rounded-lg border border-slate-800 bg-black/40 p-4 font-mono text-xs leading-relaxed">
        {LOG_LINES.map((line, i) => (
          <div key={i} className={i > 0 ? 'mt-3' : undefined}>
            <p className={LEVEL_COLOR[line.level]}>
              {line.level}: {line.message}
            </p>
            <p className="text-slate-500">at {line.at}</p>
            <p className="text-slate-500">Time: {line.time}</p>
            <p className="text-slate-600">Stack: ...</p>
          </div>
        ))}
      </div>

      <Toolbar>
        <ToolbarButton>← Filter</ToolbarButton>
        <ToolbarButton>Clear</ToolbarButton>
        <ToolbarButton>Copy</ToolbarButton>
        <ToolbarButton>Export</ToolbarButton>
        <ToolbarButton primary>Close</ToolbarButton>
      </Toolbar>
    </div>
  )
}
