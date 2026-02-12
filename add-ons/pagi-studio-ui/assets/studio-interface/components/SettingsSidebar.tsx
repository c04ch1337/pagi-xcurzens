import React, { useState, useMemo, useEffect } from 'react';
import { X, Server, Zap, Eye, Upload, Trash2, Image as ImageIcon, Palette, UserCircle, Sun, Moon, Brain, Bot, Sliders, MessageSquare, Terminal, Search, Filter, Calendar, History, Settings2, AlertCircle, Database, CheckCircle2, XCircle, RefreshCw, Folder, FolderOpen, Network, Shield, Plus, Key } from 'lucide-react';
import { AppSettings, GatewayFeatureConfig, Message } from '../types';
import { API_BASE_URL } from '../src/api/config';
import { getVaultProtectedTerms, postVaultProtectedTerms, postVaultRedactTest, getVaultStatus } from '../services/apiService';
import LogTerminal from './LogTerminal';
import SovereignPerimeter from './SovereignPerimeter';

interface KbStatusItem {
  slot_id: number;
  name: string;
  tree_name: string;
  connected: boolean;
  entry_count: number;
  error: string | null;
}

interface KbStatusResponse {
  status: string;
  all_connected: boolean;
  total_entries: number;
  knowledge_bases: KbStatusItem[];
}

interface SettingsSidebarProps {
  isOpen: boolean;
  onClose: () => void;
  settings: AppSettings;
  setSettings: React.Dispatch<React.SetStateAction<AppSettings>>;
  messages: Message[];
  onClearChat: () => void;
  /** Gateway config (from App) for MoE toggle and status; optional for backwards compat. */
  gatewayConfig?: GatewayFeatureConfig | null;
  /** After toggling MoE, refetch config so App and status line stay in sync. */
  onConfigRefetch?: () => void | Promise<void>;
  /** Active project id (current chat thread) for Perimeter scope default. */
  activeProjectId?: string | null;
  /** Optional project id -> display name for Perimeter scope dropdown. */
  projectNames?: Record<string, string>;
}

