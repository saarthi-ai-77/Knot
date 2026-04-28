import { useState } from 'react'
import { Sidebar } from './components/Sidebar'
import { Titlebar } from './components/Titlebar'
import { Statusbar } from './components/Statusbar'
import { LandingPage } from './components/LandingPage'
import { DashboardTab } from './components/tabs/DashboardTab'
import { GraphTab } from './components/tabs/GraphTab'
import { AgentsTab } from './components/tabs/AgentsTab'
import { CostTab } from './components/tabs/CostTab'
import { SettingsTab } from './components/tabs/SettingsTab'
import { useProjectStore } from './stores/projectStore'

import './index.css'

type TabId = 'dashboard' | 'graph' | 'agents' | 'cost' | 'settings'

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('dashboard')
  const { currentProject } = useProjectStore()

  // Show landing page if no project is loaded
  if (!currentProject) {
    return (
      <div className="bg-knot-bg min-h-screen">
        <LandingPage />
      </div>
    )
  }

  const renderTab = () => {
    switch (activeTab) {
      case 'dashboard':
        return <DashboardTab />
      case 'graph':
        return <GraphTab />
      case 'agents':
        return <AgentsTab />
      case 'cost':
        return <CostTab />
      case 'settings':
        return <SettingsTab />
      default:
        return <DashboardTab />
    }
  }

  return (
    <div className="bg-knot-bg min-h-screen text-knot-text">
      <Titlebar />
      
      <div className="flex h-[calc(100vh-32px-24px)] overflow-hidden">
        <Sidebar activeTab={activeTab} onTabChange={(tab) => setActiveTab(tab as TabId)} />
        
        <main className="flex-1 overflow-auto bg-knot-bg">
          {renderTab()}
        </main>
      </div>
      
      <Statusbar />
    </div>
  )
}

export default App
