export type ScreenId =
  | 'dashboard'
  | 'config'
  | 'adapters'
  | 'status'
  | 'debug'
  | 'onboarding'
  | 'marketplace'
  | 'mcp'

export interface ScreenProps {
  onNavigate: (screen: ScreenId) => void
}
