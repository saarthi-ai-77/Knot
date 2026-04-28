import { useState, useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../../stores/projectStore'
import { Folder, Code, Bot, Info, ChevronRight, Copy, Check, Terminal, ChevronDown, ChevronUp } from 'lucide-react'

export function SettingsTab() {
  const { currentProject } = useProjectStore()
  const [watchFiles, setWatchFiles] = useState(true)
  const [ignoredPatterns, setIgnoredPatterns] = useState('node_modules,.git,dist,build')
  const [defaultAgent, setDefaultAgent] = useState('claude-code')
  const [copied, setCopied] = useState(false)
  const [showHowTo, setShowHowTo] = useState(false)
  const [aiProvider, setAiProvider] = useState('none')
  const [aiModel, setAiModel] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [showApiKey, setShowApiKey] = useState(false)
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434')
  const [enrichmentTrigger, setEnrichmentTrigger] = useState('scan')
  const [ollamaStatus, setOllamaStatus] = useState<'checking' | 'available' | 'unavailable'>('checking')
  const [ollamaModels, setOllamaModels] = useState(0)

  const agentOptions = [
    { value: 'claude-code', label: 'Claude Code' },
    { value: 'cursor', label: 'Cursor' },
    { value: 'opencode', label: 'OpenCode' },
    { value: 'copilot', label: 'GitHub Copilot' },
  ]

  // Get MCP config
  const { data: mcpConfig } = useQuery({
    queryKey: ['mcpConfig', currentProject?.id],
    queryFn: async () => {
      if (!currentProject) return null
      return invoke<string>('get_mcp_config', { projectId: currentProject.id })
    },
    enabled: !!currentProject,
  })

  // Detect Ollama on mount
  useEffect(() => {
    const detectOllama = async () => {
      try {
        const result = await invoke<{ available: boolean; models: number }>('detect_ollama')
        if (result.available) {
          setOllamaStatus('available')
          setOllamaModels(result.models)
        } else {
          setOllamaStatus('unavailable')
        }
      } catch {
        setOllamaStatus('unavailable')
      }
    }
    detectOllama()
  }, [])

  const handleCopyConfig = async () => {
    if (mcpConfig) {
      await navigator.clipboard.writeText(mcpConfig)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const handleSaveAiSettings = async () => {
    try {
      await invoke('save_ai_settings', {
        settings: {
          provider: aiProvider,
          model: aiModel,
          apiKey,
          ollamaUrl,
          enrichmentTrigger,
        }
      })
      // Show success feedback
    } catch (error) {
      console.error('Failed to save settings:', error)
    }
  }

  const handleTestConnection = async () => {
    try {
      await invoke('test_ai_connection', { 
        provider: aiProvider,
        model: aiModel,
        apiKey,
        ollamaUrl,
      })
      // Show success
    } catch (error) {
      console.error('Connection test failed:', error)
    }
  }

  const getModelOptions = () => {
    switch (aiProvider) {
      case 'anthropic':
        return [
          { value: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5' },
          { value: 'claude-sonnet-4-20250514', label: 'Claude Sonnet 4' },
        ]
      case 'openai':
        return [
          { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
          { value: 'gpt-4o', label: 'GPT-4o' },
        ]
      case 'google':
        return [
          { value: 'gemini-2.0-flash', label: 'Gemini 2.0 Flash' },
          { value: 'gemini-1.5-pro', label: 'Gemini 1.5 Pro' },
        ]
      case 'ollama':
        return [] // Free text input
      default:
        return []
    }
  }

  const getApiKeyPlaceholder = () => {
    switch (aiProvider) {
      case 'anthropic':
        return 'sk-ant-...'
      case 'openai':
        return 'sk-...'
      case 'google':
        return 'AIza...'
      default:
        return ''
    }
  }

  return (
    <div className="p-6 max-w-3xl mx-auto">
      <h2 className="font-heading font-bold text-xl text-knot-text-bright mb-6">
        Settings
      </h2>

      {/* Project Section */}
      <section className="mb-8">
        <div className="flex items-center gap-2 mb-4">
          <Folder className="w-4 h-4 text-knot-cyan" />
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright uppercase tracking-wider">
            Project
          </h3>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4 space-y-4">
          <div>
            <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
              Current Project Path
            </label>
            <input
              type="text"
              value={currentProject?.root_path || ''}
              readOnly
              className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm font-mono text-knot-text-bright"
            />
          </div>
          <button className="bg-knot-cyan text-knot-bg font-heading font-semibold text-sm py-2 px-4 rounded hover:bg-knot-cyan/90 transition-knot">
            Change Project
          </button>
        </div>
      </section>

      <div className="h-px bg-knot-border mb-8" />

      {/* MCP Configuration Section */}
      <section className="mb-8">
        <div className="flex items-center gap-2 mb-4">
          <Terminal className="w-4 h-4 text-knot-cyan" />
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright uppercase tracking-wider">
            Connect to Agents
          </h3>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4 space-y-4">
          <p className="text-sm text-knot-muted">
            Add this config to your Claude Code / OpenCode / Cursor settings to enable MCP integration.
          </p>

          {mcpConfig ? (
            <>
              <div className="relative">
                <pre className="bg-knot-bg border border-knot-border rounded p-3 text-xs font-mono text-knot-text-bright overflow-x-auto">
                  {mcpConfig}
                </pre>
                <button
                  onClick={handleCopyConfig}
                  className="absolute top-2 right-2 p-1.5 rounded bg-knot-panel hover:bg-knot-panel-hover transition-knot"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-knot-emerald" />
                  ) : (
                    <Copy className="w-4 h-4 text-knot-muted" />
                  )}
                </button>
              </div>

              <button
                onClick={() => setShowHowTo(!showHowTo)}
                className="flex items-center gap-2 text-sm text-knot-cyan hover:text-knot-cyan/80 transition-knot"
              >
                {showHowTo ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                How to connect
              </button>

              {showHowTo && (
                <div className="space-y-2 text-sm text-knot-muted">
                  <p><strong>Claude Code:</strong> Add to ~/.claude/settings.json under mcpServers</p>
                  <p><strong>OpenCode:</strong> Add to opencode.json under mcpServers</p>
                  <p><strong>Cursor:</strong> Add to .cursor/mcp.json under mcpServers</p>
                </div>
              )}
            </>
          ) : (
            <p className="text-sm text-knot-muted">Open a project to generate MCP config</p>
          )}
        </div>
      </section>

      <div className="h-px bg-knot-border mb-8" />

      {/* AI Enrichment Section */}
      <section className="mb-8">
        <div className="flex items-center gap-2 mb-4">
          <Bot className="w-4 h-4 text-knot-cyan" />
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright uppercase tracking-wider">
            AI Enrichment
          </h3>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4 space-y-4">
          {/* Ollama detection banner */}
          {ollamaStatus === 'available' && (
            <div className="p-3 bg-knot-emerald/10 border border-knot-emerald/30 rounded">
              <p className="text-sm text-knot-emerald">
                Ollama detected: {ollamaModels} models available
              </p>
            </div>
          )}
          {ollamaStatus === 'unavailable' && (
            <p className="text-xs text-knot-muted">Ollama not detected on localhost:11434</p>
          )}

          {/* Provider dropdown */}
          <div>
            <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
              Provider
            </label>
            <select
              value={aiProvider}
              onChange={(e) => setAiProvider(e.target.value)}
              className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm text-knot-text-bright focus:outline-none focus:border-knot-cyan"
            >
              <option value="none">None</option>
              <option value="anthropic">Anthropic</option>
              <option value="openai">OpenAI</option>
              <option value="google">Google Gemini</option>
              <option value="ollama">Ollama</option>
            </select>
          </div>

          {/* Model selector */}
          {aiProvider !== 'none' && aiProvider !== 'ollama' && (
            <div>
              <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
                Model
              </label>
              <select
                value={aiModel}
                onChange={(e) => setAiModel(e.target.value)}
                className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm text-knot-text-bright focus:outline-none focus:border-knot-cyan"
              >
                <option value="">Select a model</option>
                {getModelOptions().map((opt) => (
                  <option key={opt.value} value={opt.value}>{opt.label}</option>
                ))}
              </select>
            </div>
          )}

          {/* Ollama model input */}
          {aiProvider === 'ollama' && (
            <div>
              <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
                Model Name
              </label>
              <input
                type="text"
                value={aiModel}
                onChange={(e) => setAiModel(e.target.value)}
                placeholder="qwen2.5-coder:7b"
                className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm font-mono text-knot-text-bright focus:outline-none focus:border-knot-cyan"
              />
            </div>
          )}

          {/* API Key input */}
          {aiProvider !== 'none' && aiProvider !== 'ollama' && (
            <div>
              <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
                API Key
              </label>
              <div className="relative">
                <input
                  type={showApiKey ? 'text' : 'password'}
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={getApiKeyPlaceholder()}
                  className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 pr-10 text-sm font-mono text-knot-text-bright placeholder:text-knot-muted focus:outline-none focus:border-knot-cyan"
                />
                <button
                  onClick={() => setShowApiKey(!showApiKey)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-knot-muted hover:text-knot-text"
                >
                  {showApiKey ? 'Hide' : 'Show'}
                </button>
              </div>
            </div>
          )}

          {/* Ollama URL */}
          {aiProvider === 'ollama' && (
            <div>
              <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
                Ollama URL
              </label>
              <input
                type="text"
                value={ollamaUrl}
                onChange={(e) => setOllamaUrl(e.target.value)}
                placeholder="http://localhost:11434"
                className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm font-mono text-knot-text-bright focus:outline-none focus:border-knot-cyan"
              />
            </div>
          )}

          {/* Enrichment trigger */}
          <div>
            <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
              Enrichment Trigger
            </label>
            <select
              value={enrichmentTrigger}
              onChange={(e) => setEnrichmentTrigger(e.target.value)}
              className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm text-knot-text-bright focus:outline-none focus:border-knot-cyan"
            >
              <option value="scan">On scan complete</option>
              <option value="manual">Manual only</option>
            </select>
          </div>

          {/* Action buttons */}
          <div className="flex gap-3">
            <button
              onClick={handleSaveAiSettings}
              className="bg-knot-cyan text-knot-bg font-heading font-semibold text-sm py-2 px-4 rounded hover:bg-knot-cyan/90 transition-knot"
            >
              Save Settings
            </button>
            {aiProvider !== 'none' && (
              <button
                onClick={handleTestConnection}
                className="bg-knot-panel border border-knot-border text-knot-text font-semibold text-sm py-2 px-4 rounded hover:bg-knot-panel-hover transition-knot"
              >
                Test Connection
              </button>
            )}
          </div>
        </div>
      </section>

      <div className="h-px bg-knot-border mb-8" />

      {/* Parser Section */}
      <section className="mb-8">
        <div className="flex items-center gap-2 mb-4">
          <Code className="w-4 h-4 text-knot-cyan" />
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright uppercase tracking-wider">
            Parser
          </h3>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4 space-y-4">
          {/* Watch for file changes */}
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm text-knot-text-bright">Watch for file changes</div>
              <div className="text-xs text-knot-muted">Automatically re-scan files when they change</div>
            </div>
            <button
              onClick={() => setWatchFiles(!watchFiles)}
              className={`w-11 h-6 rounded-full transition-knot ${
                watchFiles ? 'bg-knot-cyan' : 'bg-knot-border'
              }`}
            >
              <div className={`w-5 h-5 rounded-full bg-knot-text-bright transition-knot transform ${
                watchFiles ? 'translate-x-5' : 'translate-x-0.5'
              }`} />
            </button>
          </div>

          {/* Ignored patterns */}
          <div>
            <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
              Ignored Patterns
            </label>
            <textarea
              value={ignoredPatterns}
              onChange={(e) => setIgnoredPatterns(e.target.value)}
              placeholder="node_modules,.git,dist"
              className="w-full h-20 bg-knot-bg border border-knot-border rounded p-3 text-sm font-mono text-knot-text-bright placeholder:text-knot-muted focus:outline-none focus:border-knot-cyan resize-none"
            />
            <p className="text-xs text-knot-muted mt-1">
              Comma-separated list of patterns to ignore
            </p>
          </div>
        </div>
      </section>

      <div className="h-px bg-knot-border mb-8" />

      {/* Agents Section */}
      <section className="mb-8">
        <div className="flex items-center gap-2 mb-4">
          <Bot className="w-4 h-4 text-knot-cyan" />
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright uppercase tracking-wider">
            Default Agent
          </h3>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4">
          <div>
            <label className="text-[10px] uppercase tracking-wider text-knot-muted mb-1 block">
              Default Agent Type
            </label>
            <select
              value={defaultAgent}
              onChange={(e) => setDefaultAgent(e.target.value)}
              className="w-full bg-knot-bg border border-knot-border rounded px-3 py-2 text-sm text-knot-text-bright focus:outline-none focus:border-knot-cyan"
            >
              {agentOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
        </div>
      </section>

      <div className="h-px bg-knot-border mb-8" />

      {/* About Section */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Info className="w-4 h-4 text-knot-cyan" />
          <h3 className="font-heading font-semibold text-sm text-knot-text-bright uppercase tracking-wider">
            About
          </h3>
        </div>
        <div className="bg-knot-panel border border-knot-border p-4">
          <div className="flex items-center justify-between py-2">
            <span className="text-sm text-knot-muted">Version</span>
            <span className="text-sm text-knot-text-bright font-mono">0.1.0</span>
          </div>
          <div className="h-px bg-knot-border my-2" />
          <div className="flex items-center justify-between py-2">
            <span className="text-sm text-knot-muted">Description</span>
            <span className="text-sm text-knot-text-bright">
              Local-first codebase intelligence
            </span>
          </div>
          <div className="h-px bg-knot-border my-2" />
          <a
            href="https://github.com/yourusername/knot"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center justify-between py-2 text-sm text-knot-cyan hover:text-knot-cyan/80 transition-knot"
          >
            <span>View on GitHub</span>
            <ChevronRight className="w-4 h-4" />
          </a>
        </div>
      </section>
    </div>
  )
}
