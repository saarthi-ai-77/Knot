import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../stores/projectStore'
import { Bot, Database, GitBranch } from 'lucide-react'

interface StatusBarInfo {
  scan_status: string
  scan_progress: {
    total: number
    completed: number
    failed: number
    current_file: string | null
  } | null
  db_size_mb: number
  active_agents: number
}

export function Statusbar() {
  const { currentProject } = useProjectStore()
  
  const { data: status } = useQuery({
    queryKey: ['statusBar', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return null
      return invoke<StatusBarInfo>('get_status_bar_info', { 
        projectId: currentProject.id 
      })
    },
    enabled: !!currentProject,
    refetchInterval: 2000,
  })

  const isScanning = status?.scan_status === 'scanning' && status.scan_progress
  const progress = status?.scan_progress

  return (
    <div 
      className="h-6 flex items-center justify-between px-4 text-[11px] font-mono shrink-0"
      style={{
        backgroundColor: '#0d1117',
        borderTop: '1px solid #1e2433',
        color: '#9ca3af',
      }}
    >
      {/* Left - Scan Progress */}
      <div className="flex items-center gap-3">
        {isScanning && progress ? (
          <>
            <GitBranch style={{ width: '12px', height: '12px', color: '#00d4ff' }} className="animate-pulse" />
            <span style={{ color: '#9ca3af' }}>Indexing:</span>
            <span style={{ color: '#00d4ff' }}>
              {progress.completed}/{progress.total}
            </span>
            {/* Mini progress bar */}
            <div 
              className="rounded-full overflow-hidden"
              style={{ width: '96px', height: '4px', backgroundColor: '#1e2433' }}
            >
              <div 
                className="h-full transition-all duration-300"
                style={{ 
                  width: `${(progress.completed / Math.max(progress.total, 1)) * 100}%`,
                  backgroundColor: '#00d4ff',
                }}
              />
            </div>
            {progress.current_file && (
              <span 
                className="truncate max-w-[200px]"
                style={{ color: '#6b7280' }}
              >
                {progress.current_file.split('/').pop()}
              </span>
            )}
          </>
        ) : (
          <>
            <div 
              className="w-1.5 h-1.5 rounded-full"
              style={{ backgroundColor: '#10b981' }}
            />
            <span style={{ color: '#6b7280' }}>Graph current</span>
          </>
        )}
      </div>

      {/* Center - DB Size */}
      <div className="flex items-center gap-2" style={{ color: '#6b7280' }}>
        <Database style={{ width: '12px', height: '12px' }} />
        <span>DB: {status?.db_size_mb.toFixed(1) ?? '0.0'} MB</span>
      </div>

      {/* Right - Active Agents */}
      <div className="flex items-center gap-2">
        <Bot style={{ width: '12px', height: '12px', color: '#9ca3af' }} />
        {status?.active_agents && status.active_agents > 0 ? (
          <span style={{ color: '#00d4ff' }}>
            {status.active_agents} active agent{status.active_agents !== 1 ? 's' : ''}
          </span>
        ) : (
          <span style={{ color: '#6b7280' }}>No active agents</span>
        )}
      </div>
    </div>
  )
}
