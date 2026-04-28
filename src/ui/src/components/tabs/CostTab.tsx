import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../../stores/projectStore'
import { 
  LineChart, 
  Line, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer 
} from 'recharts'

interface CostSummary {
  total_tokens: number
  estimated_cost_usd: number
  session_count: number
}

interface CostLogRow {
  id: string
  operation: string
  provider: string
  model: string
  input_tokens: number
  output_tokens: number
  cost_usd: number
  task_type: string
  timestamp: number
}

// Generate mock data for 7 days since we don't have historical data
function generateMockChartData() {
  const data = []
  const today = new Date()
  for (let i = 6; i >= 0; i--) {
    const date = new Date(today)
    date.setDate(date.getDate() - i)
    data.push({
      date: date.toLocaleDateString('en-US', { weekday: 'short' }),
      tokens: Math.floor(Math.random() * 50000) + 10000,
    })
  }
  return data
}

export function CostTab() {
  const { currentProject } = useProjectStore()

  const { data: summary } = useQuery({
    queryKey: ['costSummary', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return null
      return invoke<CostSummary>('get_cost_summary', { 
        projectId: currentProject.id 
      })
    },
    enabled: !!currentProject,
  })

  const { data: costLog } = useQuery({
    queryKey: ['costLog', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return []
      return invoke<CostLogRow[]>('get_cost_log', { 
        projectId: currentProject.id,
        limit: 50
      })
    },
    enabled: !!currentProject,
  })

  const chartData = generateMockChartData()

  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 4,
    }).format(amount)
  }

  const formatTokens = (tokens: number) => {
    if (tokens >= 1000000) return `${(tokens / 1000000).toFixed(2)}M`
    if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`
    return tokens.toString()
  }

  const formatTimestamp = (timestamp: number) => {
    const date = new Date(timestamp * 1000)
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  return (
    <div className="p-6 max-w-6xl mx-auto">
      {/* Summary Row */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        <div className="bg-knot-panel border border-knot-border p-4">
          <div className="text-[10px] uppercase tracking-wider text-knot-muted mb-1">
            Total Tokens Used
          </div>
          <div className="font-mono text-2xl font-bold text-knot-cyan">
            {formatTokens(summary?.total_tokens ?? 0)}
          </div>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4">
          <div className="text-[10px] uppercase tracking-wider text-knot-muted mb-1">
            Estimated Cost
          </div>
          <div className="font-mono text-2xl font-bold text-knot-emerald">
            {formatCurrency(summary?.estimated_cost_usd ?? 0)}
          </div>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4">
          <div className="text-[10px] uppercase tracking-wider text-knot-muted mb-1">
            Sessions
          </div>
          <div className="font-mono text-2xl font-bold text-knot-text-bright">
            {summary?.session_count ?? 0}
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="bg-knot-panel border border-knot-border p-4 mb-8">
        <h3 className="font-heading font-semibold text-sm text-knot-text-bright mb-4">
          Token Usage (7 Days)
        </h3>
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e2433" />
              <XAxis 
                dataKey="date" 
                stroke="#6b7280"
                fontSize={12}
                tickLine={false}
              />
              <YAxis 
                stroke="#6b7280"
                fontSize={12}
                tickLine={false}
                tickFormatter={(value) => formatTokens(value)}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#0d1117',
                  border: '1px solid #1e2433',
                  borderRadius: '4px',
                }}
                labelStyle={{ color: '#9ca3af' }}
                itemStyle={{ color: '#00d4ff', fontFamily: 'JetBrains Mono' }}
                formatter={(value: number) => [formatTokens(value), 'Tokens']}
              />
              <Line 
                type="monotone" 
                dataKey="tokens" 
                stroke="#00d4ff" 
                strokeWidth={2}
                dot={{ fill: '#00d4ff', strokeWidth: 0, r: 4 }}
                activeDot={{ r: 6, fill: '#00d4ff' }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Cost Log Table */}
      <div className="bg-knot-panel border border-knot-border">
        <div className="px-4 py-3 border-b border-knot-border">
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright">
            Cost Log
          </h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-knot-border text-left">
                <th className="px-4 py-2 text-[10px] uppercase tracking-wider text-knot-muted font-medium">
                  Agent
                </th>
                <th className="px-4 py-2 text-[10px] uppercase tracking-wider text-knot-muted font-medium">
                  Model
                </th>
                <th className="px-4 py-2 text-[10px] uppercase tracking-wider text-knot-muted font-medium text-right">
                  In
                </th>
                <th className="px-4 py-2 text-[10px] uppercase tracking-wider text-knot-muted font-medium text-right">
                  Out
                </th>
                <th className="px-4 py-2 text-[10px] uppercase tracking-wider text-knot-muted font-medium text-right">
                  Cost
                </th>
                <th className="px-4 py-2 text-[10px] uppercase tracking-wider text-knot-muted font-medium">
                  Timestamp
                </th>
              </tr>
            </thead>
            <tbody>
              {costLog?.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-knot-muted">
                    No cost entries yet
                  </td>
                </tr>
              ) : (
                costLog?.map((log, idx) => (
                  <tr 
                    key={log.id}
                    className={`border-b border-knot-border last:border-0 hover:bg-knot-panel-hover transition-knot ${
                      idx % 2 === 0 ? 'bg-knot-panel' : 'bg-knot-bg'
                    }`}
                  >
                    <td className="px-4 py-3 font-mono">
                      {log.operation}
                    </td>
                    <td className="px-4 py-3">
                      <span className="text-knot-muted">{log.provider}</span>
                      <span className="mx-1">/</span>
                      <span>{log.model}</span>
                    </td>
                    <td className="px-4 py-3 text-right font-mono">
                      {formatTokens(log.input_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono">
                      {formatTokens(log.output_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-knot-emerald">
                      {formatCurrency(log.cost_usd)}
                    </td>
                    <td className="px-4 py-3 text-knot-muted">
                      {formatTimestamp(log.timestamp)}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
