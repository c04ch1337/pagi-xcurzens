import React, { useState, useRef, useEffect, useMemo } from 'react';
import { Send, Cpu, User, Loader2, Copy, Check, Bot, Pin, BrainCircuit, AlertCircle, Shield, Mic, FolderOpen, Activity, Key } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { Message, AppSettings } from '../types';
import ThoughtBlock from './ThoughtBlock';
import SubjectProfile from './SubjectProfile';
import DiagramViewer from './DiagramViewer';
import { API_BASE_URL } from '../src/api/config';
import { useChatStore } from '../src/stores/ChatStore';
import {
  getMimirPreflight,
  postMimirStart,
  postMimirStop,
  postMimirSynthesize,
  getMimirStatus,
} from '../services/apiService';

interface ChatInterfaceProps {
  messages: Message[];
  isLoading: boolean;
  isStreaming: boolean;
  /** Strategic Timing Phase 2: status line while Architect is thinking (e.g. "Architect Analyzing...", "Auditing Perimeter..."). */
  streamingStatusMessage?: string | null;
  onSendMessage: (text: string) => void;
  onTogglePin: (id: string) => void;
  settings: AppSettings;
  /** When true, gateway is in MoE (Sparse Intelligence) mode; show in status line. */
  moeActive?: boolean;
  /** Open Settings sidebar (e.g. for Manage Vault). */
  onOpenSettings?: () => void;
}

const QUICK_ACTION_CLASS =
  'inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all border bg-white/5 dark:bg-zinc-900/50 border-zinc-200/80 dark:border-zinc-700/80 hover:bg-amber-500/10 hover:border-amber-500/30 dark:hover:bg-amber-500/10 dark:hover:border-amber-500/30 text-zinc-700 dark:text-zinc-300 hover:text-amber-800 dark:hover:text-amber-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-500/50';

/** Sovereign greeting when there are no messages: Master Orchestrator tone + Mission Control Quick Actions. */
interface EmptyStateGreetingProps {
  onSendMessage: (text: string) => void;
  onOpenSettings?: () => void;
}

