import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import type { Message } from '../../types';
import { API_BASE_URL } from '../api/config';

export interface Project {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
}

export interface ChatThread {
  id: string;
  title: string;
  projectId: string | null;
  createdAt: number;
  updatedAt: number;
  messages: Message[];
}

export interface ChatState {
  projects: Project[];
  threads: ChatThread[];
  activeThreadId: string;
  /** Path to the last Mimir meeting summary (set when meeting stops); used for "Synthesize With Infrastructure". */
  lastMimirSummaryPath: string | null;
}

export interface ChatActions {
  newThread: (opts?: { title?: string; projectId?: string | null }) => string;
  setActiveThread: (threadId: string) => void;
  renameThread: (threadId: string, title: string) => void;
  deleteThread: (threadId: string) => void;
  tagThreadToProjectName: (threadId: string, projectNameOrNull: string | null) => void;

  /** Fetch latest projects/threads from KB-04 (Chronos SQLite). */
  syncFromChronos: () => Promise<void>;
  /** Fetch messages for a thread from KB-04 (Chronos SQLite) and populate local state. */
  loadThreadMessagesFromChronos: (threadId: string) => Promise<void>;

  renameProject: (projectId: string, name: string) => void;
  deleteProject: (projectId: string) => void;

  addMessage: (threadId: string, message: Message) => void;
  updateMessage: (threadId: string, messageId: string, patch: Partial<Message>) => void;
  clearThread: (threadId: string) => void;

  /** Set path to the last Mimir summary (after meeting stop) for "Synthesize With Infrastructure". */
  setLastMimirSummaryPath: (path: string | null) => void;
}

const STORAGE_KEY = 'pagi_chat_store_v1';

type ChronosProjectRow = {
  id: string;
  name: string;
  created_at_ms: number;
  updated_at_ms: number;
};

type ChronosThreadRow = {
  id: string;
  project_id: string | null;
  title: string;
  created_at_ms: number;
  updated_at_ms: number;
  last_message_at_ms: number | null;
};

type ChronosMessageRow = {
  id: string;
  thread_id: string;
  project_id: string | null;
  role: 'user' | 'assistant' | string;
  content: string;
  created_at_ms: number;
  metadata_json: string | null;
};

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }
  return (await res.json()) as T;
}

function nowMs(): number {
  return Date.now();
}

function newId(prefix: string): string {
  // deterministic-enough local id; backend will later supply canonical ids.
  return `${prefix}_${Math.random().toString(16).slice(2)}_${Date.now().toString(16)}`;
}

function normalizeProjectLabel(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) return '';
  // Allow user to type either "Beta Launch" or "Project: Beta Launch".
  const lower = trimmed.toLowerCase();
  if (lower.startsWith('project:')) {
    const rest = trimmed.slice('project:'.length).trim();
    return rest ? `Project: ${rest}` : 'Project';
  }
  return `Project: ${trimmed}`;
}

function loadInitialState(): ChatState {
  // Migration: single-thread legacy localStorage key
  const legacy = localStorage.getItem('agi_chat_history');
  const stored = localStorage.getItem(STORAGE_KEY);
  if (!stored) {
    if (legacy) {
      try {
        const legacyMessages = JSON.parse(legacy) as Message[];
        const id = newId('thread');
        return {
          projects: [],
          threads: [
            {
              id,
              title: 'Legacy Chat',
              projectId: null,
              createdAt: nowMs(),
              updatedAt: nowMs(),
              messages: Array.isArray(legacyMessages) ? legacyMessages : [],
            },
          ],
          activeThreadId: id,
          lastMimirSummaryPath: null,
        };
      } catch {
        // fall through
      }
    }
    const id = newId('thread');
    return {
      projects: [],
      threads: [
        {
          id,
          title: 'New Chat',
          projectId: null,
          createdAt: nowMs(),
          updatedAt: nowMs(),
          messages: [],
        },
      ],
      activeThreadId: id,
      lastMimirSummaryPath: null,
    };
  }

  try {
    const parsed = JSON.parse(stored) as ChatState;
    if (!parsed || !Array.isArray(parsed.threads) || typeof parsed.activeThreadId !== 'string') {
      throw new Error('invalid store');
    }
    // Basic guardrails: ensure at least one thread.
    if (parsed.threads.length === 0) {
      const id = newId('thread');
      return {
        projects: [],
        threads: [
          { id, title: 'New Chat', projectId: null, createdAt: nowMs(), updatedAt: nowMs(), messages: [] },
        ],
        activeThreadId: id,
        lastMimirSummaryPath: null,
      };
    }
    const activeExists = parsed.threads.some(t => t.id === parsed.activeThreadId);
    return {
      projects: Array.isArray(parsed.projects) ? parsed.projects : [],
      threads: parsed.threads,
      activeThreadId: activeExists ? parsed.activeThreadId : parsed.threads[0]!.id,
      lastMimirSummaryPath: typeof parsed.lastMimirSummaryPath === 'string' ? parsed.lastMimirSummaryPath : null,
    };
  } catch {
    const id = newId('thread');
    return {
      projects: [],
      threads: [
        {
          id,
          title: 'New Chat',
          projectId: null,
          createdAt: nowMs(),
          updatedAt: nowMs(),
          messages: [],
        },
      ],
      activeThreadId: id,
      lastMimirSummaryPath: null,
    };
  }
}

