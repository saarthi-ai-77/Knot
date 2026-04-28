import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../../stores/projectStore'
import { Bot, Clock, FileText } from 'lucide-react'

interface AgentSession {
  id: string
  agent_type: string
  status: string
  current_task: string | null
  last_active_at: number
  open_files_count: number
}

const agentTypes: Record<string, { label: string; className: string }> = {
  'claude-code': { label: 'Claude Code', className: 'agent-claude' },
  'cursor': { label: 'Cursor', className: 'agent-cursor' },
  'opencode': { label: 'OpenCode', className: 'agent-opencode' },
  'copilot': { label: 'Copilot', className: 'agent-copilot' },
}

export function AgentsTab() {
  const { currentProject } = useProjectStore()
  const [taskDescription, setTaskDescription] = useState('')

  const { data: sessions } = useQuery({
    queryKey: ['activeSessions', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return []
      return invoke<AgentSession[]>('get_active_sessions', { 
        projectId: currentProject.id 
      })
    },
    enabled: !!currentProject,
    refetchInterval: 5000,
  })

  const getAgentBadge = (type: string) => {
    const agent = agentTypes[type] || agentTypes['copilot']
    return agent
  }

  const getRelativeTime = (timestamp: number) => {
    const seconds = Math.floor((Date.now() / 1000 - timestamp))
    if (seconds < 60) return 'just now'
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`
    return `${Math.floor(seconds / 86400)}d ago`
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="mb-8">
        <h2 className="font-heading font-bold text-lg text-knot-text-bright mb-4">
          Active Sessions
        </h2>
        
        {sessions?.length === 0 ? (
          <div className="bg-knot-panel border border-knot-border p-8 text-center text-knot-muted">
            <Bot className="w-8 h-8 mx-auto mb-2 opacity-50" />
            <p>No active agent sessions</p>
            <p className="text-xs mt-1">Connect an agent to start tracking</p>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-4">
            {sessions?.map((session) => {
              const agent = getAgentBadge(session.agent_type)
              return (
                <div 
                  key={session.id}
                  className="bg-knot-panel border border-knot-border p-4 hover:border-knot-cyan/30 transition-knot"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className={`text-xs px-2 py-1 rounded border ${agent.className}`}>
                        {agent.label}
                      </span>
                      <span className="font-mono text-xs text-knot-muted">
                        {session.id.slice(0, 8)}...
                      </span>
                    </div>
                    <div className="flex items-center gap-1">
                      <div className={`w-2 h-2 rounded-full ${
                        session.status === 'active' ? 'bg-knot-emerald animate-pulse' :
                        session.status === 'idle' ? 'bg-knot-muted' :
                        'bg-knot-red'
                      }`} />
                      <span className="text-xs text-knot-muted capitalize">{session.status}</span>
                    </div>
                  </div>

                  {session.current_task && (
                    <p className="text-sm text-knot-text-bright mb-3">{session.current_task}</p>
                  )}

                  <div className="flex items-center gap-4 text-xs text-knot-muted">
                    <div className="flex items-center gap-1">
                      <FileText className="w-3 h-3" />
                      {session.open_files_count} files open
                    </div>
                    <div className="flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      Last active {getRelativeTime(session.last_active_at)}
                    </div>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-6">
        <div className="bg-knot-panel border border-knot-border">
          <div className="px-4 py-3 border-b border-knot-border">
            <h3 className="font-heading font-semibold text-sm text-knot-text-bright">
              Task Log
            </h3>
          </div>
          <div className="p-4">
            <textarea
              value={taskDescription}
              onChange={(e) => setTaskDescription(e.target.value)}
              placeholder="Describe the current task for cross-agent continuity..."
              className="w-full h-32 bg-knot-bg border border-knot-border rounded p-3 text-sm text-knot-text-bright placeholder:text-knot-muted focus:outline-none focus:border-knot-cyan resize-none"
            />
            <button
              disabled={!taskDescription.trim()}
              className="mt-3 bg-knot-cyan text-knot-bg font-heading font-semibold text-sm py-2 px-4 rounded hover:bg-knot-cyan/90 transition-knot disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Log Task
            </button>
          </div>
        </div>

        <div className="bg-knot-panel border border-knot-border">
          <div className="px-4 py-3 border-b border-knot-border">
            <h3 className="font-heading font-semibold text-sm text-knot-text-bright">
              Resume Context Generator
            </h3>
          </div>
          <div className="p-4">
            <p className="text-xs text-knot-muted mb-4">
              Generate a prompt to help resume work when switching agents or after hitting limits
            </p>
            <button className="w-full bg-knot-cyan text-knot-bg font-heading font-semibold text-sm py-3 px-4 rounded hover:bg-knot-cyan/90 transition-knot">
              Generate Resume Prompt
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
