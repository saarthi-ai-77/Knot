import { ReactNode, useState } from 'react'
import { useProjectStore } from '../stores/projectStore'
import { Titlebar } from './Titlebar'
import { Sidebar } from './Sidebar'
import { Statusbar } from './Statusbar'
import { LandingPage } from './LandingPage'

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  const { currentProject } = useProjectStore()
  const [activeTab, setActiveTab] = useState('dashboard')

  if (!currentProject) {
    return <LandingPage />
  }

  return (
    <div className="flex flex-col h-screen bg-knot-bg text-knot-text overflow-hidden">
      <Titlebar />
      
      <div className="flex flex-1 overflow-hidden">
        <Sidebar activeTab={activeTab} onTabChange={setActiveTab} />
        
        <main className="flex-1 overflow-auto bg-knot-bg">
          {children}
        </main>
      </div>
      
      <Statusbar />
    </div>
  )
}
