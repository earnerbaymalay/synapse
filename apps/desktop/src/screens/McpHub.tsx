import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'

const SERVERS = [
  { name: 'Anthropic', status: 'Connected', tone: 'ok' as const },
  { name: 'OpenAI', status: 'Disconnected', detail: 'Connection Refused', tone: 'error' as const },
]

export function McpHub() {
  return (
    <div>
      <PageHeader title="MCP Hub" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="🔌 MCP Servers">
          <ul className="space-y-2 text-sm">
            {SERVERS.map((s) => (
              <li key={s.name}>
                <div className="flex items-center justify-between">
                  <span>{s.name}</span>
                  <StatusPill tone={s.tone}>{s.status}</StatusPill>
                </div>
                {s.detail && <p className="text-xs text-slate-500">{s.detail}</p>}
              </li>
            ))}
          </ul>
        </Card>
        <Card title="📊 Health Status">
          <div className="space-y-3 text-sm">
            <div>
              <StatusPill tone="ok">✅ OK</StatusPill>
              <p className="mt-1 text-xs text-slate-500">Last Check: 1m ago</p>
            </div>
            <div>
              <StatusPill tone="error">❌ Error</StatusPill>
              <p className="mt-1 text-xs text-slate-500">Last Check: 5m ago</p>
            </div>
          </div>
        </Card>
      </div>

      <Card className="mb-4" title="Server Details">
        <ul className="space-y-1 text-sm text-slate-300">
          <li>• Name: <span className="text-slate-400">openai-claude</span></li>
          <li>• Type: <span className="text-slate-400">Unified MCP Server</span></li>
          <li>• Capabilities: <span className="text-slate-400">Tools, Resources, Prompts</span></li>
          <li>• Authentication: <span className="text-slate-400">API Key (via .env)</span></li>
        </ul>
      </Card>

      <Toolbar>
        <ToolbarButton>← Select</ToolbarButton>
        <ToolbarButton>Test</ToolbarButton>
        <ToolbarButton>Configure</ToolbarButton>
        <ToolbarButton primary>Disconnect</ToolbarButton>
      </Toolbar>
    </div>
  )
}