const ChatContext = createContext<{ state: ChatState; actions: ChatActions } | null>(null);

export function ChatProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<ChatState>(() => loadInitialState());

  const didInitialChronosSync = useRef(false);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // ignore
    }
  }, [state]);

  const setActiveThread = useCallback((threadId: string) => {
    setState(prev => ({
      ...prev,
      activeThreadId: prev.threads.some(t => t.id === threadId) ? threadId : prev.activeThreadId,
    }));
  }, []);

  const syncFromChronos = useCallback(async () => {
    type ProjectsResp = { status: string; projects?: ChronosProjectRow[] };
    type ThreadsResp = { status: string; threads?: ChronosThreadRow[] };

    try {
      const [projectsResp, threadsResp] = await Promise.all([
        fetchJson<ProjectsResp>(`${API_BASE_URL}/chronos/projects`),
        fetchJson<ThreadsResp>(`${API_BASE_URL}/chronos/threads?limit=200`),
      ]);

      const projects = (projectsResp.projects ?? []).map((p) => ({
        id: p.id,
        name: p.name,
        createdAt: p.created_at_ms,
        updatedAt: p.updated_at_ms,
      }));

      const threads = (threadsResp.threads ?? []).map((t) => ({
        id: t.id,
        title: t.title || 'New Chat',
        projectId: t.project_id ?? null,
        createdAt: t.created_at_ms,
        updatedAt: t.last_message_at_ms ?? t.updated_at_ms,
        messages: [],
      }));

      setState((prev) => {
        const nextActive = threads.some((t) => t.id === prev.activeThreadId)
          ? prev.activeThreadId
          : (threads[0]?.id ?? prev.activeThreadId);
        return {
          ...prev,
          projects,
          threads: threads.length > 0 ? threads : prev.threads,
          activeThreadId: nextActive,
        };
      });
    } catch {
      // If Chronos is not reachable (gateway down), stay in local mode.
    }
  }, []);

  const loadThreadMessagesFromChronos = useCallback(async (threadId: string) => {
    const tid = threadId.trim();
    if (!tid) return;
    type MessagesResp = { status: string; messages?: ChronosMessageRow[] };
    try {
      const resp = await fetchJson<MessagesResp>(
        `${API_BASE_URL}/chronos/threads/${encodeURIComponent(tid)}/messages?limit=1000`
      );
      const rows = resp.messages ?? [];
      const messages: Message[] = rows.map((m) => ({
        id: m.id,
        role: m.role === 'assistant' ? 'agi' : 'user',
        content: m.content,
        timestamp: m.created_at_ms,
      }));
      setState((prev) => ({
        ...prev,
        threads: prev.threads.map((t) =>
          t.id === tid
            ? {
                ...t,
                projectId: mProjectIdFromMessages(rows, t.projectId),
                messages,
                updatedAt:
                  (messages[messages.length - 1]?.timestamp ?? t.updatedAt ?? nowMs()),
              }
            : t
        ),
      }));
    } catch {
      // ignore
    }
  }, []);

  const newThread = useCallback((opts?: { title?: string; projectId?: string | null }) => {
    const id = newId('thread');
    const title = (opts?.title ?? 'New Chat').trim() || 'New Chat';
    const projectId = opts?.projectId ?? null;
    const ts = nowMs();
    setState(prev => ({
      ...prev,
      threads: [{ id, title, projectId, createdAt: ts, updatedAt: ts, messages: [] }, ...prev.threads],
      activeThreadId: id,
    }));

    // Best-effort: create the thread in Chronos so it appears after refresh.
    fetchJson(`${API_BASE_URL}/chronos/threads`, {
      method: 'POST',
      body: JSON.stringify({ id, title, project_id: projectId }),
    }).catch(() => {});

    return id;
  }, []);

  const renameThread = useCallback((threadId: string, title: string) => {
    const nextTitle = title.trim();
    if (!nextTitle) return;
    setState(prev => ({
      ...prev,
      threads: prev.threads.map(t => (t.id === threadId ? { ...t, title: nextTitle, updatedAt: nowMs() } : t)),
    }));

    fetchJson(`${API_BASE_URL}/chronos/threads/${encodeURIComponent(threadId)}`, {
      method: 'POST',
      body: JSON.stringify({ title: nextTitle }),
    }).catch(() => {});
  }, []);

  const deleteThread = useCallback((threadId: string) => {
    setState(prev => {
      const remaining = prev.threads.filter(t => t.id !== threadId);
      if (remaining.length === 0) {
        const id = newId('thread');
        const ts = nowMs();
        return {
          ...prev,
          threads: [{ id, title: 'New Chat', projectId: null, createdAt: ts, updatedAt: ts, messages: [] }],
          activeThreadId: id,
        };
      }
      const nextActive = prev.activeThreadId === threadId ? remaining[0]!.id : prev.activeThreadId;
      return { ...prev, threads: remaining, activeThreadId: nextActive };
    });

    fetchJson(`${API_BASE_URL}/chronos/threads/${encodeURIComponent(threadId)}`, {
      method: 'DELETE',
    }).catch(() => {});
  }, []);

  const upsertProjectByName = useCallback((rawName: string): string | null => {
    const name = normalizeProjectLabel(rawName);
    if (!name) return null;

    const id = newId('project');
    const ts = nowMs();

    let existingId: string | null = null;
    setState(prev => {
      const existing = prev.projects.find(p => p.name.toLowerCase() === name.toLowerCase());
      if (existing) {
        existingId = existing.id;
        return {
          ...prev,
          projects: prev.projects.map(p => (p.id === existing.id ? { ...p, updatedAt: ts } : p)),
        };
      }
      existingId = id;
      return {
        ...prev,
        projects: [{ id, name, createdAt: ts, updatedAt: ts }, ...prev.projects],
      };
    });
    return existingId;
  }, []);

  const tagThreadToProjectName = useCallback((threadId: string, projectNameOrNull: string | null) => {
    const ts = nowMs();

    // Optimistic local update (UI responsiveness)
    setState((prev) => {
      const normalized = projectNameOrNull ? normalizeProjectLabel(projectNameOrNull) : '';
      if (!normalized) {
        return {
          ...prev,
          threads: prev.threads.map((t) => (t.id === threadId ? { ...t, projectId: null, updatedAt: ts } : t)),
        };
      }

      // Keep a local placeholder project until backend returns canonical id.
      const existing = prev.projects.find((p) => p.name.toLowerCase() === normalized.toLowerCase());
      const localPid = existing?.id ?? newId('project');
      const projects = existing
        ? prev.projects.map((p) => (p.id === existing.id ? { ...p, updatedAt: ts } : p))
        : [{ id: localPid, name: normalized, createdAt: ts, updatedAt: ts }, ...prev.projects];

      return {
        ...prev,
        projects,
        threads: prev.threads.map((t) => (t.id === threadId ? { ...t, projectId: localPid, updatedAt: ts } : t)),
      };
    });

    // Persist to Chronos: create/upsert project by name, then tag thread to returned project_id.
    const normalized = projectNameOrNull ? normalizeProjectLabel(projectNameOrNull) : '';
    if (!normalized) {
      fetchJson(`${API_BASE_URL}/chronos/threads/${encodeURIComponent(threadId)}/tag`, {
        method: 'POST',
        body: JSON.stringify({ project_id: null }),
      }).catch(() => {});
      return;
    }

    type ProjectResp = { status: string; project?: ChronosProjectRow };
    fetchJson<ProjectResp>(`${API_BASE_URL}/chronos/projects`, {
      method: 'POST',
      body: JSON.stringify({ name: normalized }),
    })
      .then((r) => r.project)
      .then((project) => {
        if (!project) return;
        return fetchJson(`${API_BASE_URL}/chronos/threads/${encodeURIComponent(threadId)}/tag`, {
          method: 'POST',
          body: JSON.stringify({ project_id: project.id }),
        }).then(() => project);
      })
      .then((project) => {
        if (!project) return;
        // Replace placeholder project/thread ids with canonical project id.
        setState((prev) => {
          const nextProjects = dedupeProjects([
            { id: project.id, name: project.name, createdAt: project.created_at_ms, updatedAt: project.updated_at_ms },
            ...prev.projects,
          ]);
          return {
            ...prev,
            projects: nextProjects,
            threads: prev.threads.map((t) =>
              t.id === threadId ? { ...t, projectId: project.id, updatedAt: nowMs() } : t
            ),
          };
        });
      })
      .catch(() => {});
  }, []);

  // On mount: prefer Chronos as source of truth if gateway is reachable.
  useEffect(() => {
    if (didInitialChronosSync.current) return;
    didInitialChronosSync.current = true;
    void syncFromChronos();
  }, [syncFromChronos]);

  // Background sync: keeps sidebar up to date even if threads are created outside the UI.
  useEffect(() => {
    const intervalMs = 8000;
    const id = window.setInterval(() => {
      void syncFromChronos();
    }, intervalMs);
    const onFocus = () => void syncFromChronos();
    window.addEventListener('focus', onFocus);
    return () => {
      window.clearInterval(id);
      window.removeEventListener('focus', onFocus);
    };
  }, [syncFromChronos]);

  // When active thread changes: load messages (lazy) from Chronos.
  useEffect(() => {
    const tid = state.activeThreadId;
    const thread = state.threads.find((t) => t.id === tid);
    if (!thread) return;
    if (thread.messages.length > 0) return;
    void loadThreadMessagesFromChronos(tid);
  }, [state.activeThreadId, state.threads, loadThreadMessagesFromChronos]);

  const renameProject = useCallback((projectId: string, name: string) => {
    const next = normalizeProjectLabel(name);
    if (!next) return;
    setState(prev => ({
      ...prev,
      projects: prev.projects.map(p => (p.id === projectId ? { ...p, name: next, updatedAt: nowMs() } : p)),
    }));
  }, []);

  const deleteProject = useCallback((projectId: string) => {
    setState(prev => ({
      ...prev,
      projects: prev.projects.filter(p => p.id !== projectId),
      threads: prev.threads.map(t => (t.projectId === projectId ? { ...t, projectId: null, updatedAt: nowMs() } : t)),
    }));
  }, []);

  const addMessage = useCallback((threadId: string, message: Message) => {
    setState(prev => ({
      ...prev,
      threads: prev.threads.map(t => (t.id === threadId ? { ...t, messages: [...t.messages, message], updatedAt: nowMs() } : t)),
    }));
  }, []);

  const updateMessage = useCallback((threadId: string, messageId: string, patch: Partial<Message>) => {
    setState(prev => ({
      ...prev,
      threads: prev.threads.map(t => {
        if (t.id !== threadId) return t;
        return {
          ...t,
          messages: t.messages.map(m => (m.id === messageId ? { ...m, ...patch } : m)),
          updatedAt: nowMs(),
        };
      }),
    }));
  }, []);

  const clearThread = useCallback((threadId: string) => {
    setState(prev => ({
      ...prev,
      threads: prev.threads.map(t => (t.id === threadId ? { ...t, messages: [], updatedAt: nowMs() } : t)),
    }));
  }, []);

  const setLastMimirSummaryPath = useCallback((path: string | null) => {
    setState(prev => ({ ...prev, lastMimirSummaryPath: path }));
  }, []);

  const actions: ChatActions = useMemo(
    () => ({
      newThread,
      setActiveThread,
      renameThread,
      deleteThread,
      tagThreadToProjectName,
      syncFromChronos,
      loadThreadMessagesFromChronos,
      renameProject,
      deleteProject,
      addMessage,
      updateMessage,
      clearThread,
      setLastMimirSummaryPath,
    }),
    [
      newThread,
      setActiveThread,
      renameThread,
      deleteThread,
      tagThreadToProjectName,
      syncFromChronos,
      loadThreadMessagesFromChronos,
      renameProject,
      deleteProject,
      addMessage,
      updateMessage,
      clearThread,
      setLastMimirSummaryPath,
    ]
  );

  return <ChatContext.Provider value={{ state, actions }}>{children}</ChatContext.Provider>;
}