const EmptyStateGreeting: React.FC<EmptyStateGreetingProps> = ({ onSendMessage, onOpenSettings }) => {
  const [vitality, setVitality] = useState<'ok' | 'standby' | null>(null);
  const [mimirRecording, setMimirRecording] = useState(false);
  const [mimirLoading, setMimirLoading] = useState(false);
  const [mimirPreflightAdvice, setMimirPreflightAdvice] = useState<string | null>(null);
  const { state: chatState, actions: chatActions } = useChatStore();

  /** Most recently updated project (Chronos); fallback to PROOFPOINT by name if no history. */
  const recentProject = useMemo(() => {
    const sorted = [...chatState.projects].sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0));
    const mostRecent = sorted[0];
    if (mostRecent) return mostRecent;
    const proofpoint = chatState.projects.find((p) => p.name.toUpperCase() === 'PROOFPOINT');
    return proofpoint ?? null;
  }, [chatState.projects]);

  useEffect(() => {
    let cancelled = false;
    fetch(`${API_BASE_URL}/domain/vitality`)
      .then((r) => (r.ok ? r.json() : null))
      .then((data: { vitality?: string } | null) => {
        if (cancelled) return;
        const v = (data?.vitality ?? '').toLowerCase();
        setVitality(v === 'stable' || v === 'ok' ? 'ok' : 'standby');
      })
      .catch(() => { if (!cancelled) setVitality('standby'); });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    getMimirStatus()
      .then((s) => { if (!cancelled) setMimirRecording(s.recording); })
      .catch(() => {});
    return () => { cancelled = true; };
  }, []);

  const handleRecordMeeting = async () => {
    setMimirPreflightAdvice(null);
    setMimirLoading(true);
    try {
      const preflight = await getMimirPreflight();
      if (!preflight.mic_active) {
        setMimirPreflightAdvice(preflight.user_advice ?? 'No microphone detected. Check sound settings.');
        return;
      }
      if (preflight.user_advice && !preflight.loopback_active) {
        setMimirPreflightAdvice(preflight.user_advice);
      }
      await postMimirStart(recentProject?.id ?? null);
      setMimirRecording(true);
    } catch (e) {
      setMimirPreflightAdvice(e instanceof Error ? e.message : 'Could not start recording.');
    } finally {
      setMimirLoading(false);
    }
  };

  const handleStopMeeting = async () => {
    setMimirLoading(true);
    try {
      const result = await postMimirStop();
      setMimirRecording(false);
      if (result.summary_path) {
        chatActions.setLastMimirSummaryPath(result.summary_path);
      }
      const summaryPath = result.summary_path ?? 'saved';
      const architectView = `**Architect's View — Meeting summary saved.**\n\nSummary written to: \`${summaryPath}\`. You can ask me to extract action items, cross-reference with project logs, or generate SAO-ready minutes.`;
      const activeId = chatState.activeThreadId;
      if (activeId) {
        chatActions.addMessage(activeId, {
          id: `mimir-${result.meeting_id}`,
          role: 'agi',
          content: architectView,
          timestamp: Date.now(),
        });
      }
      setMimirPreflightAdvice(null);
    } catch (e) {
      setMimirPreflightAdvice(e instanceof Error ? e.message : 'Could not stop recording.');
    } finally {
      setMimirLoading(false);
    }
  };

  const handleResumeProject = () => {
    if (recentProject) {
      chatActions.newThread({ title: 'New Chat', projectId: recentProject.id });
    } else {
      chatActions.newThread({ title: 'New Chat', projectId: null });
    }
  };

  const handleHardwareAudit = () => {
    onSendMessage(
      "Please run GetHardwareStats and show the Architect's view of machine health, including a Mermaid diagram if applicable."
    );
  };

  const handleManageVault = () => {
    onOpenSettings?.();
  };

  const [synthesizeLoading, setSynthesizeLoading] = useState(false);
  const handleSynthesize = async () => {
    const path = chatState.lastMimirSummaryPath;
    if (!path) return;
    setSynthesizeLoading(true);
    try {
      const activeThread = chatState.threads.find((t) => t.id === chatState.activeThreadId);
      const projectId = activeThread?.projectId ?? null;
      const result = await postMimirSynthesize(path, projectId);
      const alertLines =
        result.alerts.length > 0
          ? result.alerts.map((a) => `- **${a.kind}:** ${a.message}`).join('\n')
          : '';
      const content = `**Sovereign Vitality × Meeting Context**\n\n${result.summary}\n\n${alertLines ? `### Alerts\n${alertLines}\n\n` : ''}\n\`\`\`mermaid\n${result.mermaid}\n\`\`\``;
      if (chatState.activeThreadId) {
        chatActions.addMessage(chatState.activeThreadId, {
          id: `synthesize-${Date.now()}`,
          role: 'agi',
          content,
          timestamp: Date.now(),
        });
      }
    } catch (e) {
      console.error('Synthesize failed:', e);
    } finally {
      setSynthesizeLoading(false);
    }
  };

  return (
    <div className="h-full flex flex-col items-center justify-center text-center px-6 text-zinc-500 dark:text-zinc-500 select-none transition-colors">
      <div className="rounded-full bg-zinc-100 dark:bg-zinc-800/80 p-5 mb-4">
        <Cpu size={40} className="text-zinc-400 dark:text-zinc-500" aria-hidden />
      </div>
      <h2 className="text-lg font-semibold text-zinc-800 dark:text-zinc-200 tracking-tight">Phoenix Sovereign Core</h2>
      <p className="text-sm mt-1.5 text-zinc-600 dark:text-zinc-400 max-w-sm">
        System Initialized. Counselor-Architect at your service, Jamey.
      </p>
      <div className="mt-3 flex items-center justify-center gap-2 flex-wrap">
        <span className="text-xs font-mono text-zinc-400 dark:text-zinc-600">Status: Idle</span>
        {vitality !== null && (
          <span
            className="text-[10px] font-mono px-2 py-0.5 rounded border border-zinc-200 dark:border-zinc-700 bg-zinc-50 dark:bg-zinc-900/50 text-zinc-500 dark:text-zinc-500"
            title="Domain vitality from gateway"
          >
            System Vitality: {vitality === 'ok' ? 'OK' : 'Standby'}
          </span>
        )}
      </div>

      {mimirPreflightAdvice && (
        <p className="mt-2 text-xs max-w-md text-amber-600 dark:text-amber-400" role="alert">
          {mimirPreflightAdvice}
        </p>
      )}
      {/* Mission Control Quick Actions */}
      <div className="flex flex-wrap gap-4 mt-6 justify-center">
        {mimirRecording ? (
          <button
            type="button"
            onClick={handleStopMeeting}
            disabled={mimirLoading}
            className={QUICK_ACTION_CLASS}
            title="Stop recording and save Architect's View summary"
          >
            <span className="relative flex h-2 w-2 mr-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75" />
              <span className="relative inline-flex rounded-full h-2 w-2 bg-red-500" />
            </span>
            {mimirLoading ? 'Stopping…' : 'Stop Recording'}
          </button>
        ) : (
          <button
            type="button"
            onClick={handleRecordMeeting}
            disabled={mimirLoading}
            className={QUICK_ACTION_CLASS}
            title="Pre-flight check then start Mimir (mic + loopback) and Whisper transcription"
          >
            <Mic size={16} aria-hidden />
            {mimirLoading ? 'Starting…' : 'Record Meeting'}
          </button>
        )}
        <button
          type="button"
          onClick={handleResumeProject}
          className={QUICK_ACTION_CLASS}
          title={recentProject ? `Resume in ${recentProject.name} (Chronos)` : 'Create a new thread; tag to a project in the sidebar'}
        >
          <FolderOpen size={16} aria-hidden />
          {recentProject ? `Resume: ${recentProject.name}` : 'Create First Project'}
        </button>
        <button
          type="button"
          onClick={handleHardwareAudit}
          className={QUICK_ACTION_CLASS}
          title="Run GetHardwareStats and show Architect's view in chat"
        >
          <Activity size={16} aria-hidden />
          Hardware Audit
        </button>
        <button
          type="button"
          onClick={handleManageVault}
          disabled={!onOpenSettings}
          className={QUICK_ACTION_CLASS}
          title="Open Secure Vault settings (OS keychain)"
        >
          <Key size={16} aria-hidden />
          Manage Vault
        </button>
        {chatState.lastMimirSummaryPath && (
          <button
            type="button"
            onClick={handleSynthesize}
            disabled={synthesizeLoading}
            className={QUICK_ACTION_CLASS}
            title="Cross-reference last meeting summary with current machine vitality (KB-Linker)"
          >
            {synthesizeLoading ? <Loader2 size={16} className="animate-spin" aria-hidden /> : <Activity size={16} aria-hidden />}
            {synthesizeLoading ? 'Synthesizing…' : 'Synthesize With Infrastructure'}
          </button>
        )}
      </div>
    </div>
  );
};