const SettingsSidebar: React.FC<SettingsSidebarProps> = ({ isOpen, onClose, settings, setSettings, messages, onClearChat, gatewayConfig: gatewayConfigProp, onConfigRefetch, activeProjectId, projectNames }) => {
  const [activeTab, setActiveTab] = useState<'settings' | 'history' | 'perimeter'>('settings');
  const [searchQuery, setSearchQuery] = useState('');
  const [roleFilter, setRoleFilter] = useState<'all' | 'user' | 'agi'>('all');
  const [timeFilter, setTimeFilter] = useState<'all' | '24h' | '7d'>('all');
  const [uploadErrors, setUploadErrors] = useState<Record<string, string>>({});
  const [kbStatus, setKbStatus] = useState<KbStatusResponse | null>(null);
  const [kbLoading, setKbLoading] = useState(false);
  const [kbError, setKbError] = useState<string | null>(null);
  const [localGatewayConfig, setLocalGatewayConfig] = useState<GatewayFeatureConfig | null>(null);
  const [moeToggling, setMoeToggling] = useState(false);
  const [personaToggling, setPersonaToggling] = useState(false);
  const [saveEnvToast, setSaveEnvToast] = useState(false);
  const [protectedTerms, setProtectedTerms] = useState<string[]>([]);
  const [protectedTermsLoading, setProtectedTermsLoading] = useState(false);
  const [protectedTermsSaving, setProtectedTermsSaving] = useState(false);
  const [newTerm, setNewTerm] = useState('');
  const [vaultStatus, setVaultStatus] = useState<{ openrouter_in_vault: boolean; pagi_llm_in_vault: boolean } | null>(null);
  const [testRedactInput, setTestRedactInput] = useState('');
  const [testRedactOutput, setTestRedactOutput] = useState('');
  const [testRedactLoading, setTestRedactLoading] = useState(false);
  const gatewayConfig = gatewayConfigProp ?? localGatewayConfig;

  const orchestratorRole = gatewayConfig?.orchestrator_role ?? gatewayConfig?.persona_mode ?? 'counselor';
  const isCounselor = orchestratorRole === 'counselor';

  const logsUrl = `${API_BASE_URL}/logs`;

  const fetchGatewayConfig = async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/config`);
      if (res.ok) {
        const data = await res.json();
        setLocalGatewayConfig(data as GatewayFeatureConfig);
      }
    } catch {
      setLocalGatewayConfig(null);
    }
  };

  const handleMoeToggle = async () => {
    const next = !(gatewayConfig?.moe_active ?? false);
    setMoeToggling(true);
    try {
      const res = await fetch(`${API_BASE_URL}/settings/moe`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: next }),
      });
      if (res.ok) {
        await onConfigRefetch?.();
        if (!gatewayConfigProp) fetchGatewayConfig();
      }
    } finally {
      setMoeToggling(false);
    }
  };

  const handleOrchestratorRoleSet = async (mode: 'counselor' | 'companion') => {
    if (orchestratorRole === mode) return;
    setPersonaToggling(true);
    try {
      const res = await fetch(`${API_BASE_URL}/settings/orchestrator-role`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ mode }),
      });
      if (res.ok) {
        await onConfigRefetch?.();
        if (!gatewayConfigProp) fetchGatewayConfig();
      }
    } finally {
      setPersonaToggling(false);
    }
  };

  const ZODIAC_SIGNS = ['aries', 'taurus', 'gemini', 'cancer', 'leo', 'virgo', 'libra', 'scorpio', 'sagittarius', 'capricorn', 'aquarius', 'pisces'];

  // Fetch KB status when sidebar opens (Gateway 8001 only)
  const fetchKbStatus = async () => {
    setKbLoading(true);
    setKbError(null);
    try {
      const response = await fetch(`${API_BASE_URL}/kb-status`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data: KbStatusResponse = await response.json();
      setKbStatus(data);
    } catch (err) {
      setKbError(err instanceof Error ? err.message : 'Failed to fetch KB status');
    } finally {
      setKbLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen && activeTab === 'settings') {
      fetchKbStatus();
      fetchGatewayConfig();
    }
  }, [isOpen, activeTab]);

  const fetchProtectedTerms = async () => {
    setProtectedTermsLoading(true);
    try {
      const data = await getVaultProtectedTerms();
      setProtectedTerms(data.terms ?? []);
    } catch {
      setProtectedTerms([]);
    } finally {
      setProtectedTermsLoading(false);
    }
  };

  const fetchVaultStatus = async () => {
    try {
      const data = await getVaultStatus();
      setVaultStatus(data);
    } catch {
      setVaultStatus(null);
    }
  };

  useEffect(() => {
    if (isOpen && activeTab === 'settings') {
      fetchProtectedTerms();
      fetchVaultStatus();
    }
  }, [isOpen, activeTab]);

  const handleAddTerm = async () => {
    const t = newTerm.trim().toUpperCase();
    if (!t) return;
    const next = [...protectedTerms, t];
    setProtectedTermsSaving(true);
    try {
      await postVaultProtectedTerms(next);
      setProtectedTerms(next);
      setNewTerm('');
    } catch (e) {
      console.error('Add term failed:', e);
    } finally {
      setProtectedTermsSaving(false);
    }
  };

  const handleRemoveTerm = async (term: string) => {
    const next = protectedTerms.filter((x) => x !== term);
    setProtectedTermsSaving(true);
    try {
      await postVaultProtectedTerms(next);
      setProtectedTerms(next);
    } catch (e) {
      console.error('Remove term failed:', e);
    } finally {
      setProtectedTermsSaving(false);
    }
  };

  const handleTestRedact = async () => {
    setTestRedactLoading(true);
    setTestRedactOutput('');
    try {
      const data = await postVaultRedactTest(testRedactInput);
      setTestRedactOutput(data.sanitized);
    } catch (e) {
      setTestRedactOutput(`Error: ${e instanceof Error ? e.message : 'Failed'}`);
    } finally {
      setTestRedactLoading(false);
    }
  };

  // Filter messages logic
  const filteredMessages = useMemo(() => {
    if (activeTab !== 'history') return [];
    
    const now = Date.now();
    const oneDay = 24 * 60 * 60 * 1000;
    const sevenDays = 7 * oneDay;

    return messages.filter(msg => {
      // Role Filter
      if (roleFilter !== 'all' && msg.role !== roleFilter) return false;
      
      // Time Filter
      if (timeFilter === '24h' && (now - msg.timestamp) > oneDay) return false;
      if (timeFilter === '7d' && (now - msg.timestamp) > sevenDays) return false;

      // Text Search
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const contentMatch = msg.content.toLowerCase().includes(query);
        const thoughtMatch = msg.thoughts?.some(t => 
            t.title.toLowerCase().includes(query) || t.content.toLowerCase().includes(query)
        );
        return contentMatch || thoughtMatch;
      }

      return true;
    }).reverse(); // Show newest first
  }, [messages, activeTab, searchQuery, roleFilter, timeFilter]);

  if (!isOpen) return null;

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>, key: 'customLogo' | 'customFavicon' | 'userAvatar' | 'agiAvatar') => {
    const file = e.target.files?.[0];
    
    // Reset error for this key
    setUploadErrors(prev => {
        const next = { ...prev };
        delete next[key];
        return next;
    });

    if (file) {
      // Validation: File Type
      if (!file.type.startsWith('image/')) {
        setUploadErrors(prev => ({ ...prev, [key]: 'Invalid file type. Please select an image (PNG, JPG, etc.).' }));
        return;
      }

      // Validation: File Size (1MB)
      if (file.size > 1024 * 1024) {
        setUploadErrors(prev => ({ ...prev, [key]: 'File is too large. Maximum size is 1MB.' }));
        return;
      }

      const reader = new FileReader();
      reader.onloadend = () => {
        setSettings(prev => ({ ...prev, [key]: reader.result as string }));
      };
      reader.onerror = () => {
        setUploadErrors(prev => ({ ...prev, [key]: 'Failed to read file.' }));
      };
      reader.readAsDataURL(file);
    }
  };

  const clearImage = (key: 'customLogo' | 'customFavicon' | 'userAvatar' | 'agiAvatar') => {
    setSettings(prev => ({ ...prev, [key]: undefined }));
    setUploadErrors(prev => {
        const next = { ...prev };
        delete next[key];
        return next;
    });
  };

  return (
    <div className="fixed inset-y-0 right-0 w-80 bg-white dark:bg-zinc-900 border-l border-zinc-200 dark:border-zinc-800 shadow-2xl transform transition-transform duration-300 z-50 overflow-hidden flex flex-col">
      {/* Sidebar Header */}
      <div className="flex items-center justify-between p-4 border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 z-10 shrink-0">
        <h2 className="text-zinc-900 dark:text-zinc-100 font-medium flex items-center gap-2">
          {activeTab === 'settings' && <Server size={18} />}
          {activeTab === 'perimeter' && <Shield size={18} />}
          {activeTab === 'history' && <History size={18} />}
          {activeTab === 'settings' ? 'Configuration' : activeTab === 'perimeter' ? 'Sovereign Perimeter' : 'Chat History'}
        </h2>
        <button 
          onClick={onClose}
          className="text-zinc-400 hover:text-zinc-900 dark:hover:text-white transition-colors"
        >
          <X size={20} />
        </button>
      </div>

      {/* Tab Switcher */}
      <div className="grid grid-cols-3 p-2 gap-1 bg-zinc-50 dark:bg-zinc-950 border-b border-zinc-200 dark:border-zinc-800 shrink-0">
         <button
            onClick={() => setActiveTab('settings')}
            className={`flex items-center justify-center gap-1 py-2 text-[10px] font-medium rounded transition-all ${
                activeTab === 'settings'
                ? 'bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 shadow-sm border border-zinc-200 dark:border-zinc-700'
                : 'text-zinc-500 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
            }`}
         >
            <Settings2 size={12} />
            Settings
         </button>
         <button
            onClick={() => setActiveTab('perimeter')}
            className={`flex items-center justify-center gap-1 py-2 text-[10px] font-medium rounded transition-all ${
                activeTab === 'perimeter'
                ? 'bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 shadow-sm border border-zinc-200 dark:border-zinc-700'
                : 'text-zinc-500 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
            }`}
         >
            <Shield size={12} />
            Perimeter
         </button>
         <button
            onClick={() => setActiveTab('history')}
             className={`flex items-center justify-center gap-1 py-2 text-[10px] font-medium rounded transition-all ${
                activeTab === 'history'
                ? 'bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 shadow-sm border border-zinc-200 dark:border-zinc-700'
                : 'text-zinc-500 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
            }`}
         >
            <History size={12} />
            History ({messages.length})
         </button>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'perimeter' ? (
            <SovereignPerimeter
              userName={settings.userAlias || undefined}
              activeProjectId={activeProjectId}
              projectNames={projectNames ?? {}}
            />
        ) : activeTab === 'settings' ? (
            <div className="p-6 space-y-6">
                
                {/* Theme Selector */}
                <div className="p-1 bg-zinc-100 dark:bg-zinc-950 rounded-lg flex border border-zinc-200 dark:border-zinc-800">
                <button
                    onClick={() => setSettings(prev => ({ ...prev, theme: 'light' }))}
                    className={`flex-1 flex items-center justify-center gap-2 py-1.5 text-xs font-medium rounded-md transition-all ${
                    settings.theme === 'light' 
                        ? 'bg-white shadow text-zinc-900' 
                        : 'text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-300'
                    }`}
                >
                    <Sun size={14} />
                    Light
                </button>
                <button
                    onClick={() => setSettings(prev => ({ ...prev, theme: 'dark' }))}
                    className={`flex-1 flex items-center justify-center gap-2 py-1.5 text-xs font-medium rounded-md transition-all ${
                    settings.theme === 'dark' 
                        ? 'bg-zinc-800 shadow text-white' 
                        : 'text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-300'
                    }`}
                >
                    <Moon size={14} />
                    Dark
                </button>
                </div>

                {/* Counselor Settings / Orchestrator Role (Sovereign Base) */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                        <Shield size={14} />
                        Counselor Settings
                    </h3>

                    {/* Orchestrator Role: Counselor (base); system state tied */}
                    <div className="space-y-2">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500">Orchestrator Role</label>
                        <div className="p-1 rounded-lg flex border border-emerald-500/30 dark:border-emerald-600/40 bg-emerald-500/10 dark:bg-emerald-900/20">
                            <div className="flex-1 flex items-center justify-center gap-2 py-2 text-xs font-medium rounded-md bg-emerald-500 dark:bg-emerald-600 text-white shadow">
                                <Shield size={14} />
                                Counselor
                            </div>
                            {!isCounselor && (
                                <button
                                    onClick={() => !personaToggling && handleOrchestratorRoleSet('counselor')}
                                    disabled={personaToggling}
                                    className="flex-1 flex items-center justify-center gap-2 py-2 text-xs font-medium rounded-md text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                                >
                                    Reset to Counselor
                                </button>
                            )}
                        </div>
                        <p className="text-[10px] text-zinc-400 dark:text-zinc-500">System guidance and boundary management. Base template.</p>
                    </div>

                    {/* Birth Sign */}
                    <div className="space-y-1">
                        <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Birth Sign (Kardia)</label>
                        <select
                            value={settings.birthSign ?? ''}
                            onChange={(e) => setSettings(prev => ({ ...prev, birthSign: e.target.value || undefined }))}
                            className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600"
                        >
                            <option value="">— Select —</option>
                            {ZODIAC_SIGNS.map((s) => (
                                <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>
                            ))}
                        </select>
                    </div>

                    {/* Ascendant */}
                    <div className="space-y-1">
                        <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Ascendant</label>
                        <input
                            type="text"
                            value={settings.ascendant ?? ''}
                            onChange={(e) => setSettings(prev => ({ ...prev, ascendant: e.target.value || undefined }))}
                            className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600"
                            placeholder="e.g. Leo"
                        />
                    </div>

                    {/* Jungian Shadow Focus (Counselor mode) */}
                    {isCounselor && (
                        <div className="space-y-1">
                            <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Jungian Shadow Focus (Ethos)</label>
                            <textarea
                                value={settings.jungianShadowFocus ?? ''}
                                onChange={(e) => setSettings(prev => ({ ...prev, jungianShadowFocus: e.target.value || undefined }))}
                                className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 resize-y min-h-[60px]"
                                placeholder="Self-sabotage notes, e.g. perfectionism, people-pleasing"
                                rows={2}
                            />
                        </div>
                    )}

                    <p className="text-[10px] text-zinc-500 dark:text-zinc-600">
                        Archetype (birth sign, ascendant, shadow) is read from .env (PAGI_USER_SIGN, PAGI_ASCENDANT, PAGI_JUNGIAN_SHADOW_FOCUS). Restart gateway to apply.
                    </p>
                    <button
                        type="button"
                        onClick={() => {
                            setSaveEnvToast(true);
                            setTimeout(() => setSaveEnvToast(false), 3000);
                        }}
                        className="w-full flex items-center justify-center gap-2 py-2 rounded-lg border border-zinc-200 dark:border-zinc-700 text-zinc-600 dark:text-zinc-400 text-xs font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
                    >
                        {saveEnvToast ? 'Copied to clipboard idea — save to .env' : 'Save to Environment'}
                    </button>
                    {saveEnvToast && (
                        <p className="text-[10px] text-emerald-600 dark:text-emerald-400 animate-in">
                            Add PAGI_USER_SIGN, PAGI_ASCENDANT, PAGI_JUNGIAN_SHADOW_FOCUS to .env and restart the gateway.
                        </p>
                    )}
                </div>

                {/* Sovereign Perimeter — Protected Terms (SAO Redaction) */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                        <Shield size={14} />
                        Sovereign Perimeter
                    </h3>

                    {/* Vault Status (Keyring) */}
                    {vaultStatus && (
                        <div className="flex items-center gap-2 flex-wrap text-[10px]">
                            <span className="text-zinc-400 dark:text-zinc-500">Vault status:</span>
                            {vaultStatus.openrouter_in_vault ? (
                                <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
                                    <CheckCircle2 size={10} /> OpenRouter
                                </span>
                            ) : (
                                <span className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
                                    <XCircle size={10} /> OpenRouter
                                </span>
                            )}
                            {vaultStatus.pagi_llm_in_vault ? (
                                <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
                                    <CheckCircle2 size={10} /> PAGI LLM
                                </span>
                            ) : (
                                <span className="flex items-center gap-1 text-zinc-400 dark:text-zinc-500">
                                    <Key size={10} /> PAGI LLM
                                </span>
                            )}
                        </div>
                    )}

                    {/* Active Terms */}
                    <div className="space-y-2">
                        <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Protected terms (SAO redaction)</label>
                        {protectedTermsLoading ? (
                            <p className="text-[10px] text-zinc-500">Loading…</p>
                        ) : (
                            <>
                                <ul className="space-y-1 max-h-32 overflow-y-auto rounded border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-950 p-2">
                                    {protectedTerms.length === 0 ? (
                                        <li className="text-[10px] text-zinc-500 italic">No terms</li>
                                    ) : (
                                        protectedTerms.map((term) => (
                                            <li key={term} className="flex items-center justify-between gap-2 text-xs">
                                                <span className="font-mono text-zinc-800 dark:text-zinc-200">{term}</span>
                                                <button
                                                    type="button"
                                                    onClick={() => handleRemoveTerm(term)}
                                                    disabled={protectedTermsSaving}
                                                    className="p-1 rounded text-zinc-400 hover:text-red-500 hover:bg-red-500/10 disabled:opacity-50"
                                                    title="Remove term"
                                                >
                                                    <Trash2 size={12} />
                                                </button>
                                            </li>
                                        ))
                                    )}
                                </ul>
                                <div className="flex gap-2">
                                    <input
                                        type="text"
                                        value={newTerm}
                                        onChange={(e) => setNewTerm(e.target.value.toUpperCase())}
                                        onKeyDown={(e) => e.key === 'Enter' && handleAddTerm()}
                                        placeholder="New term (e.g. OMEGA)"
                                        className="flex-1 bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-amber-500/50"
                                    />
                                    <button
                                        type="button"
                                        onClick={handleAddTerm}
                                        disabled={protectedTermsSaving || !newTerm.trim()}
                                        className="flex items-center gap-1 px-2 py-1.5 rounded border border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300 text-xs font-medium hover:bg-amber-500/20 disabled:opacity-50"
                                    >
                                        <Plus size={12} /> Add
                                    </button>
                                </div>
                            </>
                        )}
                    </div>

                    {/* Test Redaction */}
                    <div className="space-y-2">
                        <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Test redaction</label>
                        <textarea
                            value={testRedactInput}
                            onChange={(e) => setTestRedactInput(e.target.value)}
                            placeholder="e.g. Discussing OMEGA with the team"
                            className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-amber-500/50 resize-y min-h-[60px]"
                            rows={2}
                        />
                        <button
                            type="button"
                            onClick={handleTestRedact}
                            disabled={testRedactLoading}
                            className="w-full flex items-center justify-center gap-2 py-1.5 rounded border border-zinc-300 dark:border-zinc-700 text-zinc-600 dark:text-zinc-400 text-xs font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800 disabled:opacity-50"
                        >
                            {testRedactLoading ? '…' : 'Preview sanitized'}
                        </button>
                        {testRedactOutput !== '' && (
                            <div className="rounded border border-zinc-200 dark:border-zinc-800 bg-zinc-100 dark:bg-zinc-900 p-2">
                                <p className="text-[10px] text-zinc-500 mb-1">Sanitized:</p>
                                <p className="text-xs font-mono text-zinc-800 dark:text-zinc-200 break-words">{testRedactOutput}</p>
                            </div>
                        )}
                    </div>
                </div>

                {/* User Profile Section */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                        <UserCircle size={14} />
                        User Profile
                    </h3>

                    <div className="flex items-start gap-3">
                        {/* Avatar Preview */}
                        <div className="w-12 h-12 bg-zinc-100 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded-full flex items-center justify-center overflow-hidden shrink-0 relative group shadow-sm">
                            {settings.userAvatar ? (
                            <>
                                <img src={settings.userAvatar} alt="Avatar" className="w-full h-full object-cover" />
                                <button 
                                onClick={() => clearImage('userAvatar')}
                                className="absolute inset-0 bg-black/60 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-white"
                                title="Remove avatar"
                                >
                                <Trash2 size={16} />
                                </button>
                            </>
                            ) : (
                            <UserCircle size={24} className="text-zinc-300 dark:text-zinc-700" />
                            )}
                        </div>

                        <div className="flex-1 space-y-2">
                            {/* Avatar Upload Drop Zone */}
                            <div className="relative">
                                <label 
                                    className={`cursor-pointer flex flex-col items-center justify-center gap-1.5 px-3 py-2 bg-zinc-50 dark:bg-zinc-900/50 hover:bg-zinc-100 dark:hover:bg-zinc-800 border-2 border-dashed rounded-lg transition-all duration-200 group/label ${
                                        uploadErrors['userAvatar'] 
                                        ? 'border-red-300 dark:border-red-900/50 bg-red-50/50 dark:bg-red-900/10' 
                                        : 'border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700'
                                    }`}
                                >
                                    <div className="flex items-center gap-2 text-zinc-500 dark:text-zinc-400 group-hover/label:text-zinc-700 dark:group-hover/label:text-zinc-200 transition-colors">
                                        <Upload size={14} />
                                        <span className="text-xs font-medium">Click to upload image</span>
                                    </div>
                                    <span className="text-[10px] text-zinc-400 dark:text-zinc-600">Max 1MB (PNG, JPG)</span>
                                    <input
                                        type="file"
                                        accept="image/*"
                                        className="hidden"
                                        onChange={(e) => handleFileUpload(e, 'userAvatar')}
                                    />
                                </label>
                                {uploadErrors['userAvatar'] && (
                                    <div className="flex items-center gap-1.5 mt-1.5 text-red-500 dark:text-red-400 text-[10px] animate-in slide-in-from-top-1">
                                        <AlertCircle size={10} />
                                        <span>{uploadErrors['userAvatar']}</span>
                                    </div>
                                )}
                            </div>

                            {/* Name Input */}
                            <div className="space-y-1">
                                <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Display Name</label>
                                <input
                                type="text"
                                value={settings.userAlias || ''}
                                onChange={(e) => setSettings(prev => ({ ...prev, userAlias: e.target.value }))}
                                className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600 focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 transition-all"
                                placeholder="e.g. Operator"
                                />
                            </div>
                        </div>
                    </div>
                </div>

                {/* Sovereign Core Persona & Avatar */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                        <Bot size={14} />
                        Sovereign Core & Persona
                    </h3>

                    {/* Orchestrator Avatar Upload */}
                    <div className="flex items-start gap-3">
                        <div className="w-10 h-10 bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded-lg flex items-center justify-center overflow-hidden shrink-0 relative group shadow-sm mt-1">
                            {settings.agiAvatar ? (
                            <>
                                <img src={settings.agiAvatar} alt="Orchestrator Avatar" className="w-full h-full object-cover" />
                                <button 
                                onClick={() => clearImage('agiAvatar')}
                                className="absolute inset-0 bg-black/60 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-white"
                                title="Remove orchestrator avatar"
                                >
                                <Trash2 size={16} />
                                </button>
                            </>
                            ) : (
                            <Bot size={20} className="text-zinc-400 dark:text-zinc-600" />
                            )}
                        </div>
                        <div className="flex-1">
                            <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold block mb-1">Orchestrator Avatar</label>
                            
                             <div className="relative mb-2">
                                <label 
                                    className={`cursor-pointer flex flex-col items-center justify-center gap-1.5 px-3 py-2 bg-zinc-50 dark:bg-zinc-900/50 hover:bg-zinc-100 dark:hover:bg-zinc-800 border-2 border-dashed rounded-lg transition-all duration-200 group/label ${
                                        uploadErrors['agiAvatar'] 
                                        ? 'border-red-300 dark:border-red-900/50 bg-red-50/50 dark:bg-red-900/10' 
                                        : 'border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700'
                                    }`}
                                >
                                    <div className="flex items-center gap-2 text-zinc-500 dark:text-zinc-400 group-hover/label:text-zinc-700 dark:group-hover/label:text-zinc-200 transition-colors">
                                        <Upload size={14} />
                                        <span className="text-xs font-medium">Upload Icon</span>
                                    </div>
                                    <span className="text-[10px] text-zinc-400 dark:text-zinc-600">Max 1MB (PNG, JPG)</span>
                                    <input
                                        type="file"
                                        accept="image/*"
                                        className="hidden"
                                        onChange={(e) => handleFileUpload(e, 'agiAvatar')}
                                    />
                                </label>
                                {uploadErrors['agiAvatar'] && (
                                    <div className="flex items-center gap-1.5 mt-1.5 text-red-500 dark:text-red-400 text-[10px] animate-in slide-in-from-top-1">
                                        <AlertCircle size={10} />
                                        <span>{uploadErrors['agiAvatar']}</span>
                                    </div>
                                )}
                            </div>

                            {/* URL Input */}
                            <input
                                type="text"
                                value={settings.agiAvatar?.startsWith('data:') ? '' : settings.agiAvatar || ''}
                                onChange={(e) => setSettings(prev => ({ ...prev, agiAvatar: e.target.value }))}
                                className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600 focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 transition-all"
                                placeholder={settings.agiAvatar?.startsWith('data:') ? "Using uploaded image" : "Or paste image URL..."}
                            />
                        </div>
                    </div>

                    {/* Orchestrator Persona */}
                    <div className="space-y-2">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500">Orchestrator Persona</label>
                        <div className="relative">
                        <select
                            value={settings.orchestratorPersona}
                            onChange={(e) => setSettings(prev => ({ ...prev, orchestratorPersona: e.target.value }))}
                            className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-sm focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600 focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 appearance-none cursor-pointer transition-all"
                        >
                            <option value="general_assistant">General Assistant</option>
                            <option value="researcher">Deep Researcher</option>
                            <option value="coder">Senior Developer</option>
                            <option value="creative">Creative Writer</option>
                            <option value="analyst">Data Analyst</option>
                            <option value="socratic">Socratic Tutor</option>
                        </select>
                        <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-zinc-500">
                            <Brain size={14} />
                        </div>
                        </div>
                    </div>

                    {/* LLM Model Name */}
                    <div className="space-y-2">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500">Model ID</label>
                        <div className="relative">
                        <input
                            type="text"
                            list="model-suggestions"
                            value={settings.llmModel}
                            onChange={(e) => setSettings(prev => ({ ...prev, llmModel: e.target.value }))}
                            className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-sm focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600 focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 transition-all font-mono"
                            placeholder="e.g. deepseek/deepseek-v3.2, openai/gpt-4o-mini"
                        />
                        <datalist id="model-suggestions">
                            <option value="deepseek/deepseek-v3.2" />
                            <option value="openai/gpt-4o" />
                            <option value="anthropic/claude-3.5-sonnet" />
                            <option value="meta-llama/llama-3.3-70b-instruct:free" />
                            <option value="google/gemini-2.0-flash-001" />
                            <option value="mistralai/mistral-7b-instruct" />
                        </datalist>
                        </div>
                    </div>

                    {/* Temperature */}
                    <div className="space-y-2">
                        <div className="flex justify-between items-center">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500 flex items-center gap-1">
                            <Sliders size={12} />
                            Temperature
                        </label>
                        <span className="text-xs font-mono text-zinc-600 dark:text-zinc-400 bg-zinc-100 dark:bg-zinc-800 px-1.5 py-0.5 rounded">
                            {settings.llmTemperature}
                        </span>
                        </div>
                        <input
                        type="range"
                        min="0"
                        max="2"
                        step="0.1"
                        value={settings.llmTemperature}
                        onChange={(e) => setSettings(prev => ({ ...prev, llmTemperature: parseFloat(e.target.value) }))}
                        className="w-full h-1.5 bg-zinc-200 dark:bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-orange-500"
                        />
                        <div className="flex justify-between text-[10px] text-zinc-400">
                        <span>Precise</span>
                        <span>Creative</span>
                        </div>
                    </div>

                    {/* Max Tokens */}
                    <div className="space-y-2">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500 flex items-center gap-1">
                            <MessageSquare size={12} />
                            Max Tokens
                        </label>
                        <input
                        type="number"
                        value={settings.llmMaxTokens}
                        onChange={(e) => setSettings(prev => ({ ...prev, llmMaxTokens: parseInt(e.target.value) || 0 }))}
                        className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-sm focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600 focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 transition-all font-mono"
                        />
                    </div>
                </div>

                {/* API URL Configuration */}
                <div className="space-y-2 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                <label className="text-xs uppercase tracking-wider text-zinc-500 font-semibold flex items-center gap-2">
                    <Server size={14} />
                    Orchestrator Endpoint (Gateway only)
                </label>
                <input
                    type="text"
                    readOnly
                    value="http://127.0.0.1:8000/api/v1/chat"
                    className="w-full bg-zinc-100 dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-600 dark:text-zinc-400 text-sm font-mono cursor-not-allowed"
                    title="Chat is hardcoded to the Rust Gateway. No 3001, no demo."
                />
                </div>

                {/* Server configuration from .env (read-only here; edit .env and restart gateway to change) */}
                <div className="space-y-3 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                <label className="text-xs uppercase tracking-wider text-zinc-500 font-semibold flex items-center gap-2">
                    <Folder size={14} />
                    Server configuration (.env)
                </label>
                <p className="text-[10px] text-zinc-500 dark:text-zinc-600 leading-relaxed">
                    Values below come from .env. To change them, edit .env and restart the gateway. MoE mode can be toggled in this UI and is persisted to KB-6.
                </p>
                {gatewayConfig ? (
                    <>
                    <div className="grid grid-cols-1 gap-2 text-sm">
                        <div className="flex items-center justify-between">
                            <span className="text-zinc-600 dark:text-zinc-400">File system access</span>
                            <span className={`font-mono text-xs px-1.5 py-0.5 rounded ${gatewayConfig.fs_access_enabled ? 'bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-400' : 'bg-zinc-200 dark:bg-zinc-700 text-zinc-600 dark:text-zinc-400'}`}>
                                {gatewayConfig.fs_access_enabled ? 'Enabled' : 'Disabled'}
                            </span>
                        </div>
                        <div className="space-y-0.5">
                            <label className="text-[10px] text-zinc-500 dark:text-zinc-600">Workspace root (PAGI_FS_ROOT)</label>
                            <input
                                type="text"
                                readOnly
                                value={gatewayConfig.fs_root || '(current directory)'}
                                className="w-full bg-zinc-100 dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-600 dark:text-zinc-400 text-sm font-mono truncate"
                                title="Set PAGI_FS_ROOT in .env to restrict file operations to a folder."
                            />
                        </div>
                        <div className="flex items-center justify-between">
                            <span className="text-zinc-600 dark:text-zinc-400">LLM mode</span>
                            <span className="font-mono text-xs text-zinc-500 dark:text-zinc-400">{gatewayConfig.llm_mode}</span>
                        </div>
                        {gatewayConfig.llm_model != null && (
                        <div className="space-y-0.5">
                            <label className="text-[10px] text-zinc-500 dark:text-zinc-600">LLM model (PAGI_LLM_MODEL)</label>
                            <div className="font-mono text-xs text-zinc-500 dark:text-zinc-400 truncate" title={gatewayConfig.llm_model}>{gatewayConfig.llm_model}</div>
                        </div>
                        )}
                        {gatewayConfig.tick_rate_secs != null && (
                        <div className="flex items-center justify-between">
                            <span className="text-zinc-600 dark:text-zinc-400">Heartbeat tick (PAGI_TICK_RATE_SECS)</span>
                            <span className="font-mono text-xs text-zinc-500 dark:text-zinc-400">{gatewayConfig.tick_rate_secs}s</span>
                        </div>
                        )}
                        {gatewayConfig.local_context_limit != null && (
                        <div className="flex items-center justify-between">
                            <span className="text-zinc-600 dark:text-zinc-400">Gater context limit (PAGI_LOCAL_CONTEXT_LIMIT)</span>
                            <span className="font-mono text-xs text-zinc-500 dark:text-zinc-400">{gatewayConfig.local_context_limit}</span>
                        </div>
                        )}
                        {gatewayConfig.moe_default != null && (
                        <div className="flex items-center justify-between">
                            <span className="text-zinc-600 dark:text-zinc-400">MoE default at startup (PAGI_MOE_DEFAULT)</span>
                            <span className="font-mono text-xs text-zinc-500 dark:text-zinc-400">{gatewayConfig.moe_default}</span>
                        </div>
                        )}
                    </div>
                    </>
                ) : (
                    <p className="text-[10px] text-zinc-500">Connect to gateway to load config.</p>
                )}
                <div className="space-y-1 pt-2">
                    <label className="text-[10px] text-zinc-500 dark:text-zinc-600">Preferred workspace path (optional override)</label>
                    <input
                        type="text"
                        value={settings.preferredWorkspacePath ?? ''}
                        onChange={(e) => setSettings(prev => ({ ...prev, preferredWorkspacePath: e.target.value }))}
                        placeholder="Leave empty to use server PAGI_FS_ROOT"
                        className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-sm font-mono focus:outline-none focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600"
                    />
                    <p className="text-[10px] text-zinc-500 dark:text-zinc-600">Stored in browser; use when requesting file operations.</p>
                </div>
                </div>

                {/* Logs */}
                <div className="space-y-3 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                  <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                    <Terminal size={14} />
                    Logs
                  </h3>
                  <p className="text-[10px] text-zinc-500 dark:text-zinc-600 leading-relaxed">
                    Live log stream from the Gateway (Server-Sent Events).
                  </p>
                  <div className="rounded-lg overflow-hidden border border-zinc-200 dark:border-zinc-800">
                    <LogTerminal logsUrl={logsUrl} />
                  </div>
                </div>

                {/* Feature Toggles */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                <div 
                    className="flex items-center justify-between cursor-pointer group"
                    onClick={() => setSettings(prev => ({ ...prev, stream: !prev.stream }))}
                >
                    <span className="flex items-center gap-2 text-zinc-600 dark:text-zinc-400 group-hover:text-zinc-900 dark:group-hover:text-zinc-200 transition-colors text-sm">
                    <Zap size={16} />
                    Streaming (Experimental)
                    </span>
                    <div className={`w-10 h-5 rounded-full relative transition-colors ${settings.stream ? 'bg-orange-500 dark:bg-orange-900' : 'bg-zinc-300 dark:bg-zinc-800'}`}>
                    <div className={`absolute top-1 w-3 h-3 rounded-full bg-white transition-all duration-200 ${settings.stream ? 'left-6 bg-orange-100 dark:bg-orange-400' : 'left-1 bg-zinc-500'}`} />
                    </div>
                </div>

                <div 
                    className="flex items-center justify-between cursor-pointer group"
                    onClick={() => setSettings(prev => ({ ...prev, showThoughts: !prev.showThoughts }))}
                >
                    <span className="flex items-center gap-2 text-zinc-600 dark:text-zinc-400 group-hover:text-zinc-900 dark:group-hover:text-zinc-200 transition-colors text-sm">
                    <Eye size={16} />
                    Show Reasoning Layers
                    </span>
                    <div className={`w-10 h-5 rounded-full relative transition-colors ${settings.showThoughts ? 'bg-indigo-600 dark:bg-indigo-900' : 'bg-zinc-300 dark:bg-zinc-800'}`}>
                    <div className={`absolute top-1 w-3 h-3 rounded-full bg-white transition-all duration-200 ${settings.showThoughts ? 'left-6 bg-indigo-100 dark:bg-indigo-400' : 'left-1 bg-zinc-500'}`} />
                    </div>
                </div>

                <div 
                    className="flex items-center justify-between cursor-pointer group"
                    onClick={() => !moeToggling && handleMoeToggle()}
                >
                    <span className="flex items-center gap-2 text-zinc-600 dark:text-zinc-400 group-hover:text-zinc-900 dark:group-hover:text-zinc-200 transition-colors text-sm">
                    <Network size={16} />
                    Sparse Intelligence (MoE Mode)
                    {gatewayConfig?.moe_mode && (
                      <span className="text-[10px] font-medium uppercase text-zinc-500 dark:text-zinc-500">
                        ({gatewayConfig.moe_mode === 'sparse' ? 'Sparse' : 'Dense'})
                      </span>
                    )}
                    </span>
                    <div className={`w-10 h-5 rounded-full relative transition-colors ${gatewayConfig?.moe_active ? 'bg-amber-500 dark:bg-amber-900' : 'bg-zinc-300 dark:bg-zinc-800'} ${moeToggling ? 'opacity-70' : ''}`}>
                    <div className={`absolute top-1 w-3 h-3 rounded-full bg-white transition-all duration-200 ${gatewayConfig?.moe_active ? 'left-6 bg-amber-100 dark:bg-amber-400' : 'left-1 bg-zinc-500'}`} />
                    </div>
                </div>

                <div 
                    className="flex items-center justify-between cursor-pointer group"
                    onClick={() => setSettings(prev => ({ ...prev, sovereignProtocols: !prev.sovereignProtocols }))}
                >
                    <span className="flex items-center gap-2 text-zinc-600 dark:text-zinc-400 group-hover:text-zinc-900 dark:group-hover:text-zinc-200 transition-colors text-sm">
                    <Shield size={16} />
                    Active Domain Protection (KB-05)
                    <span className="text-[10px] font-medium uppercase text-zinc-500 dark:text-zinc-500">
                        (Sovereign Security)
                    </span>
                    </span>
                    <div className={`w-10 h-5 rounded-full relative transition-colors ${settings.sovereignProtocols ? 'bg-emerald-600 dark:bg-emerald-900' : 'bg-zinc-300 dark:bg-zinc-800'}`}>
                    <div className={`absolute top-1 w-3 h-3 rounded-full bg-white transition-all duration-200 ${settings.sovereignProtocols ? 'left-6 bg-emerald-100 dark:bg-emerald-400' : 'left-1 bg-zinc-500'}`} />
                    </div>
                </div>
                </div>

                {/* Branding Settings */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider">Custom Branding</h3>
                    
                    {/* Logo Upload */}
                    <div className="space-y-2">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500">Custom Logo</label>
                        <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded flex items-center justify-center overflow-hidden shrink-0 relative group shadow-sm">
                            {settings.customLogo ? (
                            <>
                                <img src={settings.customLogo} alt="Logo" className="w-full h-full object-contain p-1" />
                                <button 
                                onClick={() => clearImage('customLogo')}
                                className="absolute inset-0 bg-black/60 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-red-400"
                                >
                                <Trash2 size={14} />
                                </button>
                            </>
                            ) : (
                            <ImageIcon size={16} className="text-zinc-400 dark:text-zinc-600" />
                            )}
                        </div>
                        
                        <div className="flex-1 relative">
                             <label className={`cursor-pointer flex items-center justify-center gap-2 bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700 hover:bg-white dark:hover:bg-zinc-800/50 text-zinc-700 dark:text-zinc-300 text-xs py-2 px-3 rounded transition-all ${uploadErrors['customLogo'] ? 'border-red-300 dark:border-red-900/50 bg-red-50/20' : ''}`}>
                                <Upload size={14} />
                                <span>Upload Logo</span>
                                <input
                                type="file"
                                accept="image/*"
                                className="hidden"
                                onChange={(e) => handleFileUpload(e, 'customLogo')}
                                />
                            </label>
                             {uploadErrors['customLogo'] && (
                                <div className="flex items-center gap-1.5 mt-1.5 text-red-500 dark:text-red-400 text-[10px] animate-in slide-in-from-top-1">
                                    <AlertCircle size={10} />
                                    <span>{uploadErrors['customLogo']}</span>
                                </div>
                            )}
                        </div>
                        </div>
                    </div>

                    {/* Favicon Upload */}
                    <div className="space-y-2">
                        <label className="text-xs text-zinc-400 dark:text-zinc-500">Custom Favicon</label>
                        <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded flex items-center justify-center overflow-hidden shrink-0 relative group shadow-sm">
                            {settings.customFavicon ? (
                            <>
                                <img src={settings.customFavicon} alt="Favicon" className="w-full h-full object-contain p-2" />
                                <button 
                                onClick={() => clearImage('customFavicon')}
                                className="absolute inset-0 bg-black/60 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-red-400"
                                >
                                <Trash2 size={14} />
                                </button>
                            </>
                            ) : (
                            <ImageIcon size={16} className="text-zinc-400 dark:text-zinc-600" />
                            )}
                        </div>
                        <div className="flex-1 relative">
                             <label className={`cursor-pointer flex items-center justify-center gap-2 bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700 hover:bg-white dark:hover:bg-zinc-800/50 text-zinc-700 dark:text-zinc-300 text-xs py-2 px-3 rounded transition-all ${uploadErrors['customFavicon'] ? 'border-red-300 dark:border-red-900/50 bg-red-50/20' : ''}`}>
                                <Upload size={14} />
                                <span>Upload Favicon</span>
                                <input
                                type="file"
                                accept="image/*"
                                className="hidden"
                                onChange={(e) => handleFileUpload(e, 'customFavicon')}
                                />
                            </label>
                             {uploadErrors['customFavicon'] && (
                                <div className="flex items-center gap-1.5 mt-1.5 text-red-500 dark:text-red-400 text-[10px] animate-in slide-in-from-top-1">
                                    <AlertCircle size={10} />
                                    <span>{uploadErrors['customFavicon']}</span>
                                </div>
                            )}
                        </div>
                        </div>
                    </div>
                </div>

                {/* Custom CSS */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                    <Palette size={12} />
                    Advanced Styling
                </h3>
                <div className="space-y-2">
                    <label className="text-xs text-zinc-400 dark:text-zinc-500">Custom CSS</label>
                    <textarea
                    value={settings.customCss || ''}
                    onChange={(e) => setSettings(prev => ({ ...prev, customCss: e.target.value }))}
                    className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-3 py-2 text-zinc-900 dark:text-zinc-300 text-xs font-mono focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600 focus:ring-1 focus:ring-zinc-400 dark:focus:ring-zinc-600 transition-all h-32 resize-y"
                    placeholder=".bg-zinc-950 { background-color: #000; }"
                    spellCheck={false}
                    />
                    <p className="text-[10px] text-zinc-500 dark:text-zinc-600">
                    Override global styles. Changes apply immediately.
                    </p>
                </div>
                </div>

                {/* Data Management */}
                <div className="space-y-4 pt-4 border-t border-zinc-200 dark:border-zinc-800">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                        <Trash2 size={12} />
                        Data Management
                    </h3>
                    <button
                        onClick={onClearChat}
                        className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-red-50 dark:bg-red-900/10 hover:bg-red-100 dark:hover:bg-red-900/20 text-red-600 dark:text-red-400 border border-red-200 dark:border-red-900/30 rounded-md transition-colors text-xs font-medium"
                    >
                        <Trash2 size={14} />
                        Clear All Chat History
                    </button>
                </div>

                {/* L2 Memory - Knowledge Bases Status */}
                <div className="mt-4 p-4 bg-zinc-50 dark:bg-zinc-950/50 rounded border border-zinc-200 dark:border-zinc-800/50">
                <div className="flex items-center justify-between mb-3">
                    <h3 className="text-xs font-semibold text-zinc-500 uppercase flex items-center gap-2">
                        <Database size={12} />
                        L2 Memory (8 KBs)
                    </h3>
                    <button
                        onClick={fetchKbStatus}
                        disabled={kbLoading}
                        className="p-1 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors disabled:opacity-50"
                        title="Refresh KB Status"
                    >
                        <RefreshCw size={12} className={kbLoading ? 'animate-spin' : ''} />
                    </button>
                </div>
                
                {kbError && (
                    <div className="text-xs text-red-500 dark:text-red-400 mb-2 flex items-center gap-1">
                        <AlertCircle size={10} />
                        <span>{kbError}</span>
                    </div>
                )}
                
                {kbStatus && (
                    <>
                        <div className="flex items-center gap-2 mb-3 pb-2 border-b border-zinc-200 dark:border-zinc-800">
                            {kbStatus.all_connected ? (
                                <CheckCircle2 size={14} className="text-emerald-500" />
                            ) : (
                                <XCircle size={14} className="text-red-500" />
                            )}
                            <span className={`text-xs font-medium ${kbStatus.all_connected ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-600 dark:text-red-400'}`}>
                                {kbStatus.all_connected ? 'All KBs Connected' : 'Some KBs Offline'}
                            </span>
                            <span className="text-[10px] text-zinc-400 ml-auto">
                                {kbStatus.total_entries} entries
                            </span>
                        </div>
                        
                        <div className="grid grid-cols-2 gap-1.5">
                            {kbStatus.knowledge_bases.map((kb) => (
                                <div 
                                    key={kb.slot_id}
                                    className={`flex items-center gap-1.5 px-2 py-1 rounded text-[10px] ${
                                        kb.connected 
                                            ? 'bg-emerald-50 dark:bg-emerald-900/20 text-emerald-700 dark:text-emerald-300' 
                                            : 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300'
                                    }`}
                                    title={`${kb.name}\nTree: ${kb.tree_name}\nEntries: ${kb.entry_count}${kb.error ? `\nError: ${kb.error}` : ''}`}
                                >
                                    <span className={`w-1.5 h-1.5 rounded-full ${kb.connected ? 'bg-emerald-500' : 'bg-red-500'}`} />
                                    <span className="font-medium truncate">KB-{kb.slot_id}</span>
                                    <span className="text-[9px] opacity-70 ml-auto">{kb.entry_count}</span>
                                </div>
                            ))}
                        </div>
                    </>
                )}
                
                {!kbStatus && !kbError && kbLoading && (
                    <div className="text-xs text-zinc-400 flex items-center gap-2">
                        <RefreshCw size={12} className="animate-spin" />
                        <span>Loading KB status...</span>
                    </div>
                )}
                </div>

                {/* Debug Info */}
                <div className="mt-4 p-4 bg-zinc-50 dark:bg-zinc-950/50 rounded border border-zinc-200 dark:border-zinc-800/50">
                <h3 className="text-xs font-semibold text-zinc-500 mb-2 uppercase">System Info</h3>
                <div className="text-xs text-zinc-500 dark:text-zinc-600 font-mono space-y-1">
                    <p>Status: <span className="text-orange-600 dark:text-orange-500">Ready</span></p>
                    <p>Memory: <span className="text-zinc-500">Bare Metal (Sled)</span></p>
                    <p>Protocol: <span className="text-zinc-500">REST/JSON</span></p>
                    <div className="pt-2 flex items-center gap-2 text-[10px] opacity-70">
                    <Terminal size={10} />
                    <span>{settings.llmModel} ({settings.llmTemperature})</span>
                    </div>
                </div>
                </div>
            </div>
        ) : (
            <div className="p-4 space-y-4 min-h-0 flex flex-col h-full">
                {/* Search & Filters */}
                <div className="space-y-3 pb-4 border-b border-zinc-200 dark:border-zinc-800 shrink-0">
                    <div className="relative">
                        <input 
                           type="text" 
                           placeholder="Search history..." 
                           value={searchQuery}
                           onChange={(e) => setSearchQuery(e.target.value)}
                           className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded-md pl-9 pr-3 py-2 text-sm text-zinc-900 dark:text-zinc-200 focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600"
                        />
                        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400" />
                        {searchQuery && (
                            <button 
                                onClick={() => setSearchQuery('')}
                                className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
                            >
                                <X size={12} />
                            </button>
                        )}
                    </div>
                    
                    <div className="flex gap-2">
                        <div className="relative flex-1">
                           <select
                                value={roleFilter}
                                onChange={(e) => setRoleFilter(e.target.value as any)}
                                className="w-full appearance-none bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded-md pl-8 pr-3 py-1.5 text-xs font-medium text-zinc-600 dark:text-zinc-400 focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600"
                            >
                                <option value="all">All Roles</option>
                                <option value="user">User Only</option>
                                <option value="agi">AGI Only</option>
                            </select>
                            <Filter size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-400" />
                        </div>
                         <div className="relative flex-1">
                           <select
                                value={timeFilter}
                                onChange={(e) => setTimeFilter(e.target.value as any)}
                                className="w-full appearance-none bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded-md pl-8 pr-3 py-1.5 text-xs font-medium text-zinc-600 dark:text-zinc-400 focus:outline-none focus:border-zinc-400 dark:focus:border-zinc-600"
                            >
                                <option value="all">Any Time</option>
                                <option value="24h">Past 24h</option>
                                <option value="7d">Past 7 Days</option>
                            </select>
                            <Calendar size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-400" />
                        </div>
                    </div>
                </div>

                {/* Results List */}
                <div className="flex-1 overflow-y-auto min-h-0 -mx-2 px-2 space-y-3">
                    {filteredMessages.length === 0 ? (
                        <div className="flex flex-col items-center justify-center h-40 text-zinc-400 text-center">
                             <Search size={24} className="mb-2 opacity-20" />
                             <p className="text-xs">No messages found</p>
                        </div>
                    ) : (
                        filteredMessages.map(msg => (
                            <div key={msg.id} className="p-3 rounded-lg bg-zinc-50 dark:bg-zinc-950/50 border border-zinc-100 dark:border-zinc-800/50 hover:border-zinc-300 dark:hover:border-zinc-700 transition-colors group">
                                <div className="flex items-center justify-between mb-1.5">
                                    <span className={`text-[10px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded ${msg.role === 'user' ? 'bg-zinc-200 text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300' : 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-300'}`}>
                                        {msg.role}
                                    </span>
                                    <span className="text-[10px] text-zinc-400 font-mono">
                                        {new Date(msg.timestamp).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                                    </span>
                                </div>
                                <p className="text-xs text-zinc-600 dark:text-zinc-400 line-clamp-3 leading-relaxed font-medium">
                                    {msg.content}
                                </p>
                                {msg.thoughts && msg.thoughts.length > 0 && (
                                    <div className="mt-2 flex items-center gap-1.5 text-[10px] text-zinc-400 dark:text-zinc-500">
                                        <Brain size={10} />
                                        <span>{msg.thoughts.length} thought layers</span>
                                    </div>
                                )}
                            </div>
                        ))
                    )}
                </div>
            </div>
        )}
      </div>
    </div>
  );
};

export default SettingsSidebar;
