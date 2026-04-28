import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

export interface ParsedEntity {
  name: string
  kind: 'function' | 'class' | 'interface' | 'import' | 'export' | 'type'
  line_start: number
  line_end: number
  signature: string
  is_public: boolean
}

export interface ParsedRelationship {
  source_name: string
  target_name: string
  kind: 'imports' | 'exports' | 'calls' | 'extends' | 'implements'
}

export interface ParsedFile {
  file_path: string
  language: string
  entities: ParsedEntity[]
  relationships: ParsedRelationship[]
}

export interface ScanProgress {
  total: number
  completed: number
  failed: number
  current_file: string | null
}

interface GraphState {
  parsedFiles: Map<string, ParsedFile>
  scanProgress: ScanProgress | null
  isScanning: boolean
  error: string | null
  
  // Actions
  parseFile: (filePath: string) => Promise<ParsedFile>
  parseProject: (projectId: string) => Promise<string>
  getScanProgress: (projectId: string) => Promise<ScanProgress>
  setScanProgress: (progress: ScanProgress | null) => void
  setIsScanning: (isScanning: boolean) => void
  clearError: () => void
}

export const useGraphStore = create<GraphState>((set, get) => ({
  parsedFiles: new Map(),
  scanProgress: null,
  isScanning: false,
  error: null,
  
  parseFile: async (filePath) => {
    set({ error: null })
    try {
      const parsed = await invoke<ParsedFile>('parse_file', { filePath })
      const { parsedFiles } = get()
      parsedFiles.set(filePath, parsed)
      set({ parsedFiles: new Map(parsedFiles) })
      return parsed
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },
  
  parseProject: async (projectId) => {
    set({ isScanning: true, error: null })
    try {
      const result = await invoke<string>('parse_project', { projectId })
      // Start polling for progress
      startProgressPolling(projectId, get)
      return result
    } catch (error) {
      set({ error: String(error), isScanning: false })
      throw error
    }
  },
  
  getScanProgress: async (projectId) => {
    try {
      const progress = await invoke<ScanProgress>('get_scan_progress', { projectId })
      set({ scanProgress: progress })
      return progress
    } catch (error) {
      set({ error: String(error) })
      throw error
    }
  },
  
  setScanProgress: (progress) => {
    set({ scanProgress: progress })
  },
  
  setIsScanning: (isScanning) => {
    set({ isScanning })
  },
  
  clearError: () => {
    set({ error: null })
  },
}))

function startProgressPolling(projectId: string, getState: () => GraphState) {
  const poll = async () => {
    try {
      const progress = await invoke<ScanProgress>('get_scan_progress', { projectId })
      getState().setScanProgress(progress)
      
      if (progress.completed + progress.failed < progress.total) {
        // Still scanning, poll again in 500ms
        setTimeout(poll, 500)
      } else {
        // Scan complete
        getState().setIsScanning(false)
      }
    } catch (error) {
      console.error('Failed to get scan progress:', error)
      getState().setIsScanning(false)
    }
  }
  
  poll()
}
