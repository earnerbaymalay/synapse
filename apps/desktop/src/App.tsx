import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { cn } from './lib/utils'
import type { ScreenId } from './screens/types'
import { MainDashboard } from './screens/MainDashboard'
import { ConfigurationManager } from './screens/ConfigurationManager'
import { AdapterInspector } from './screens/AdapterInspector'
import { StatusMonitor } from './screens/StatusMonitor'
import { DebugConsole } from './screens/DebugConsole'
import { OnboardingWizard } from './screens/OnboardingWizard'
import { Marketplace } from './screens/Marketplace'
import { McpHub } from './screens/McpHub'

const NAV: { id: ScreenId; label: string; icon: string }[] = [
  { id: 'dashboard', label: 'Main Dashboard', icon: '📊' },
  { id: 'config', label: 'Configuration Manager', icon: '🔧' },
  { id: 'adapters', label: 'Adapter Inspector', icon: '🔌' },
  { id: 'status', label: 'Status Monitor', icon: '📈' },
  { id: 'debug', label: 'Debug Console', icon: '🐛' },
  { id: 'onboarding', label: 'Onboarding Wizard', icon: '🎯' },
  { id: 'marketplace', label: 'Marketplace', icon: '🏪' },
  { id: 'mcp', label: 'MCP Hub', icon: '🔗' },
]

function App() {
  const [screen, setScreen] = useState<ScreenId>('dashboard')
  const [version, setVersion] = useState('—')

  useEffect(() => {
    // Not running inside a Tauri webview (e.g. previewed in a plain
    // browser for screenshots) — invoke() has no backend to call.
    if (!('__TAURI_INTERNALS__' in window)) return
    invoke('get_version')
      .then((v) => setVersion(v as string))
      .catch(() => setVersion('—'))
  }, [])

  const ActiveScreen = () => {
    switch (screen) {
      case 'dashboard':
        return <MainDashboard onNavigate={setScreen} />
      case 'config':
        return <ConfigurationManager />
      case 'adapters':
        return <AdapterInspector />
      case 'status':
        return <StatusMonitor />
      case 'debug':
        return <DebugConsole />
      case 'onboarding':
        return <OnboardingWizard />
      case 'marketplace':
        return <Marketplace />
      case 'mcp':
        return <McpHub />
    }
  }

  return (
    <div className="flex h-screen bg-slate-950 text-slate-200">
      <aside className="flex w-56 shrink-0 flex-col border-r border-slate-800 bg-slate-900/40">
        <div className="border-b border-slate-800 px-4 py-4">
          <p className="text-sm font-semibold text-white">LLM Neurosurgeon</p>
          <p className="text-xs text-slate-500">v{version}</p>
        </div>
        <nav className="flex-1 space-y-0.5 p-2">
          {NAV.map((item) => (
            <button
              key={item.id}
              onClick={() => setScreen(item.id)}
              className={cn(
                'flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors',
                screen === item.id
                  ? 'bg-primary-500/15 text-primary-400'
                  : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200',
              )}
            >
              <span>{item.icon}</span>
              {item.label}
            </button>
          ))}
        </nav>
      </aside>

      <main className="flex-1 overflow-y-auto p-6">
        <ActiveScreen />
      </main>
    </div>
  )
}

export default App
