import { create } from 'zustand'

export interface AgentSession {
  id: string
  project_id: string
  agent_type: string
  status: 'active' | 'idle' | 'disconnected' | 'error'
  context_pack_version: number
  current_task?: string
  last_context_pack_id?: string
  created_at: number
  last_active_at: number
  resumed_at?: number
  resume_count: number
}

interface AgentState {
  sessions: AgentSession[]
  currentSession: AgentSession | null
  isLoading: boolean
  error: string | null
  
  // Actions
  loadSessions: (projectId: string) => Promise<void>
  createSession: (projectId: string, agentType: string) => Promise<AgentSession>
  setCurrentSession: (session: AgentSession | null) => void
  clearError: () => void
}

export const useAgentStore = create<AgentState>((set) => ({
  sessions: [],
  currentSession: null,
  isLoading: false,
  error: null,
  
  loadSessions: async (projectId) => {
    set({ isLoading: true, error: null })
    try {
      // This will be implemented when we add agent commands
      set({ isLoading: false })
    } catch (error) {
      set({ error: String(error), isLoading: false })
    }
  },
  
  createSession: async (projectId, agentType) => {
    set({ isLoading: true, error: null })
    try {
      // This will be implemented when we add agent commands
      const session: AgentSession = {
        id: crypto.randomUUID(),
        project_id: projectId,
        agent_type: agentType,
        status: 'active',
        context_pack_version: 1,
        created_at: Date.now(),
        last_active_at: Date.now(),
        resume_count: 0,
      }
      set((state) => ({
        sessions: [session, ...state.sessions],
        currentSession: session,
        isLoading: false,
      }))
      return session
    } catch (error) {
      set({ error: String(error), isLoading: false })
      throw error
    }
  },
  
  setCurrentSession: (session) => {
    set({ currentSession: session })
  },
  
  clearError: () => {
    set({ error: null })
  },
}))
