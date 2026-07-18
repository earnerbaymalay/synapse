import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'

const TRENDING = ['github-copilot', 'vscode-cline', 'zed-agents']
const CATEGORIES = ['Skills', 'MCP', 'Adapters']

export function Marketplace() {
  return (
    <div>
      <PageHeader title="Marketplace" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="🔍 Search">
          <input
            type="text"
            placeholder="adapter ..."
            className="w-full rounded-md border border-slate-800 bg-slate-800/60 px-3 py-1.5 text-sm text-slate-200 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none"
          />
          <div className="mt-3 flex items-center justify-between rounded-md bg-slate-800/60 px-3 py-2 text-sm">
            <span>GitHub Copilot</span>
            <div className="flex items-center gap-2">
              <StatusPill tone="ok">Active</StatusPill>
              <span className="text-xs text-slate-500">MIT</span>
            </div>
          </div>
        </Card>
        <Card title="📦 Categories">
          <div className="mb-3 flex gap-2">
            {CATEGORIES.map((c) => (
              <span
                key={c}
                className="rounded-full border border-slate-700 px-2.5 py-0.5 text-xs text-slate-300"
              >
                {c}
              </span>
            ))}
          </div>
          <p className="mb-1 text-xs font-medium text-slate-400">📊 Trending</p>
          <ul className="space-y-1 text-sm text-slate-300">
            {TRENDING.map((t) => (
              <li key={t}>• {t}</li>
            ))}
          </ul>
        </Card>
      </div>

      <Toolbar>
        <ToolbarButton primary>Install</ToolbarButton>
        <ToolbarButton>Details</ToolbarButton>
        <ToolbarButton>Reviews</ToolbarButton>
        <ToolbarButton>Recommend</ToolbarButton>
      </Toolbar>
    </div>
  )
}