// Improved CodeBlock component with Copy functionality and enhanced styling
const CodeBlock = ({ node, inline, className, children, ...props }: any) => {
  const [isCopied, setIsCopied] = useState(false);
  const match = /language-(\w+)/.exec(className || '');
  const language = match ? match[1] : 'text';
  const codeString = String(children).replace(/\n$/, '');

  const handleCopy = async () => {
    if (!navigator.clipboard) return;
    try {
        await navigator.clipboard.writeText(codeString);
        setIsCopied(true);
        setTimeout(() => setIsCopied(false), 2000);
    } catch (e) {
        console.error("Failed to copy code", e);
    }
  };

  if (inline) {
    return (
      <code className="bg-zinc-200 dark:bg-zinc-800 px-1.5 py-0.5 rounded-md font-mono text-[0.85em] text-zinc-900 dark:text-zinc-100 border border-zinc-300 dark:border-zinc-700" {...props}>
        {children}
      </code>
    );
  }

  return (
    <div className="relative group my-4 rounded-lg overflow-hidden border border-zinc-200 dark:border-zinc-800 shadow-sm bg-zinc-50 dark:bg-[#1e1e1e]">
      {/* Code Block Header */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-100 dark:bg-[#252526] border-b border-zinc-200 dark:border-zinc-800 select-none">
         <span className="text-[10px] text-zinc-500 dark:text-zinc-400 font-mono font-medium lowercase">
            {language}
         </span>
         <button 
           onClick={handleCopy}
           className="flex items-center gap-1.5 text-[10px] text-zinc-500 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100 transition-colors px-1.5 py-0.5 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700"
           title="Copy code"
         >
           {isCopied ? (
             <>
               <Check size={12} className="text-emerald-500" />
               <span className="text-emerald-600 dark:text-emerald-400 font-medium">Copied!</span>
             </>
           ) : (
             <>
               <Copy size={12} />
               <span>Copy</span>
             </>
           )}
         </button>
      </div>
      <SyntaxHighlighter
        style={vscDarkPlus}
        language={language}
        PreTag="div"
        {...props}
        customStyle={{
          margin: 0,
          borderRadius: 0,
          padding: '1rem',
          fontSize: '0.85rem',
          lineHeight: '1.6',
          backgroundColor: 'transparent', 
        }}
        codeTagProps={{
            style: {
                fontFamily: "Menlo, Monaco, Consolas, 'Courier New', monospace",
            }
        }}
      >
        {codeString}
      </SyntaxHighlighter>
    </div>
  );
};

// Robust Avatar component to handle image loading errors
const Avatar = ({ url, role, fallbackIcon }: { url?: string, role: 'user' | 'agi', fallbackIcon: React.ReactNode }) => {
  const [error, setError] = useState(false);
  
  useEffect(() => setError(false), [url]);

  if (url && !error) {
    return (
        <img 
            src={url} 
            alt={role} 
            className="w-full h-full object-cover transition-opacity duration-300" 
            onError={() => setError(true)}
        />
    );
  }
  
  return (
    <div className={`w-full h-full flex items-center justify-center ${role === 'agi' ? 'bg-linear-to-br from-indigo-500 to-purple-600' : 'bg-zinc-200 dark:bg-zinc-700'}`}>
        {fallbackIcon}
    </div>
  );
};

// Helper function to detect and extract diagram envelopes from message content
interface DiagramEnvelope {
  type: 'diagram';
  format: 'mermaid' | 'dot';
  content: string;
  metadata?: {
    created_at?: string;
    kb_key?: string;
    title?: string;
  };
}

interface ParsedContent {
  hasDiagrams: boolean;
  diagrams: DiagramEnvelope[];
  textContent: string;
}

const parseDiagramEnvelopes = (content: string): ParsedContent => {
  const result: ParsedContent = {
    hasDiagrams: false,
    diagrams: [],
    textContent: content,
  };

  // Look for JSON diagram envelopes in the content
  // Pattern: { "type": "diagram", "format": "mermaid", "content": "...", ... }
  const jsonEnvelopeRegex = /\{[\s\n]*"type"[\s\n]*:[\s\n]*"diagram"[\s\n]*,[\s\S]*?\}/g;
  const matches = content.match(jsonEnvelopeRegex);

  if (matches && matches.length > 0) {
    result.hasDiagrams = true;
    let textContent = content;

    matches.forEach((match) => {
      try {
        const envelope = JSON.parse(match) as DiagramEnvelope;
        if (envelope.type === 'diagram' && envelope.content) {
          result.diagrams.push(envelope);
          // Remove the JSON envelope from text content
          textContent = textContent.replace(match, '');
        }
      } catch (e) {
        console.warn('Failed to parse diagram envelope:', e);
      }
    });

    result.textContent = textContent.trim();
  }

  return result;
};

const ChatInterface: React.FC<ChatInterfaceProps> = ({ messages, isLoading, isStreaming, streamingStatusMessage, onSendMessage, onTogglePin, settings, moeActive, onOpenSettings }) => {
  const [input, setInput] = useState('');
  const [inputError, setInputError] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const { state: chatState, actions: chatActions } = useChatStore();
  const [synthesizeLoading, setSynthesizeLoading] = useState(false);
  const [expandedThoughts, setExpandedThoughts] = useState<Record<string, boolean>>({});
  const [boundaryAlert, setBoundaryAlert] = useState<{ flagged: boolean; matchedTriggers: string[] }>({ flagged: false, matchedTriggers: [] });

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Debounced subject-check (KB-05): show Boundary Alert when input matches sovereignty_leak_triggers
  useEffect(() => {
    if (!input.trim()) {
      setBoundaryAlert({ flagged: false, matchedTriggers: [] });
      return;
    }
    const t = setTimeout(() => {
      const params = new URLSearchParams({ text: input.trim() });
      fetch(`${API_BASE_URL}/subject-check?${params}`)
        .then((r) => r.json())
        .then((data: { flagged?: boolean; matched_triggers?: string[] }) => {
          setBoundaryAlert({
            flagged: !!data.flagged,
            matchedTriggers: Array.isArray(data.matched_triggers) ? data.matched_triggers : [],
          });
        })
        .catch(() => setBoundaryAlert({ flagged: false, matchedTriggers: [] }));
    }, 400);
    return () => clearTimeout(t);
  }, [input]);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 150)}px`;
    }
  }, [input]);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, isLoading, isStreaming]);

  const handleSubmit = (e?: React.FormEvent) => {
    e?.preventDefault();
    if (isLoading || isStreaming) return;
    
    if (!input.trim()) {
      setInputError('Message cannot be empty.');
      return;
    }

    onSendMessage(input);
    setInput('');
    setInputError(null);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const copyToClipboard = async (id: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const toggleThoughts = (id: string) => {
    setExpandedThoughts(prev => ({
        ...prev,
        [id]: !(prev[id] ?? true)
    }));
  };

  const handleSynthesizeFromBar = async () => {
    const path = chatState.lastMimirSummaryPath;
    if (!path) return;
    setSynthesizeLoading(true);
    try {
      const activeThread = chatState.threads.find((t) => t.id === chatState.activeThreadId);
      const projectId = activeThread?.projectId ?? null;
      const result = await postMimirSynthesize(path, projectId);
      const alertLines =
        result.alerts.length > 0
          ? result.alerts.map((a) => `- **${a.kind}:** ${a.message}`).join('\n')
          : '';
      const content = `**Sovereign Vitality × Meeting Context**\n\n${result.summary}\n\n${alertLines ? `### Alerts\n${alertLines}\n\n` : ''}\n\`\`\`mermaid\n${result.mermaid}\n\`\`\``;
      if (chatState.activeThreadId) {
        chatActions.addMessage(chatState.activeThreadId, {
          id: `synthesize-${Date.now()}`,
          role: 'agi',
          content,
          timestamp: Date.now(),
        });
      }
    } catch (e) {
      console.error('Synthesize failed:', e);
    } finally {
      setSynthesizeLoading(false);
    }
  };

  const markdownComponents = useMemo(() => ({
    code: CodeBlock,
    p: ({node, ...props}: any) => <p {...props} className="mb-3 last:mb-0 leading-relaxed" />,
    a: ({node, ...props}: any) => <a {...props} className="text-blue-500 hover:text-blue-600 hover:underline" target="_blank" rel="noopener noreferrer" />,
    ul: ({node, ...props}: any) => <ul {...props} className="list-disc pl-5 mb-3 space-y-1" />,
    ol: ({node, ...props}: any) => <ol {...props} className="list-decimal pl-5 mb-3 space-y-1" />,
    li: ({node, ...props}: any) => <li {...props} className="pl-1" />,
    strong: ({node, ...props}: any) => <strong {...props} className="font-semibold text-zinc-900 dark:text-zinc-100" />,
    em: ({node, ...props}: any) => <em {...props} className="italic text-zinc-800 dark:text-zinc-200" />,
    del: ({node, ...props}: any) => <del {...props} className="line-through text-zinc-400 dark:text-zinc-500" />,
    h1: ({node, ...props}: any) => <h1 {...props} className="text-xl font-bold mt-4 mb-2 pb-1 border-b border-zinc-200 dark:border-zinc-700" />,
    h2: ({node, ...props}: any) => <h2 {...props} className="text-lg font-bold mt-3 mb-2" />,
    h3: ({node, ...props}: any) => <h3 {...props} className="text-md font-bold mt-2 mb-1" />,
    blockquote: ({node, ...props}: any) => <blockquote {...props} className="border-l-4 border-zinc-300 dark:border-zinc-700 pl-4 italic text-zinc-600 dark:text-zinc-400 my-3" />,
    table: ({node, ...props}: any) => <div className="overflow-x-auto my-4 rounded-lg border border-zinc-200 dark:border-zinc-700"><table {...props} className="w-full text-sm text-left" /></div>,
    thead: ({node, ...props}: any) => <thead {...props} className="bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 uppercase font-medium" />,
    tbody: ({node, ...props}: any) => <tbody {...props} className="divide-y divide-zinc-200 dark:divide-zinc-700" />,
    tr: ({node, ...props}: any) => <tr {...props} className="hover:bg-zinc-50 dark:hover:bg-zinc-800/50 transition-colors" />,
    th: ({node, ...props}: any) => <th {...props} className="px-4 py-3 border-b border-zinc-200 dark:border-zinc-700" />,
    td: ({node, ...props}: any) => <td {...props} className="px-4 py-3" />,
    img: ({node, ...props}: any) => <img {...props} className="rounded-lg max-w-full h-auto my-2 border border-zinc-200 dark:border-zinc-700" />,
    hr: ({node, ...props}: any) => <hr {...props} className="my-6 border-zinc-200 dark:border-zinc-700" />,
  }), []);

  return (
    <div className="flex flex-col h-full max-w-4xl mx-auto w-full relative">
      {/* Messages Area */}
      <div className="flex-1 overflow-y-auto px-4 py-6 space-y-6">
        {messages.length === 0 && (
          <EmptyStateGreeting onSendMessage={onSendMessage} onOpenSettings={onOpenSettings} />
        )}

        {messages.map((msg, index) => (
          <div key={msg.id} className={`flex gap-4 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
            {msg.role === 'agi' && (
              <div className="w-8 h-8 rounded-full bg-white dark:bg-zinc-800 flex items-center justify-center shrink-0 border border-zinc-200 dark:border-zinc-700 transition-colors overflow-hidden">
                <Avatar 
                    url={settings.agiAvatar} 
                    role="agi" 
                    fallbackIcon={<Bot size={16} className="text-white drop-shadow-md" />} 
                />
              </div>
            )}
            
            <div className={`max-w-[85%] ${msg.role === 'user' ? 'items-end' : 'items-start'} flex flex-col`}>
              {/* User Alias Display */}
              {msg.role === 'user' && settings.userAlias && (
                <span className="text-[10px] text-zinc-500 dark:text-zinc-400 mb-1 px-1 font-mono uppercase tracking-wider">
                  {settings.userAlias}
                </span>
              )}
              {/* Phoenix name display (short identity in chat) */}
              {msg.role === 'agi' && (
                <span className="text-[10px] text-zinc-500 dark:text-zinc-400 mb-1 px-1 font-mono uppercase tracking-wider flex items-center gap-1.5" title={settings.sovereignProtocols ? 'Phoenix Security – Domain Protection Active (KB-05)' : undefined}>
                  Phoenix
                  {settings.sovereignProtocols && (
                    <Shield
                      size={12}
                      className="text-emerald-600 dark:text-emerald-400"
                      aria-label="Phoenix Security (KB-05)"
                    />
                  )}
                </span>
              )}

              <div 
                className={`px-4 py-3 rounded-lg text-sm shadow-sm relative group transition-colors overflow-hidden
                  ${msg.role === 'user' 
                    ? 'bg-zinc-200 dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 border border-zinc-300 dark:border-zinc-700 rounded-tr-none' 
                    : 'bg-white dark:bg-zinc-900/50 text-zinc-800 dark:text-zinc-300 border border-zinc-200 dark:border-zinc-800/80 rounded-tl-none pr-10'
                  } 
                  ${msg.isError ? 'border-red-200 dark:border-red-900/50 bg-red-50 dark:bg-red-950/10 text-red-800 dark:text-red-200' : ''}
                  ${msg.isPinned && !msg.isError ? 'border-orange-200 dark:border-orange-900/50 bg-orange-50/30 dark:bg-orange-900/10' : ''}
                `}
              >
                {!msg.isError ? (
                  <>
                    {/* MoE Expert Routing indicator (Gater chose local expert) */}
                    {msg.role === 'agi' && msg.expertRouting && (
                      <div className="mb-1.5 inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide bg-amber-500/15 text-amber-700 dark:bg-amber-400/20 dark:text-amber-300 border border-amber-400/30 dark:border-amber-500/30">
                        <span className="opacity-90">Expert:</span>
                        <span>{msg.expertRouting}</span>
                      </div>
                    )}
                    {(() => {
                      const parsed = parseDiagramEnvelopes(msg.content);
                      return (
                        <>
                          {/* Render text content */}
                          {parsed.textContent && (
                            <ReactMarkdown
                                remarkPlugins={[remarkGfm]}
                                components={markdownComponents}
                            >
                                {parsed.textContent}
                            </ReactMarkdown>
                          )}
                          {/* Render diagrams */}
                          {parsed.diagrams.map((diagram, idx) => (
                            <DiagramViewer
                              key={`${msg.id}-diagram-${idx}`}
                              diagram={diagram}
                              className="mt-4"
                            />
                          ))}
                        </>
                      );
                    })()}
                    {/* Streaming Indicator — High-Signal Progress: phase-based status with fade on change */}
                    {msg.role === 'agi' && isStreaming && index === messages.length - 1 && (
                        <div className="mt-2 flex items-center gap-2 text-zinc-400 dark:text-zinc-500 animate-pulse">
                            <span className="w-1.5 h-1.5 bg-current rounded-full shrink-0" />
                            <span
                                key={streamingStatusMessage ?? '__default'}
                                className="text-[10px] font-mono uppercase tracking-widest status-message-transition"
                            >
                                {streamingStatusMessage ?? 'Phoenix is processing…'}
                            </span>
                        </div>
                    )}
                  </>
                ) : (
                  msg.content
                )}

                {/* Message Actions (Toggle Thoughts, Pin, Copy) */}
                {msg.role === 'agi' && !msg.isError && (
                  <div className={`absolute top-2 right-2 flex items-center gap-1 transition-opacity duration-200 ${
                      msg.isPinned ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
                  }`}>
                     {/* Toggle Thoughts Button */}
                     {settings.showThoughts && msg.thoughts && msg.thoughts.length > 0 && (
                        <button
                            type="button"
                            onClick={() => toggleThoughts(msg.id)}
                            aria-label={(expandedThoughts[msg.id] ?? true) ? "Collapse thoughts" : "Expand thoughts"}
                            className={`p-1.5 rounded-md transition-all border focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-1 ${
                                (expandedThoughts[msg.id] ?? true)
                                ? 'bg-indigo-100 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-800'
                                : 'bg-zinc-100/80 dark:bg-zinc-800/80 text-zinc-400 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 border-transparent hover:border-zinc-300 dark:hover:border-zinc-600'
                            }`}
                        >
                            <BrainCircuit size={14} className={(expandedThoughts[msg.id] ?? true) ? "fill-current" : ""} aria-hidden />
                        </button>
                     )}
                     
                     <button
                        type="button"
                        onClick={() => onTogglePin(msg.id)}
                        aria-label={msg.isPinned ? "Unpin message" : "Pin message"}
                        className={`p-1.5 rounded-md transition-all border focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-1 ${
                            msg.isPinned 
                            ? 'bg-orange-100 dark:bg-orange-900/30 text-orange-600 dark:text-orange-400 border-orange-200 dark:border-orange-800'
                            : 'bg-zinc-100/80 dark:bg-zinc-800/80 text-zinc-400 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 border-transparent hover:border-zinc-300 dark:hover:border-zinc-600'
                        }`}
                      >
                        <Pin size={14} className={msg.isPinned ? "fill-current" : ""} aria-hidden />
                      </button>
                      <button
                        type="button"
                        onClick={() => copyToClipboard(msg.id, msg.content)}
                        aria-label={copiedId === msg.id ? "Copied" : "Copy message"}
                        className="p-1.5 rounded-md bg-zinc-100/80 dark:bg-zinc-800/80 hover:bg-zinc-200 dark:hover:bg-zinc-700 text-zinc-400 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 transition-all border border-transparent hover:border-zinc-300 dark:hover:border-zinc-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-1"
                      >
                        {copiedId === msg.id ? (
                          <Check size={14} className="text-emerald-500 dark:text-emerald-400" aria-hidden />
                        ) : (
                          <Copy size={14} aria-hidden />
                        )}
                      </button>
                  </div>
                )}
              </div>
              
              {/* Render Thoughts if AGI and enabled */}
              {msg.role === 'agi' && settings.showThoughts && msg.thoughts && (
                <div className="w-full mt-1">
                  <ThoughtBlock 
                    thoughts={msg.thoughts} 
                    isExpanded={expandedThoughts[msg.id] ?? true}
                    onToggle={() => toggleThoughts(msg.id)}
                  />
                </div>
              )}
              
              <span className="text-[10px] text-zinc-400 dark:text-zinc-600 mt-1 px-1">
                {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
            </div>

            {msg.role === 'user' && (
              <div className="w-8 h-8 rounded-full bg-zinc-200 dark:bg-zinc-300 flex items-center justify-center shrink-0 overflow-hidden border border-zinc-300 dark:border-zinc-600">
                <Avatar 
                    url={settings.userAvatar} 
                    role="user" 
                    fallbackIcon={<User size={16} className="text-zinc-700 dark:text-zinc-900" />} 
                />
              </div>
            )}
          </div>
        ))}

        {isLoading && (
          <div className="flex gap-4 justify-start animate-pulse">
            <div className="w-8 h-8 rounded-full bg-white dark:bg-zinc-800 flex items-center justify-center border border-zinc-200 dark:border-zinc-700 overflow-hidden">
               <Avatar 
                   url={settings.agiAvatar} 
                   role="agi" 
                   fallbackIcon={<Bot size={16} className="text-white drop-shadow-md" />} 
                />
            </div>
            <div className="bg-white/50 dark:bg-zinc-900/30 px-4 py-3 rounded-lg border border-zinc-200 dark:border-zinc-800/50 flex items-center gap-2">
              <span className="w-2 h-2 bg-zinc-400 dark:bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: '0ms' }}/>
              <span className="w-2 h-2 bg-zinc-400 dark:bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: '150ms' }}/>
              <span className="w-2 h-2 bg-zinc-400 dark:bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: '300ms' }}/>
              {streamingStatusMessage && (
                <span
                  key={streamingStatusMessage}
                  className="text-[10px] font-mono uppercase tracking-widest text-zinc-500 dark:text-zinc-400 ml-1 status-message-transition"
                >
                  {streamingStatusMessage}
                </span>
              )}
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Synthesize With Infrastructure — shown after a meeting when summary path is set */}
      {messages.length > 0 && chatState.lastMimirSummaryPath && (
        <div className="px-4 py-2 bg-amber-500/5 dark:bg-amber-500/10 border-t border-amber-500/20 flex items-center justify-center">
          <button
            type="button"
            onClick={handleSynthesizeFromBar}
            disabled={synthesizeLoading}
            className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium border border-amber-500/30 bg-amber-500/10 hover:bg-amber-500/20 text-amber-800 dark:text-amber-200 transition-colors disabled:opacity-50"
            title="Cross-reference last meeting summary with current machine vitality"
          >
            {synthesizeLoading ? <Loader2 size={14} className="animate-spin" aria-hidden /> : <Activity size={14} aria-hidden />}
            {synthesizeLoading ? 'Synthesizing…' : 'Synthesize With Infrastructure'}
          </button>
        </div>
      )}

      {/* Input Area */}
      <div className="p-4 bg-white/80 dark:bg-zinc-950/80 backdrop-blur-md border-t border-zinc-200 dark:border-zinc-800/50 sticky bottom-0 z-10 transition-colors">
        <div className="max-w-4xl mx-auto relative">
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => {
              setInput(e.target.value);
              if (inputError) setInputError(null);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Talk with Phoenix..."
            aria-label="Message Phoenix"
            aria-invalid={!!inputError}
            aria-describedby={inputError ? 'input-error-msg' : undefined}
            className={`w-full bg-zinc-50 dark:bg-zinc-900 border text-zinc-900 dark:text-zinc-200 text-sm rounded-xl px-4 py-3 pr-12 focus:outline-none focus-visible:ring-2 transition-all resize-none max-h-[150px] placeholder:text-zinc-400 dark:placeholder:text-zinc-600
              ${inputError 
                ? 'border-red-400 dark:border-red-500/80 focus-visible:border-red-500 focus-visible:ring-red-500/30' 
                : 'border-zinc-300 dark:border-zinc-700 focus-visible:border-blue-500 dark:focus-visible:border-blue-400 focus-visible:ring-blue-500/20'
              }
            `}
            rows={1}
          />
          <button
            type="button"
            onClick={() => handleSubmit()}
            disabled={isLoading || isStreaming}
            aria-label="Send message"
            className={`absolute right-2 bottom-2 p-2 rounded-lg transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 disabled:opacity-40 disabled:pointer-events-none
               ${inputError 
                 ? 'text-red-400 hover:text-red-600 dark:text-red-500 dark:hover:text-red-400 hover:bg-red-500/10' 
                 : 'text-zinc-500 dark:text-zinc-400 hover:text-orange-500 dark:hover:text-orange-400 hover:bg-orange-500/10'
               }`}
          >
            <Send size={18} aria-hidden />
          </button>
        </div>
        <div className="max-w-4xl mx-auto mt-2 flex justify-between items-center px-1">
            <div className="flex items-center gap-3 flex-wrap">
              <p className="text-[10px] text-zinc-400 dark:text-zinc-600 font-mono">
                  CONNECTED: 127.0.0.1:8000/api/v1/chat
                  {moeActive ? ' · MODE: MoE Sparse' : ''}
              </p>
              {boundaryAlert.flagged && (
                <SubjectProfile
                  subjectName="Current message"
                  flaggedByAstroLogic
                  matchedTriggers={boundaryAlert.matchedTriggers}
                  className="text-[10px] text-amber-600 dark:text-amber-400"
                />
              )}
              {inputError && (
                <div id="input-error-msg" className="flex items-center gap-1 text-[10px] text-red-500 dark:text-red-400 animate-in fade-in slide-in-from-left-1" role="alert">
                  <AlertCircle size={10} aria-hidden />
                  <span>{inputError}</span>
                </div>
              )}
            </div>
            <span className={`text-[10px] font-mono opacity-60 transition-colors ${inputError ? 'text-red-400' : 'text-zinc-400 dark:text-zinc-600'}`}>
                {input.length} chars
            </span>
        </div>
      </div>
    </div>
  );
};

export default ChatInterface;