import { useState, useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useProjectStore } from '../../stores/projectStore'
import { 
  Layers, 
  GitBranch, 
  FileText, 
  Bot, 
  Check,
  FileCode,
  FolderOpen
} from 'lucide-react'

interface Stats {
  entities: number
  relationships: number
  decisions: number
  sessions: number
}

interface EventRow {
  id: string
  event_type: string
  file_path: string
  author: string | null
  timestamp: number
}

interface ScanProgress {
  total: number
  completed: number
  failed: number
  current_file: string | null
  percentage: number
}

export function DashboardTab() {
  const { currentProject } = useProjectStore()
  const queryClient = useQueryClient()
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null)
  const [isScanning, setIsScanning] = useState(false)

  // Stats queries
  const { data: stats } = useQuery({
    queryKey: ['dashboardStats', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return null
      const [entities, relationships, decisions, sessions] = await Promise.all([
        invoke<number>('get_entity_count', { projectId: currentProject.id }),
        invoke<number>('get_relationship_count', { projectId: currentProject.id }),
        invoke<number>('get_decision_count', { projectId: currentProject.id }),
        invoke<number>('get_session_count', { projectId: currentProject.id }),
      ])
      return { entities, relationships, decisions, sessions }
    },
    enabled: !!currentProject,
    refetchInterval: 10000,
  })

  // Recent events
  const { data: events } = useQuery({
    queryKey: ['recentEvents', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return []
      return invoke<EventRow[]>('get_recent_events', { 
        projectId: currentProject.id,
        limit: 10 
      })
    },
    enabled: !!currentProject,
    refetchInterval: 5000,
  })

  // Listen for scan events
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = []

    // Scan started
    unlisteners.push(
      listen('knot://scan-started', (event: any) => {
        const payload = event.payload
        setIsScanning(true)
        setScanProgress({
          total: payload.total_files,
          completed: 0,
          failed: 0,
          current_file: null,
          percentage: 0,
        })
      })
    )

    // Scan progress
    unlisteners.push(
      listen('knot://scan-progress', (event: any) => {
        const payload = event.payload
        setScanProgress({
          total: payload.total,
          completed: payload.completed,
          failed: payload.failed,
          current_file: payload.current_file,
          percentage: payload.percentage,
        })
      })
    )

    // Scan complete
    unlisteners.push(
      listen('knot://scan-complete', () => {
        setIsScanning(false)
        // Refresh all stats
        queryClient.invalidateQueries({ queryKey: ['dashboardStats'] })
        queryClient.invalidateQueries({ queryKey: ['recentEvents'] })
        queryClient.invalidateQueries({ queryKey: ['entity-count'] })
        queryClient.invalidateQueries({ queryKey: ['relationship-count'] })
      })
    )

    // Graph updated (from file watcher)
    unlisteners.push(
      listen('knot://graph-updated', () => {
        // Refresh on file watcher updates
        queryClient.invalidateQueries({ queryKey: ['dashboardStats'] })
        queryClient.invalidateQueries({ queryKey: ['recentEvents'] })
        queryClient.invalidateQueries({ queryKey: ['entity-count'] })
      })
    )

    return () => {
      unlisteners.forEach(p => p.then(f => f()))
    }
  }, [queryClient])

  const statCards = [
    { 
      label: 'Entities Indexed', 
      value: stats?.entities ?? 0, 
      icon: Layers,
      color: 'text-knot-cyan'
    },
    { 
      label: 'Relationships Mapped', 
      value: stats?.relationships ?? 0, 
      icon: GitBranch,
      color: 'text-knot-emerald'
    },
    { 
      label: 'Decisions Recorded', 
      value: stats?.decisions ?? 0, 
      icon: FileText,
      color: 'text-knot-amber'
    },
    { 
      label: 'Agent Sessions', 
      value: stats?.sessions ?? 0, 
      icon: Bot,
      color: 'text-knot-purple'
    },
  ]

  const getEventColor = (type: string) => {
    switch (type) {
      case 'created': return 'bg-knot-emerald'
      case 'modified': return 'bg-knot-amber'
      case 'deleted': return 'bg-knot-red'
      default: return 'bg-knot-muted'
    }
  }

  const formatRelativeTime = (timestamp: number) => {
    const seconds = Math.floor((Date.now() / 1000 - timestamp))
    if (seconds < 60) return 'just now'
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`
    return `${Math.floor(seconds / 86400)}d ago`
  }

  const getCurrentFileName = (path: string | null) => {
    if (!path) return ''
    const parts = path.split(/[\/\\]/)
    // Return last 2 segments if available
    if (parts.length >= 2) {
      return `${parts[parts.length - 2]}/${parts[parts.length - 1]}`
    }
    return parts[parts.length - 1] || path
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      {/* Stat Cards */}
      <div className="grid grid-cols-4 gap-4 mb-8">
        {statCards.map((card) => {
          const Icon = card.icon
          return (
            <div 
              key={card.label}
              className="bg-knot-panel border border-knot-border p-4 hover:border-knot-cyan/50 transition-knot group"
            >
              <div className="flex items-center justify-between mb-2">
                <Icon className={`w-5 h-5 ${card.color}`} />
              </div>
              <div className="font-mono text-2xl font-bold text-knot-text-bright group-hover:text-knot-cyan transition-knot">
                {card.value.toLocaleString()}
              </div>
              <div className="text-[10px] uppercase tracking-wider text-knot-muted mt-1">
                {card.label}
              </div>
            </div>
          )
        })}
      </div>

      {/* Two Column Layout */}
      <div className="grid grid-cols-2 gap-6">
        {/* Left - Recent Activity */}
        <div className="bg-knot-panel border border-knot-border">
          <div className="px-4 py-3 border-b border-knot-border flex items-center justify-between">
            <h3 className="font-heading font-semibold text-sm text-knot-text-bright">
              Recent Activity
            </h3>
          </div>
          <div className="p-0">
            {events?.length === 0 ? (
              <div className="p-8 text-center text-knot-muted text-sm">
                No activity yet. Start scanning a project.
              </div>
            ) : (
              <div className="divide-y divide-knot-border">
                {events?.map((event) => (
                  <div 
                    key={event.id}
                    className="px-4 py-3 flex items-center gap-3 hover:bg-knot-panel-hover transition-knot"
                  >
                    <div className={`w-2 h-2 rounded-full ${getEventColor(event.event_type)}`} />
                    <div className="flex-1 min-w-0">
                      <div className="font-mono text-xs text-knot-text-bright truncate">
                        {event.file_path.split('/').pop() || event.file_path}
                      </div>
                      <div className="text-[10px] text-knot-muted truncate">
                        {event.file_path}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`
                        text-[10px] px-1.5 py-0.5 rounded border
                        ${event.event_type === 'created' ? 'bg-knot-emerald/10 text-knot-emerald border-knot-emerald/30' : ''}
                        ${event.event_type === 'modified' ? 'bg-knot-amber/10 text-knot-amber border-knot-amber/30' : ''}
                        ${event.event_type === 'deleted' ? 'bg-knot-red/10 text-knot-red border-knot-red/30' : ''}
                      `}>
                        {event.event_type}
                      </span>
                      <span className="text-[10px] text-knot-muted">
                        {formatRelativeTime(event.timestamp)}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Open Project Button */}
          <div className="p-4 border-t border-knot-border">
            <button className="w-full bg-knot-cyan text-knot-bg font-heading font-semibold text-sm py-2 px-4 rounded hover:bg-knot-cyan/90 transition-knot flex items-center justify-center gap-2">
              <FolderOpen className="w-4 h-4" />
              Open Project
            </button>
          </div>
        </div>

        {/* Right - Scan Status */}
        <div className="bg-knot-panel border border-knot-border">
          <div className="px-4 py-3 border-b border-knot-border">
            <h3 className="font-heading font-semibold text-sm text-knot-text-bright">
              Scan Status
            </h3>
          </div>
          <div className="p-4">
            {isScanning && scanProgress ? (
              <>
                {/* Progress bar */}
                <div className="mb-4">
                  <div className="flex items-center justify-between mb-2 text-sm">
                    <span className="text-knot-text">
                      {scanProgress.completed} of {scanProgress.total} files indexed
                    </span>
                    <span className="text-knot-muted">
                      {scanProgress.percentage}%
                    </span>
                  </div>
                  <div className="h-2 bg-knot-border rounded-full overflow-hidden">
                    <div 
                      className="h-full bg-knot-cyan transition-all duration-300"
                      style={{ 
                        width: `${scanProgress.percentage}%` 
                      }}
                    />
                  </div>
                </div>

                {/* Current file */}
                {scanProgress.current_file && (
                  <div className="flex items-center gap-2 text-xs text-knot-muted mb-4">
                    <FileCode className="w-3 h-3" />
                    <span className="font-mono truncate">
                      {getCurrentFileName(scanProgress.current_file)}
                    </span>
                  </div>
                )}
              </>
            ) : (
              <div className="flex items-center gap-2 text-knot-emerald">
                <Check className="w-4 h-4" />
                <span className="text-sm">Graph is up to date</span>
              </div>
            )}

            {/* Completed files list */}
            <div className="mt-4">
              <h4 className="text-[10px] uppercase tracking-wider text-knot-muted mb-2">
                Recently Indexed
              </h4>
              <div className="space-y-1">
                <div className="text-xs text-knot-muted py-2 text-center">
                  {isScanning ? 'Scanning...' : 'No recent scans'}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
