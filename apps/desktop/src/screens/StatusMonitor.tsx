import { Card, PageHeader, StatusPill, Toolbar, ToolbarButton } from '../components/ui'

export function StatusMonitor() {
  return (
    <div>
      <PageHeader title="Status Monitor" />

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="📈 System Health">
          <ul className="space-y-1 text-sm text-slate-300">
            <li>CPU: 45%</li>
            <li>Memory: 2.1/8GB</li>
            <li>Disk: 78%</li>
          </ul>
        </Card>
        <Card title="🔄 Sync Status">
          <ul className="space-y-1 text-sm text-slate-300">
            <li>Last Sync: 2m ago</li>
            <li>Mode: Continuous</li>
            <li>Queue: 3 items</li>
          </ul>
        </Card>
        <Card title="👤 User Activity">
          <p className="text-sm text-slate-300">Last: 5m ago</p>
        </Card>
        <Card title="🕐 Session Time">
          <p className="text-sm text-slate-300">Current: 12m</p>
        </Card>
      </div>

      <Card className="mb-4">
        <div className="flex items-center justify-between">
          <StatusPill tone="error">🚨 Issues: 1 Critical</StatusPill>
          <StatusPill tone="ok">🟢 ALL SYSTEMS GO</StatusPill>
        </div>
      </Card>

      <Toolbar>
        <ToolbarButton>← Refresh</ToolbarButton>
        <ToolbarButton>Filter</ToolbarButton>
        <ToolbarButton primary>Export</ToolbarButton>
      </Toolbar>
    </div>
  )
}
