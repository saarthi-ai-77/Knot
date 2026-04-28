import { 
  LayoutDashboard, 
  GitBranch, 
  Bot, 
  DollarSign, 
  Settings 
} from 'lucide-react'

interface SidebarProps {
  activeTab: string
  onTabChange: (tab: string) => void
}

interface NavItem {
  id: string
  label: string
  icon: React.ElementType
}

const navItems: NavItem[] = [
  { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { id: 'graph', label: 'Graph', icon: GitBranch },
  { id: 'agents', label: 'Agents', icon: Bot },
  { id: 'cost', label: 'Cost', icon: DollarSign },
  { id: 'settings', label: 'Settings', icon: Settings },
]

export function Sidebar({ activeTab, onTabChange }: SidebarProps) {
  return (
    <div 
      className="w-16 flex flex-col items-center py-4 border-r shrink-0"
      style={{
        backgroundColor: '#0d1117',
        borderColor: '#1e2433',
      }}
    >
      <nav className="flex flex-col gap-1 w-full">
        {navItems.map((item) => {
          const isActive = activeTab === item.id
          const Icon = item.icon
          
          return (
            <button
              key={item.id}
              onClick={() => onTabChange(item.id)}
              className="flex flex-col items-center justify-center py-3 px-2 mx-1 rounded transition-all duration-120 group relative"
              style={{
                backgroundColor: isActive ? 'rgba(0, 212, 255, 0.06)' : 'transparent',
                borderLeft: isActive ? '2px solid #00d4ff' : '2px solid transparent',
                marginLeft: isActive ? '0' : '0',
                paddingLeft: isActive ? '6px' : '8px',
              }}
              title={item.label}
            >
              <Icon 
                className="w-[18px] h-[18px] transition-colors"
                style={{
                  color: isActive ? '#00d4ff' : '#6b7280',
                }}
              />
              <span 
                className="text-[10px] mt-1 font-heading transition-colors"
                style={{
                  fontFamily: "'Syne', sans-serif",
                  color: isActive ? '#00d4ff' : '#6b7280',
                }}
              >
                {item.label}
              </span>
            </button>
          )
        })}
      </nav>
    </div>
  )
}
