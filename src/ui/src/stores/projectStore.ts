import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

export interface Project {
  id: string
  name: string
  root_path: string
  tech_stack?: string
  created_at: number
  last_scanned_at?: number
  health_score: number
}

interface CreateProjectRequest {
  name: string
  root_path: string
  tech_stack?: string[]
}

interface ProjectState {
  projects: Project[]
  currentProject: Project | null
  isLoading: boolean
  error: string | null
  
  // Actions
  loadProjects: () => Promise<void>
  createProject: (request: CreateProjectRequest) => Promise<Project>
  loadProject: (projectId: string) => Promise<Project>
  setCurrentProject: (project: Project | null) => void
  openProject: (path: string) => Promise<Project>
  clearError: () => void
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  currentProject: null,
  isLoading: false,
  error: null,
  
  loadProjects: async () => {
    set({ isLoading: true, error: null })
    try {
      const projects = await invoke<Project[]>('get_projects')
      set({ projects, isLoading: false })
    } catch (error) {
      set({ error: String(error), isLoading: false })
    }
  },
  
  createProject: async (request) => {
    set({ isLoading: true, error: null })
    try {
      const project = await invoke<Project>('create_project', { request })
      const { projects } = get()
      set({ 
        projects: [project, ...projects],
        currentProject: project,
        isLoading: false 
      })
      return project
    } catch (error) {
      set({ error: String(error), isLoading: false })
      throw error
    }
  },
  
  loadProject: async (projectId) => {
    set({ isLoading: true, error: null })
    try {
      const project = await invoke<Project>('load_project', { projectId })
      set({ currentProject: project, isLoading: false })
      return project
    } catch (error) {
      set({ error: String(error), isLoading: false })
      throw error
    }
  },
  
  openProject: async (path) => {
    set({ isLoading: true, error: null })
    try {
      const project = await invoke<Project>('open_project', { path })
      set({ 
        currentProject: project,
        isLoading: false 
      })
      return project
    } catch (error) {
      set({ error: String(error), isLoading: false })
      throw error
    }
  },
  
  setCurrentProject: (project) => {
    set({ currentProject: project })
  },
  
  clearError: () => {
    set({ error: null })
  },
}))