export function useChatStore() {
  const ctx = useContext(ChatContext);
  if (!ctx) throw new Error('useChatStore must be used within ChatProvider');
  return ctx;
}

function dedupeProjects(list: Project[]): Project[] {
  const seen = new Set<string>();
  const out: Project[] = [];
  for (const p of list) {
    if (!p?.id) continue;
    if (seen.has(p.id)) continue;
    seen.add(p.id);
    out.push(p);
  }
  // Also coalesce by name (case-insensitive), keep the most recently updated.
  const byName = new Map<string, Project>();
  for (const p of out) {
    const key = (p.name ?? '').toLowerCase();
    if (!key) continue;
    const existing = byName.get(key);
    if (!existing || (p.updatedAt ?? 0) > (existing.updatedAt ?? 0)) {
      byName.set(key, p);
    }
  }
  const final: Project[] = [];
  const used = new Set<string>();
  for (const p of out) {
    const key = (p.name ?? '').toLowerCase();
    const winner = key ? byName.get(key) : p;
    if (winner && !used.has(winner.id)) {
      used.add(winner.id);
      final.push(winner);
    }
  }
  return final;
}

function mProjectIdFromMessages(rows: ChronosMessageRow[], fallback: string | null): string | null {
  for (const r of rows) {
    if (r.project_id) return r.project_id;
  }
  return fallback;
}

