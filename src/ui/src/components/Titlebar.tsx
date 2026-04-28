import { useProjectStore } from '../stores/projectStore'
import { ChevronDown, FolderGit2 } from 'lucide-react'

export function Titlebar() {
  const { currentProject } = useProjectStore()

  return (
    <div 
      className="h-8 flex items-center justify-between px-4 select-none shrink-0"
      style={{
        backgroundColor: '#0d1117',
        borderBottom: '1px solid #1e2433',
      }}
      data-tauri-drag-region
    >
      {/* Left - Logo */}
      <div className="flex items-center gap-3">
        <span 
          className="font-bold text-sm tracking-wide"
          style={{ 
            fontFamily: "'Syne', sans-serif",
            color: '#e5e7eb',
          }}
        >
          Knot
        </span>
        <div 
          className="w-px h-4"
          style={{ backgroundColor: '#1e2433' }}
        />
        
        {/* Project Selector */}
        {currentProject && (
          <div className="flex items-center gap-2 group cursor-pointer">
            <FolderGit2 style={{ width: '14px', height: '14px', color: '#00d4ff' }} />
            <span 
              className="text-xs"
              style={{ 
                fontFamily: "'JetBrains Mono', monospace",
                color: '#e5e7eb',
              }}
            >
              {currentProject.name}
            </span>
            <ChevronDown style={{ width: '12px', height: '12px', color: '#6b7280' }} />
          </div>
        )}
      </div>

      {/* Center - Window controls spacer */}
      <div className="flex-1" data-tauri-drag-region />

      {/* Right - Actions */}
      <div className="flex items-center gap-2">
        <button 
          className="w-6 h-6 flex items-center justify-center rounded transition-colors"
          style={{ color: '#6b7280' }}
          onMouseEnter={(e) => e.currentTarget.style.backgroundColor = '#161b22'}
          onMouseLeave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
        >
          <span className="text-xs">···</span>
        </button>
      </div>
    </div>
  )
}
