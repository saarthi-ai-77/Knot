import { useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { useProjectStore } from '../stores/projectStore'
import { FolderOpen, Sparkles, CloudOff, Bot } from 'lucide-react'

export function LandingPage() {
  const [isDragging, setIsDragging] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const { openProject } = useProjectStore()

  const handleBrowse = async () => {
    try {
      setIsLoading(true)
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Open Project Folder',
      })
      
      if (selected && typeof selected === 'string') {
        await openProject(selected)
      }
    } catch (err) {
      console.error('Failed to open project:', err)
    } finally {
      setIsLoading(false)
    }
  }

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    
    const items = Array.from(e.dataTransfer.items)
    for (const item of items) {
      if (item.kind === 'file') {
        const entry = item.webkitGetAsEntry?.() || item.getAsFileSystemHandle?.()
        if (entry) {
          // Get the path from the dropped item
          // In Tauri context, we need to use the file object differently
          const file = item.getAsFile()
          if (file) {
            // For Tauri desktop apps, we can get path from the file object
            const path = (file as any).path
            if (path) {
              try {
                await openProject(path)
              } catch (err) {
                console.error('Failed to open project:', err)
              }
            }
          }
        }
      }
    }
  }

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
  }

  return (
    <div className="min-h-screen w-full bg-knot-bg flex flex-col items-center justify-center relative overflow-hidden">
      {/* Background gradient */}
      <div 
        className="absolute inset-0 pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse at center, rgba(0, 212, 255, 0.05) 0%, transparent 70%)'
        }}
      />
      
      <div className="relative z-10 text-center px-6">
        {/* Logo */}
        <div className="mb-10">
          <h1 
            className="text-5xl text-white mb-3 tracking-tight"
            style={{ 
              fontFamily: "'Syne', sans-serif", 
              fontWeight: 800,
              fontSize: '64px'
            }}
          >
            Knot
          </h1>
          
          <p 
            className="text-knot-muted mb-1"
            style={{ fontSize: '16px' }}
          >
            Local-first codebase intelligence
          </p>
          
          <p 
            className="text-knot-cyan/50 text-xs mt-4"
            style={{ fontFamily: "'JetBrains Mono', monospace" }}
          >
            v0.1.0
          </p>
        </div>

        {/* Drop zone */}
        <div
          onDragEnter={() => setIsDragging(true)}
          onDragLeave={() => setIsDragging(false)}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
          className="transition-all duration-200"
          style={{
            width: '400px',
            height: '200px',
            borderRadius: '8px',
            border: `2px dashed ${isDragging ? '#00d4ff' : '#1e2433'}`,
            backgroundColor: isDragging ? 'rgba(0, 212, 255, 0.06)' : '#0d1117',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '16px',
          }}
        >
          <FolderOpen 
            className="transition-colors"
            style={{
              width: '48px',
              height: '48px',
              color: isDragging ? '#00d4ff' : '#6b7280',
            }}
          />
          <div className="text-center">
            <p 
              className="text-white mb-1"
              style={{ fontFamily: "'Syne', sans-serif", fontWeight: 600, fontSize: '16px' }}
            >
              Drop a folder to open
            </p>
            <p 
              className="text-knot-muted"
              style={{ fontSize: '13px' }}
            >
              or{' '}
              <button 
                onClick={handleBrowse}
                disabled={isLoading}
                className="text-knot-cyan hover:underline transition-colors disabled:opacity-50"
                style={{ fontWeight: 500 }}
              >
                {isLoading ? 'Opening...' : 'browse'}
              </button>
            </p>
          </div>
        </div>

        {/* Feature pills */}
        <div 
          className="mt-12 flex items-center justify-center gap-3"
        >
          <div 
            className="flex items-center gap-2 px-3 py-1.5 rounded-full border"
            style={{ 
              borderColor: '#1e2433',
              backgroundColor: 'rgba(13, 17, 23, 0.8)',
            }}
          >
            <Sparkles className="w-3.5 h-3.5 text-knot-cyan" />
            <span style={{ fontSize: '11px', color: '#9ca3af' }}>AI-native context</span>
          </div>
          <div 
            className="flex items-center gap-2 px-3 py-1.5 rounded-full border"
            style={{ 
              borderColor: '#1e2433',
              backgroundColor: 'rgba(13, 17, 23, 0.8)',
            }}
          >
            <CloudOff className="w-3.5 h-3.5 text-knot-emerald" />
            <span style={{ fontSize: '11px', color: '#9ca3af' }}>Zero cloud</span>
          </div>
          <div 
            className="flex items-center gap-2 px-3 py-1.5 rounded-full border"
            style={{ 
              borderColor: '#1e2433',
              backgroundColor: 'rgba(13, 17, 23, 0.8)',
            }}
          >
            <Bot className="w-3.5 h-3.5 text-knot-purple" />
            <span style={{ fontSize: '11px', color: '#9ca3af' }}>MCP compatible</span>
          </div>
        </div>
      </div>

      {/* Version badge */}
      <div 
        className="absolute bottom-6 right-6 px-2 py-1 rounded border"
        style={{ 
          borderColor: '#1e2433',
          backgroundColor: 'rgba(13, 17, 23, 0.8)',
          fontFamily: "'JetBrains Mono', monospace",
          fontSize: '10px',
          color: '#6b7280',
        }}
      >
        v0.1.0
      </div>
    </div>
  )
}
