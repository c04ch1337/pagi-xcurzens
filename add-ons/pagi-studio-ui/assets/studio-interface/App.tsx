import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Settings, Boxes, Pin, Brain, MessageSquare, Activity, FileBarChart } from 'lucide-react';
import ChatInterface from './components/ChatInterface';
import SettingsSidebar from './components/SettingsSidebar';
import PinnedSidebar from './components/PinnedSidebar';
import SystemHealth from './components/SystemHealth';
import ChronosAuditLog from './components/ChronosAuditLog';
import BalanceCheckModal from './components/BalanceCheckModal';
import ChatHistorySidebar from './components/ChatHistorySidebar';
import WellnessTab from './components/WellnessTab';
import SovereignReport from './components/SovereignReport';
import OnboardingOverlay from './components/OnboardingOverlay';
import { Message, AppSettings, ApiResponse, GatewayFeatureConfig, WellnessReport, OnboardingState } from './types';
import { sendMessageToOrchestrator, streamMessageToOrchestrator, type MilestoneSuggestPayload } from './services/apiService';
import { API_BASE_URL } from './src/api/config';
import { ChatProvider, useChatStore } from './src/stores/ChatStore';

const AppInner: React.FC = () => {
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [isPinnedSidebarOpen, setIsPinnedSidebarOpen] = useState(false);
  const [isAuditLogOpen, setIsAuditLogOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isStreaming, setIsStreaming] = useState(false);
  const [gatewayConfig, setGatewayConfig] = useState<GatewayFeatureConfig | null>(null);
  const [balanceModalOpen, setBalanceModalOpen] = useState(false);
  const [balanceModalMessage, setBalanceModalMessage] = useState<string>('');
  const [sovereignResetToast, setSovereignResetToast] = useState<{ message?: string; healthReminder?: string } | null>(null);
  const [stressVitalityToast, setStressVitalityToast] = useState<boolean>(false);
  // velocity score and sentinel status were previously used by Vitality/Astro widgets (removed).
  const [forcedResetCountdown, setForcedResetCountdown] = useState<number | null>(null);
  const [activeView, setActiveView] = useState<'chat' | 'wellness' | 'briefing'>('chat');
  const [wellnessReport, setWellnessReport] = useState<WellnessReport | null>(null);
  const wellnessReportRef = useRef<WellnessReport | null>(null);
  const wellnessFetchedForToastRef = useRef<boolean>(false);
  const personaStreamRef = useRef<EventSource | null>(null);
  const [onboardingState, setOnboardingState] = useState<OnboardingState | null>(null);
  const [onboardingDismissed, setOnboardingDismissed] = useState(false);
  const [milestoneSuggest, setMilestoneSuggest] = useState<MilestoneSuggestPayload | null>(null);
  /** Strategic Timing Phase 2: status line while Architect is "thinking" (e.g. "Architect Analyzing...", "Auditing Perimeter..."). */
  const [streamingStatusMessage, setStreamingStatusMessage] = useState<string | null>(null);

  // Keep wellnessReportRef in sync with wellnessReport state
  useEffect(() => {
    wellnessReportRef.current = wellnessReport;
  }, [wellnessReport]);

  const fetchConfig = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/config`);
      if (res.ok) {
        const data = await res.json();
        setGatewayConfig(data as GatewayFeatureConfig);
      }
    } catch {
      setGatewayConfig(null);
    }
  }, []);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  // Phoenix Marie onboarding: fetch status when gateway is reachable; show overlay if needs_onboarding and no history
  const fetchOnboardingStatus = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/onboarding/status`);
      if (!res.ok) return;
      const data = (await res.json()) as OnboardingState;
      setOnboardingState(data);
    } catch {
      // Gateway not reachable or not configured; skip onboarding
    }
  }, []);

  useEffect(() => {
    fetchOnboardingStatus();
  }, [fetchOnboardingStatus]);

  // Persona stream: 4-hour heartbeat + sovereign reset suggestion
  useEffect(() => {
    const es = new EventSource(`${API_BASE_URL}/persona/stream`);
    personaStreamRef.current = es;
    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === 'persona_heartbeat' && data.message) {
          setBalanceModalMessage(data.message);
          setBalanceModalOpen(true);
        }
        if (data.type === 'sentinel_update') {
          const score = typeof data.velocity_score === 'number' ? data.velocity_score : null;
          const highVelocity = (score != null && score >= 60) || !!data.is_rage_detected;
          if (highVelocity) {
            if (wellnessReportRef.current?.is_critical) {
              setStressVitalityToast(true);
              setTimeout(() => setStressVitalityToast(false), 8000);
            }
            if (!wellnessFetchedForToastRef.current) {
              wellnessFetchedForToastRef.current = true;
              fetch(`${API_BASE_URL}/skills/wellness-report`)
                .then(r => r.json().catch(() => ({})))
                .then(payload => {
                  if (payload?.status === 'ok' && payload?.report) {
                    const report = payload.report as WellnessReport;
                    setWellnessReport(report);
                    if (report.is_critical) {
                      setStressVitalityToast(true);
                      setTimeout(() => setStressVitalityToast(false), 8000);
                    }
                  }
                })
                .catch(() => {});
            }
          }
        }
        if (data.sovereign_reset_suggested) {
          setSovereignResetToast({
            message: data.message,
            healthReminder: data.health_reminder,
          });
          setTimeout(() => setSovereignResetToast(null), 8000);
        }
        if (data.type === 'forced_reset_countdown') {
          setForcedResetCountdown(10);
        }
      } catch {
        // ignore parse errors
      }
    };
    es.onerror = () => {};
    return () => {
      es.close();
      personaStreamRef.current = null;
    };
  }, []);

  // Countdown timer for forced reset
  useEffect(() => {
    if (forcedResetCountdown === null || forcedResetCountdown <= 0) return;
    
    const timer = setInterval(() => {
      setForcedResetCountdown(prev => {
        if (prev === null || prev <= 1) {
          return null;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, [forcedResetCountdown]);

  // Initialize settings from localStorage if available
  // Sovereign architecture: Gateway is hard-locked to port 8000. UI must point only to 8000.
  const GATEWAY_API_URL = 'http://127.0.0.1:8000/api/v1/chat';

  const [settings, setSettings] = useState<AppSettings>(() => {
    const savedSettings = localStorage.getItem('agi_settings');
    if (savedSettings) {
      try {
        const parsed = JSON.parse(savedSettings);
        // Ensure theme exists for migration
        if (!parsed.theme) parsed.theme = 'dark';
        // Ensure userAlias exists for migration
        if (!parsed.userAlias) parsed.userAlias = 'User';
        // Ensure LLM settings exist for migration
        if (!parsed.llmModel) parsed.llmModel = 'deepseek/deepseek-v3.2';
        // Migration: replace invalid OpenRouter model IDs.
        if (parsed.llmModel === 'llama3-70b-8192') parsed.llmModel = 'meta-llama/llama-3.3-70b-instruct:free';
        if (parsed.llmTemperature === undefined) parsed.llmTemperature = 0.7;
        if (!parsed.llmMaxTokens) parsed.llmMaxTokens = 8192;
        if (!parsed.orchestratorPersona) parsed.orchestratorPersona = 'general_assistant';
        if (parsed.preferredWorkspacePath === undefined) parsed.preferredWorkspacePath = '';

        // Enforce port 8000 only (any other host/port is overwritten)
        parsed.apiUrl = GATEWAY_API_URL;

        return parsed;
      } catch (e) {
        console.error("Failed to parse settings", e);
      }
    }
    return {
      apiUrl: GATEWAY_API_URL,
      stream: true,
      showThoughts: true,
      userAlias: 'User',
      theme: 'dark',
      llmModel: 'deepseek/deepseek-v3.2',
      llmTemperature: 0.7,
      llmMaxTokens: 8192,
      orchestratorPersona: 'general_assistant',
      preferredWorkspacePath: '',
    };
  });

  // Save settings to localStorage whenever they change
  useEffect(() => {
    try {
      const toSave = { ...settings, apiUrl: GATEWAY_API_URL };
      localStorage.setItem('agi_settings', JSON.stringify(toSave));
    } catch (e) {
      console.error("Failed to save settings to localStorage", e);
    }
  }, [settings]);

  // Apply Theme
  useEffect(() => {
    if (settings.theme === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [settings.theme]);

  // Effect to handle favicon updates (Robust Handler)
  useEffect(() => {
    const updateFavicon = (url: string) => {
      // Remove any existing favicon links to prevent conflicts
      const existingLinks = document.querySelectorAll("link[rel*='icon']");
      existingLinks.forEach(link => link.remove());
      
      // Create and append the new favicon link
      const link = document.createElement('link');
      link.type = 'image/x-icon';
      link.rel = 'shortcut icon';
      link.href = url;
      document.head.appendChild(link);
    };

    const defaultFavicon = 'data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22><text y=%22.9em%22 font-size=%2290%22>🤖</text></svg>';
    updateFavicon(settings.customFavicon || defaultFavicon);
  }, [settings.customFavicon]);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isModifier = e.metaKey || e.ctrlKey;

      // Toggle Settings: Ctrl+S / Cmd+S
      if (isModifier && (e.key === 's' || e.key === 'S')) {
        e.preventDefault(); // Prevent Save dialog
        setIsSidebarOpen(prev => !prev);
        // Ensure other sidebar is closed if we're opening this one
        if (!isSidebarOpen) setIsPinnedSidebarOpen(false);
      }

      // Toggle Pinned: Ctrl+P / Cmd+P
      if (isModifier && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault(); // Prevent Print dialog
        setIsPinnedSidebarOpen(prev => !prev);
        // Ensure other sidebar is closed if we're opening this one
        if (!isPinnedSidebarOpen) setIsSidebarOpen(false);
      }

      // Close Sidebars: Escape
      if (e.key === 'Escape') {
        setIsSidebarOpen(false);
        setIsPinnedSidebarOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isSidebarOpen, isPinnedSidebarOpen]);

  const { state: chatState, actions: chatActions } = useChatStore();
  const activeThread = chatState.threads.find(t => t.id === chatState.activeThreadId) ?? chatState.threads[0];
  const messages: Message[] = activeThread?.messages ?? [];

  const showOnboarding = Boolean(
    onboardingState?.needs_onboarding && messages.length === 0 && !onboardingDismissed
  );

  const handleOnboardingComplete = useCallback(async () => {
    try {
      await fetch(`${API_BASE_URL}/onboarding/complete`, { method: 'POST' });
    } catch {
      // ignore
    }
    setOnboardingDismissed(true);
    if (onboardingState?.phase1_greeting) {
      const handshakeMsg: Message = {
        id: `onboarding-${Date.now()}`,
        role: 'agi',
        content: `${onboardingState.phase1_greeting}\n\n— **Phoenix Marie**`,
        timestamp: Date.now(),
      };
      if (activeThread?.id) {
        chatActions.clearThread(activeThread.id);
        chatActions.addMessage(activeThread.id, handshakeMsg);
      }
    }
  }, [onboardingState?.phase1_greeting, activeThread?.id, chatActions]);

  const togglePin = (messageId: string) => {
    if (!activeThread?.id) return;
    const msg = messages.find(m => m.id === messageId);
    if (!msg) return;
    chatActions.updateMessage(activeThread.id, messageId, { isPinned: !msg.isPinned });
  };

  const handleClearChat = () => {
    // Keeping the existing confirmation semantics, but now we create a new thread.
    if (window.confirm('Start a new chat thread? The current thread will remain in history.')) {
      chatActions.newThread({ title: 'New Chat' });
    }
  };

  const handleSendMessage = async (text: string) => {
    if (!activeThread?.id) {
      chatActions.newThread({ title: 'New Chat' });
      return;
    }
    const userMsg: Message = {
      id: Date.now().toString(),
      role: 'user',
      content: text,
      timestamp: Date.now(),
    };

    chatActions.addMessage(activeThread.id, userMsg);
    setIsLoading(true);
    setIsStreaming(true);

    try {
      if (settings.stream) {
        const agiMsgId = (Date.now() + 1).toString();
        let accumulatedResponse = '';
        let hasCreatedMessage = false;
        let expertRouting: string | undefined;

        const stream = streamMessageToOrchestrator(
          text,
          settings,
          activeThread?.projectId ?? null,
          activeThread?.id ?? null
        );

        for await (const chunk of stream) {
          if (typeof chunk === 'object' && chunk !== null && '__expertRouting' in chunk) {
            expertRouting = chunk.__expertRouting;
            continue;
          }
          if (typeof chunk === 'object' && chunk !== null && '__milestoneSuggest' in chunk) {
            setMilestoneSuggest(chunk.__milestoneSuggest);
            continue;
          }
          if (typeof chunk === 'object' && chunk !== null && '__streamingStatus' in chunk) {
            setStreamingStatusMessage(chunk.__streamingStatus);
            continue;
          }
          const str = typeof chunk === 'string' ? chunk : String(chunk);
          accumulatedResponse += str;

          if (!hasCreatedMessage) {
            setIsLoading(false);
            const agiMsg: Message = {
              id: agiMsgId,
              role: 'agi',
              content: accumulatedResponse,
              timestamp: Date.now(),
              ...(expertRouting ? { expertRouting } : {}),
            };
            chatActions.addMessage(activeThread.id, agiMsg);
            hasCreatedMessage = true;
          } else {
            chatActions.updateMessage(activeThread.id, agiMsgId, {
              content: accumulatedResponse,
              ...(expertRouting !== undefined ? { expertRouting } : {}),
            });
          }
        }

        setStreamingStatusMessage(null);
        setIsStreaming(false);

        // If stream finished but no content was received (rare)
        if (!hasCreatedMessage) {
           setIsLoading(false);
        }

      } else {
        const data: ApiResponse = await sendMessageToOrchestrator(
          text,
          settings,
          activeThread?.projectId ?? null,
          activeThread?.id ?? null
        );

        const agiMsg: Message = {
          id: (Date.now() + 1).toString(),
          role: 'agi',
          content: data.response,
          thoughts: data.thoughts,
          timestamp: Date.now(),
          ...(data.expert_routing ? { expertRouting: data.expert_routing } : {}),
        };

        chatActions.addMessage(activeThread.id, agiMsg);
        setStreamingStatusMessage(null);
        setIsLoading(false);
        setIsStreaming(false);
      }
    } catch (error) {
      console.error(error);
      const errorMsg: Message = {
        id: (Date.now() + 1).toString(),
        role: 'agi',
        content: `Connection Error: Failed to reach http://127.0.0.1:8000/api/v1/chat. Ensure the Gateway is running.`,
        isError: true,
        timestamp: Date.now(),
      };
      chatActions.addMessage(activeThread.id, errorMsg);
      setStreamingStatusMessage(null);
      setIsLoading(false);
      setIsStreaming(false);
    }
  };

  const handleMilestoneDocumentNow = useCallback(async () => {
    if (!milestoneSuggest) return;
    const thread = chatState.threads.find(t => t.id === chatState.activeThreadId);
    if (!thread || thread.messages.length === 0) {
      setMilestoneSuggest(null);
      return;
    }
    try {
      const res = await fetch(`${API_BASE_URL}/projects/document-session`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          project_id: milestoneSuggest.project_id,
          title: thread.title,
          messages: thread.messages.map(m => ({
            role: m.role,
            content: m.content,
            thoughts: m.thoughts?.map(t => ({ title: t.title, content: t.content })),
          })),
        }),
      });
      const data = await res.json().catch(() => ({}));
      if (res.ok && data.status === 'ok') {
        setMilestoneSuggest(null);
      }
    } catch {
      setMilestoneSuggest(null);
    }
  }, [milestoneSuggest, chatState.threads, chatState.activeThreadId]);

  return (
    <>
      {settings.customCss && (
        <style dangerouslySetInnerHTML={{ __html: settings.customCss }} />
      )}
      {showOnboarding && onboardingState && (
        <OnboardingOverlay
          state={onboardingState}
          onComplete={handleOnboardingComplete}
          onProfileSaved={fetchOnboardingStatus}
        />
      )}
      {milestoneSuggest && (
        <div
          className="fixed bottom-20 left-1/2 -translate-x-1/2 z-50 max-w-md w-[calc(100%-2rem)] rounded-lg border border-emerald-500/40 bg-emerald-50 dark:bg-emerald-950/90 dark:border-emerald-600/50 shadow-lg p-3 flex flex-col gap-2"
          role="alert"
          aria-live="polite"
        >
          <p className="text-sm text-zinc-800 dark:text-zinc-200">{milestoneSuggest.message}</p>
          <div className="flex gap-2 justify-end">
            <button
              type="button"
              onClick={() => setMilestoneSuggest(null)}
              className="px-3 py-1.5 rounded text-sm font-medium text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200 dark:hover:bg-zinc-800"
            >
              Dismiss
            </button>
            <button
              type="button"
              onClick={handleMilestoneDocumentNow}
              className="px-3 py-1.5 rounded text-sm font-medium bg-emerald-600 text-white hover:bg-emerald-700"
            >
              Document now
            </button>
          </div>
        </div>
      )}
      <div
        className={`h-screen w-full bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-200 flex flex-col font-sans overflow-hidden transition-colors duration-300 ${
          (gatewayConfig?.orchestrator_role ?? gatewayConfig?.persona_mode) === 'counselor' ? 'role-counselor' : 'role-companion'
        }`}
      >
        {/* Header */}
        <header className="h-14 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between px-4 sm:px-6 bg-white/80 dark:bg-zinc-950/80 backdrop-blur-sm z-20 shrink-0 transition-colors duration-300">
          <div className="flex items-center gap-3 min-w-0">
            {settings.customLogo ? (
              <img 
                src={settings.customLogo} 
                alt="Phoenix Sovereign Core" 
                className="h-8 w-auto object-contain rounded-sm shrink-0" 
              />
            ) : (
              <div className="bg-zinc-100 dark:bg-zinc-800 p-1.5 rounded-md text-orange-500 dark:text-orange-400 shrink-0" aria-hidden>
                <Boxes size={18} />
              </div>
            )}
            <span className="font-semibold text-sm tracking-wide text-zinc-900 dark:text-zinc-100 truncate">Phoenix Sovereign Core</span>
            <span className="text-xs text-zinc-500 dark:text-zinc-600 bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 px-2 py-0.5 rounded-full shrink-0 hidden sm:inline">Counselor-Architect</span>
          </div>
          
          <nav className="flex items-center gap-1 sm:gap-2" aria-label="App actions">
            {/* New Chat moved to left sidebar (Project-based history). */}
            <button 
              onClick={() => setIsPinnedSidebarOpen(true)}
              className="p-2 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-md transition-colors relative focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-zinc-950"
              title="Pinned Messages (Ctrl+P)"
              aria-label="Open pinned messages"
            >
              <Pin size={20} aria-hidden />
              {messages.some(m => m.isPinned) && (
                <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-orange-500 rounded-full border-2 border-white dark:border-zinc-950" aria-hidden />
              )}
            </button>
            <button
              onClick={() => setIsAuditLogOpen(!isAuditLogOpen)}
              className="p-2 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-zinc-950"
              title="Maintenance Audit Log"
              aria-label="Open maintenance audit log"
            >
              <Brain size={20} aria-hidden />
            </button>
            <button
              onClick={() => setActiveView(activeView === 'wellness' ? 'chat' : 'wellness')}
              className={`p-2 rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-zinc-950 ${
                activeView === 'wellness'
                  ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-500/15 dark:bg-emerald-500/20'
                  : 'text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800'
              }`}
              title={activeView === 'wellness' ? 'Back to Chat' : 'Wellness (7-day Soma)'}
              aria-label={activeView === 'wellness' ? 'Back to chat' : 'Open wellness view'}
              aria-pressed={activeView === 'wellness'}
            >
              {activeView === 'wellness' ? <MessageSquare size={20} aria-hidden /> : <Activity size={20} aria-hidden />}
            </button>
            <button
              onClick={() => setActiveView(activeView === 'briefing' ? 'chat' : 'briefing')}
              className={`p-2 rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-zinc-950 ${
                activeView === 'briefing'
                  ? 'text-emerald-600 dark:text-emerald-400 bg-emerald-500/15 dark:bg-emerald-500/20'
                  : 'text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800'
              }`}
              title={activeView === 'briefing' ? 'Back to Chat' : 'Sovereign Health Report (Briefing)'}
              aria-label={activeView === 'briefing' ? 'Back to chat' : 'Open sovereign briefing'}
              aria-pressed={activeView === 'briefing'}
            >
              <FileBarChart size={20} aria-hidden />
            </button>
            <button
              onClick={() => setIsSidebarOpen(true)}
              className="p-2 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-zinc-950"
              title="Settings (Ctrl+S)"
              aria-label="Open settings"
            >
              <Settings size={20} aria-hidden />
            </button>
          </nav>
        </header>

        {/* Sovereign Reset toast */}
        {sovereignResetToast && (
          <div className="fixed top-20 left-1/2 -translate-x-1/2 z-90 max-w-md px-4 py-3 rounded-lg shadow-lg border border-amber-500/50 bg-amber-50 dark:bg-amber-950/90 dark:border-amber-500/30 text-amber-900 dark:text-amber-100 text-sm animate-in fade-in slide-in-from-top-2">
            <p className="font-medium">Sovereign Reset suggested</p>
            {sovereignResetToast.message && <p className="mt-1 text-xs opacity-90">{sovereignResetToast.message}</p>}
            {sovereignResetToast.healthReminder && (
              <p className="mt-1 text-xs text-amber-700 dark:text-amber-300">{sovereignResetToast.healthReminder}</p>
            )}
          </div>
        )}
        {/* High stress + low vitality toast (autonomous hook) */}
        {stressVitalityToast && (
          <div className="fixed top-20 left-1/2 -translate-x-1/2 z-90 max-w-md px-4 py-3 rounded-lg shadow-lg border border-red-500/50 bg-red-50 dark:bg-red-950/90 dark:border-red-500/30 text-red-900 dark:text-red-100 text-sm animate-in fade-in slide-in-from-top-2">
            <p className="font-medium">High stress and low physical vitality</p>
            <p className="mt-1 text-xs opacity-90">Consider a Sovereign Reset or a short break.</p>
          </div>
        )}

        {/* Forced Reset Countdown Overlay */}
        {forcedResetCountdown !== null && forcedResetCountdown > 0 && (
          <div className="fixed inset-0 z-100 flex items-center justify-center bg-black/70 backdrop-blur-md animate-in fade-in">
            <div className="bg-red-950/95 border-2 border-red-500 rounded-2xl p-8 max-w-lg text-center shadow-2xl">
              <div className="text-red-400 text-6xl font-bold mb-4 tabular-nums">
                {forcedResetCountdown}
              </div>
              <h2 className="text-2xl font-bold text-red-100 mb-3">
                Sovereign Reset Initiated
              </h2>
              <p className="text-red-200 text-sm mb-2">
                Physical and Cognitive limits exceeded.
              </p>
              <p className="text-red-300 text-base font-semibold">
                System Lock in T-minus {forcedResetCountdown}
              </p>
            </div>
          </div>
        )}

        {/* Main Content */}
        <main className="flex-1 overflow-hidden relative flex flex-col">
          <div className="flex-1 overflow-hidden flex">
            <div className="hidden sm:flex">
              <ChatHistorySidebar />
            </div>
            <div className="flex-1 overflow-hidden min-w-0">
            {activeView === 'wellness' ? (
              <WellnessTab
                onReportLoaded={setWellnessReport}
                accentMode={(gatewayConfig?.orchestrator_role ?? gatewayConfig?.persona_mode) === 'counselor' ? 'counselor' : 'companion'}
              />
            ) : activeView === 'briefing' ? (
              <SovereignReport
                accentMode={(gatewayConfig?.orchestrator_role ?? gatewayConfig?.persona_mode) === 'counselor' ? 'counselor' : 'companion'}
              />
            ) : (
              <ChatInterface
                messages={messages}
                isLoading={isLoading}
                isStreaming={isStreaming}
                streamingStatusMessage={streamingStatusMessage}
                onSendMessage={handleSendMessage}
                settings={settings}
                onTogglePin={togglePin}
                moeActive={gatewayConfig?.moe_active ?? false}
                onOpenSettings={() => setIsSidebarOpen(true)}
              />
            )}
            </div>
          </div>
          
          {/* System Health Bar (bottom of main area) */}
          <SystemHealth />
          
          {/* Overlay for Sidebars */}
          {(isSidebarOpen || isPinnedSidebarOpen || isAuditLogOpen) && (
            <div
              className="absolute inset-0 bg-black/20 dark:bg-black/50 backdrop-blur-sm z-40 transition-opacity"
              onClick={() => {
                setIsSidebarOpen(false);
                setIsPinnedSidebarOpen(false);
                setIsAuditLogOpen(false);
              }}
            />
          )}
          
          <SettingsSidebar
            isOpen={isSidebarOpen}
            onClose={() => setIsSidebarOpen(false)}
            settings={settings}
            setSettings={setSettings}
            messages={messages}
            onClearChat={handleClearChat}
            gatewayConfig={gatewayConfig}
            onConfigRefetch={fetchConfig}
            activeProjectId={activeThread?.projectId ?? null}
            projectNames={{}}
          />

          <PinnedSidebar
             isOpen={isPinnedSidebarOpen}
             onClose={() => setIsPinnedSidebarOpen(false)}
             messages={messages}
             onTogglePin={togglePin}
          />

          {/* Chronos Audit Log (Maintenance Loop history) */}
          <ChronosAuditLog
            isOpen={isAuditLogOpen}
            onClose={() => setIsAuditLogOpen(false)}
          />

          {/* Warden Check-in modal (Spirit/Mind/Body) */}
          <BalanceCheckModal
            isOpen={balanceModalOpen}
            onClose={() => setBalanceModalOpen(false)}
            message={balanceModalMessage || undefined}
            onSaved={() => fetchConfig()}
          />
        </main>
      </div>
    </>
  );
}; 

const App: React.FC = () => (
  <ChatProvider>
    <AppInner />
  </ChatProvider>
);

export default App;
