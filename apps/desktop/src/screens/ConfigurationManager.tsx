import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'

export function ConfigurationManager() {
  return (
    <div>
      <PageHeader title="Configuration Manager" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="🔧 Adapter Settings">
          <div className="flex items-center justify-between rounded-md bg-slate-800/60 px-3 py-2 text-sm">
            <span>GitHub Copilot</span>
            <StatusPill tone="ok">OK</StatusPill>
          </div>
        </Card>
        <Card title="🌐 Network Config">
          <div className="rounded-md bg-slate-800/60 px-3 py-2 text-sm">
            MCP Server Endpoints
          </div>
        </Card>
      </div>

      <Card className="mb-4">
        <ul className="space-y-1.5 text-sm text-slate-300">
          <li>• Export Format: <span className="text-slate-400">JSON (Default)</span></li>
          <li>• Symlink Policy: <span className="text-slate-400">Read-Only</span></li>
          <li>• Backup Enabled: <span className="text-slate-400">Yes</span></li>
        </ul>
      </Card>

      <Toolbar>
        <ToolbarButton>← Cancel</ToolbarButton>
        <ToolbarButton>Apply</ToolbarButton>
        <ToolbarButton primary>Save</ToolbarButton>
      </Toolbar>
    </div>
  )
}
