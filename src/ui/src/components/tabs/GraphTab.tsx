import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../../stores/projectStore'
import { Search, FileCode, ExternalLink } from 'lucide-react'

interface EntitySummary {
  id: string
  name: string
  kind: string
  file_path: string
  line_start: number
}

interface EntityDetailSummary {
  id: string
  name: string
  kind: string
  file_path: string
  signature: string
}

interface EntityEvent {
  event_type: string
  timestamp: number
  old_value: string | null
  new_value: string | null
}

interface EntityDecision {
  id: string
  title: string
  rationale: string
}

interface EntityDetail {
  entity: EntityDetailSummary
  imports: EntityDetailSummary[]
  imported_by: EntityDetailSummary[]
  calls: EntityDetailSummary[]
  called_by: EntityDetailSummary[]
  recent_events: EntityEvent[]
  decisions: EntityDecision[]
}

const kindFilters = [
  { id: null, label: 'All' },
  { id: 'function', label: 'Functions' },
  { id: 'class', label: 'Classes' },
  { id: 'interface', label: 'Interfaces' },
  { id: 'import', label: 'Imports' },
]

export function GraphTab() {
  const { currentProject } = useProjectStore()
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedKind, setSelectedKind] = useState<string | null>(null)
  const [selectedEntity, setSelectedEntity] = useState<string | null>(null)

  // Query entities
  const { data: entities, isLoading: isLoadingEntities } = useQuery({
    queryKey: ['entities', currentProject?.id, searchQuery, selectedKind],
    queryFn: async () => {
      if (!currentProject) return []
      return invoke<EntitySummary[]>('query_entities', {
        projectId: currentProject.id,
        query: searchQuery,
        kindFilter: selectedKind,
      })
    },
    enabled: !!currentProject,
    debounce: 300,
  })

  // Query entity detail
  const { data: entityDetail, isLoading: isLoadingDetail } = useQuery({
    queryKey: ['entityDetail', selectedEntity],
    queryFn: async () => {
      if (!selectedEntity) return null
      return invoke<EntityDetail>('get_entity_detail', {
        entityId: selectedEntity,
      })
    },
    enabled: !!selectedEntity,
  })

  const getKindBadgeClass = (kind: string) => {
    switch (kind) {
      case 'function': return 'badge-function'
      case 'class': return 'badge-class'
      case 'interface': return 'badge-interface'
      default: return 'badge-import'
    }
  }

  const getEventColor = (type: string) => {
    switch (type) {
      case 'entity_added': return 'bg-knot-emerald'
      case 'entity_removed': return 'bg-knot-red'
      case 'signature_changed': return 'bg-knot-amber'
      case 'file_modified': return 'bg-knot-muted'
      default: return 'bg-knot-muted'
    }
  }

  const getRelativeTime = (timestamp: number) => {
    const seconds = Math.floor((Date.now() / 1000 - timestamp))
    if (seconds < 60) return 'just now'
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`
    return `${Math.floor(seconds / 86400)}d ago`
  }

  const handleEntityClick = (entityId: string) => {
    setSelectedEntity(entityId)
  }

  const RelationshipSection = ({ 
    title, 
    entities, 
    emptyText 
  }: { 
    title: string
    entities: EntityDetailSummary[]
    emptyText: string
  }) => (
    <div className="mb-4">
      <h4 className="text-[10px] uppercase tracking-wider text-knot-muted mb-2">
        {title}
      </h4>
      {entities.length === 0 ? (
        <span className="text-xs text-knot-muted">{emptyText}</span>
      ) : (
        <div className="flex flex-wrap gap-2">
          {entities.map((e) => (
            <button
              key={e.id}
              onClick={() => handleEntityClick(e.id)}
              className="px-2 py-1 text-xs font-mono rounded bg-knot-border text-knot-text hover:bg-knot-cyan/20 hover:text-knot-cyan transition-knot"
            >
              {e.name}
            </button>
          ))}
        </div>
      )}
    </div>
  )

  return (
    <div className="flex h-full">
      {/* Left Panel - Entity Explorer */}
      <div className="w-80 bg-knot-panel border-r border-knot-border flex flex-col">
        {/* Search */}
        <div className="p-3 border-b border-knot-border">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-knot-muted" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search entities..."
              className="w-full bg-knot-bg border border-knot-border rounded pl-9 pr-3 py-2 text-sm font-mono text-knot-text-bright placeholder:text-knot-muted focus:outline-none focus:border-knot-cyan"
            />
          </div>
        </div>

        {/* Kind Filters */}
        <div className="px-3 py-2 border-b border-knot-border">
          <div className="flex flex-wrap gap-1">
            {kindFilters.map((filter) => (
              <button
                key={filter.label}
                onClick={() => setSelectedKind(filter.id)}
                className={`
                  px-2 py-1 text-[10px] uppercase tracking-wider rounded transition-knot
                  ${selectedKind === filter.id 
                    ? 'bg-knot-cyan text-knot-bg' 
                    : 'bg-knot-bg text-knot-muted hover:text-knot-text border border-knot-border'
                  }
                `}
              >
                {filter.label}
              </button>
            ))}
          </div>
        </div>

        {/* Results List */}
        <div className="flex-1 overflow-auto">
          {isLoadingEntities ? (
            <div className="p-4 text-center text-knot-muted text-sm">
              Loading...
            </div>
          ) : entities?.length === 0 ? (
            <div className="p-4 text-center text-knot-muted text-sm">
              No entities found
            </div>
          ) : (
            <div className="divide-y divide-knot-border">
              {entities?.map((entity) => (
                <button
                  key={entity.id}
                  onClick={() => setSelectedEntity(entity.id)}
                  className={`
                    w-full px-3 py-3 text-left hover:bg-knot-panel-hover transition-knot
                    ${selectedEntity === entity.id ? 'bg-knot-cyan-dim border-l-2 border-knot-cyan' : 'border-l-2 border-transparent'}
                  `}
                >
                  <div className="flex items-start justify-between gap-2">
                    <span className="font-mono text-sm text-knot-text-bright truncate">
                      {entity.name}
                    </span>
                    <span className={`text-[10px] px-1.5 py-0.5 rounded border shrink-0 ${getKindBadgeClass(entity.kind)}`}>
                      {entity.kind}
                    </span>
                  </div>
                  <div className="text-[10px] text-knot-muted font-mono truncate mt-1">
                    {entity.file_path.split('/').pop()}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Right Panel - Entity Detail */}
      <div className="flex-1 overflow-auto">
        {selectedEntity && entityDetail ? (
          <div className="p-6 max-w-3xl">
            {/* Header */}
            <div className="mb-6">
              <div className="flex items-center gap-3 mb-2">
                <h2 className="font-mono text-2xl font-bold text-knot-cyan">
                  {entityDetail.entity.name}
                </h2>
                <span className={`text-xs px-2 py-1 rounded border ${getKindBadgeClass(entityDetail.entity.kind)}`}>
                  {entityDetail.entity.kind}
                </span>
              </div>
              <div className="flex items-center gap-2 text-sm text-knot-muted">
                <FileCode className="w-4 h-4" />
                <span className="font-mono">{entityDetail.entity.file_path}</span>
                <button 
                  className="text-knot-cyan hover:text-knot-cyan/80 ml-2"
                  title="Open in VS Code"
                >
                  <ExternalLink className="w-3 h-3" />
                </button>
              </div>
            </div>

            {/* Signature Block */}
            {entityDetail.entity.signature && (
              <div className="mb-6">
                <h3 className="text-[10px] uppercase tracking-wider text-knot-muted mb-2">
                  Signature
                </h3>
                <div className="code-block">
                  <pre className="text-xs text-knot-text-bright">{entityDetail.entity.signature}</pre>
                </div>
              </div>
            )}

            {/* Relationships */}
            <div className="mb-6">
              <h3 className="text-[10px] uppercase tracking-wider text-knot-muted mb-3">
                Relationships
              </h3>
              
              <RelationshipSection 
                title="Imports" 
                entities={entityDetail.imports}
                emptyText="None"
              />
              
              <RelationshipSection 
                title="Imported by" 
                entities={entityDetail.imported_by}
                emptyText="None"
              />
              
              <RelationshipSection 
                title="Calls" 
                entities={entityDetail.calls}
                emptyText="None"
              />
              
              <RelationshipSection 
                title="Called by" 
                entities={entityDetail.called_by}
                emptyText="None"
              />
            </div>

            {/* Recent Changes */}
            {entityDetail.recent_events.length > 0 && (
              <div className="mb-6">
                <h3 className="text-[10px] uppercase tracking-wider text-knot-muted mb-3">
                  Recent Changes
                </h3>
                <div className="space-y-2">
                  {entityDetail.recent_events.map((event, idx) => (
                    <div 
                      key={idx}
                      className="flex items-center gap-3 text-sm py-2 border-b border-knot-border last:border-0"
                    >
                      <span className={`w-2 h-2 rounded-full ${getEventColor(event.event_type)}`} />
                      <span className="text-knot-text-bright">{event.event_type}</span>
                      <span className="text-knot-muted">{getRelativeTime(event.timestamp)}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Decisions */}
            {entityDetail.decisions.length > 0 && (
              <div>
                <h3 className="text-[10px] uppercase tracking-wider text-knot-muted mb-3">
                  Linked Decisions
                </h3>
                <div className="space-y-2">
                  {entityDetail.decisions.map((decision) => (
                    <div 
                      key={decision.id}
                      className="p-3 bg-knot-bg border border-knot-border rounded"
                    >
                      <span className="text-sm text-knot-text-bright">{decision.title}</span>
                      <p className="text-xs text-knot-muted mt-1">{decision.rationale}</p>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          /* Empty State */
          <div className="h-full flex flex-col items-center justify-center text-knot-muted">
            {/* Node graph SVG illustration */}
            <svg 
              className="w-24 h-24 mb-4 opacity-20" 
              viewBox="0 0 100 100"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            >
              <circle cx="50" cy="50" r="20" stroke="#00d4ff" />
              <circle cx="20" cy="30" r="8" stroke="#6b7280" />
              <circle cx="80" cy="30" r="8" stroke="#6b7280" />
              <circle cx="20" cy="70" r="8" stroke="#6b7280" />
              <circle cx="80" cy="70" r="8" stroke="#6b7280" />
              <line x1="50" y1="50" x2="20" y2="30" stroke="#1e2433" />
              <line x1="50" y1="50" x2="80" y2="30" stroke="#1e2433" />
              <line x1="50" y1="50" x2="20" y2="70" stroke="#1e2433" />
              <line x1="50" y1="50" x2="80" y2="70" stroke="#1e2433" />
            </svg>
            <p 
              className="text-sm mb-1"
              style={{ fontFamily: "'Syne', sans-serif", fontWeight: 600 }}
            >
              Select an entity to explore
            </p>
            <p className="text-xs text-knot-muted">
              Search for functions, classes, or modules on the left
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
