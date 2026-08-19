import { useEffect, useState, ComponentType } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  LayoutDashboard,
  Settings,
  Cpu,
  Activity,
  Terminal,
  Compass,
  Store,
  Server,
} from 'lucide-react'
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

interface NavItem {
  id: ScreenId
  label: string
  icon: ComponentType<{ className?: string }>
}

const NAV: NavItem[] = [
  { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { id: 'config', label: 'Configuration', icon: Settings },
  { id: 'adapters', label: 'Adapters', icon: Cpu },
  { id: 'status', label: 'Vitals & Status', icon: Activity },
  { id: 'debug', label: 'CLI & Debug', icon: Terminal },
  { id: 'onboarding', label: 'Onboarding', icon: Compass },
  { id: 'marketplace', label: 'Marketplace', icon: Store },
  { id: 'mcp', label: 'MCP Hub', icon: Server },
]

function App() {
  const [screen, setScreen] = useState<ScreenId>('dashboard')
  const [version, setVersion] = useState('1.0.0')

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return
    invoke('get_version')
      .then((v) => setVersion(v as string))
      .catch(() => setVersion('1.0.0'))
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
    <div className="flex h-screen bg-ink-950 font-sans text-ink-100 antialiased">
      <aside className="flex w-60 shrink-0 flex-col border-r border-ink-800 bg-ink-900/60">
        <div className="border-b border-ink-800 px-5 py-4">
          <div className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-none bg-gold-500" />
            <p className="font-display text-sm uppercase tracking-wider text-ink-100">SYNAPSE</p>
          </div>
          <p className="mt-0.5 font-mono text-[10px] uppercase tracking-widest text-accent-500">
            llm-neuro-surgeon <span className="text-ink-400">v{version}</span>
          </p>
        </div>
        <nav className="flex-1 space-y-1 p-3 font-mono text-xs">
          {NAV.map((item) => {
            const Icon = item.icon
            const active = screen === item.id
            return (
              <button
                key={item.id}
                onClick={() => setScreen(item.id)}
                className={cn(
                  'flex w-full items-center gap-3 rounded-none px-3 py-2 text-left uppercase tracking-wider transition-colors',
                  active
                    ? 'border-l-2 border-accent-500 bg-accent-500/10 text-accent-300 font-semibold'
                    : 'text-ink-300 hover:bg-ink-800/50 hover:text-ink-100',
                )}
              >
                <Icon className={cn('h-4 w-4 shrink-0', active ? 'text-accent-500' : 'text-ink-400')} />
                {item.label}
              </button>
            )
          })}
        </nav>
        <div className="border-t border-ink-800 p-4 font-mono text-[10px] text-ink-400">
          <p className="uppercase">One Brain. All Models.</p>
          <p className="mt-0.5 text-accent-500/80">~/AIBrain</p>
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto p-6 bg-ink-950">
        <ActiveScreen />
      </main>
    </div>
  )
}

export default App
