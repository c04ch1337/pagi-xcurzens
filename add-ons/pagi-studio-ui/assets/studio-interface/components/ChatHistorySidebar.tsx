import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { ChevronDown, ChevronRight, FileText, Folder, FolderPlus, Pencil, Plus, Tag, Trash2 } from 'lucide-react';
import { useChatStore } from '../src/stores/ChatStore';
import { API_BASE_URL } from '../src/api/config';

interface ProjectAssociationRecord {
  project_id: string;
  local_path: string;
  master_analysis: boolean;
}

function formatTime(ts: number): string {
  try {
    return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  } catch {
    return '';
  }
}

export default function ChatHistorySidebar() {
  const { state, actions } = useChatStore();
  const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({});
  const [coreVersion, setCoreVersion] = useState<string | null>(null);
  const [associations, setAssociations] = useState<ProjectAssociationRecord[]>([]);
  const [mountModalProject, setMountModalProject] = useState<{ id: string; name: string } | null>(null);
  const [mountPathInput, setMountPathInput] = useState('');
  const [documentingProjectId, setDocumentingProjectId] = useState<string | null>(null);

  const fetchAssociations = useCallback(() => {
    fetch(`${API_BASE_URL}/projects/associations`)
      .then((res) => (res.ok ? res.json() : { associations: [] }))
      .then((data: { associations?: ProjectAssociationRecord[] }) => {
        setAssociations(data.associations ?? []);
      })
      .catch(() => setAssociations([]));
  }, []);

  useEffect(() => {
    let cancelled = false;
    fetch(`${API_BASE_URL}/system/status`)
      .then((res) => (res.ok ? res.json() : null))
      .then((data: { version?: string } | null) => {
        if (!cancelled && data?.version) setCoreVersion(data.version);
      })
      .catch(() => {});
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    fetchAssociations();
  }, [fetchAssociations]);

  const activeThread = state.threads.find(t => t.id === state.activeThreadId) ?? state.threads[0];

  const { ungroupedThreads, projectsWithThreads } = useMemo(() => {
    const threadsSorted = [...state.threads].sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0));
    const byProject = new Map<string, typeof threadsSorted>();
    const ungrouped: typeof threadsSorted = [];
    for (const t of threadsSorted) {
      if (!t.projectId) {
        ungrouped.push(t);
      } else {
        const arr = byProject.get(t.projectId) ?? [];
        arr.push(t);
        byProject.set(t.projectId, arr);
      }
    }
    const projects = [...state.projects].sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0));
    const projectsWith = projects
      .map(p => ({ project: p, threads: byProject.get(p.id) ?? [] }))
      .filter(x => x.threads.length > 0);
    return { ungroupedThreads: ungrouped, projectsWithThreads: projectsWith };
  }, [state.projects, state.threads]);

  const onNewChat = () => {
    actions.newThread({ title: 'New Chat' });
  };

  const renameThread = (threadId: string, current: string) => {
    const title = window.prompt('Rename chat', current);
    if (title == null) return;
    actions.renameThread(threadId, title);
  };

  const deleteThread = (threadId: string, current: string) => {
    if (!window.confirm(`Delete chat "${current}"? This cannot be undone.`)) return;
    actions.deleteThread(threadId);
  };

  const tagThread = (threadId: string, currentProjectName?: string | null) => {
    const next = window.prompt('Project tag (e.g. "Project: Beta Launch" or "Beta Launch"). Leave blank to remove.', currentProjectName ?? '');
    if (next == null) return;
    const trimmed = next.trim();
    actions.tagThreadToProjectName(threadId, trimmed ? trimmed : null);
  };

  const assocByProjectId = useMemo(() => {
    const map = new Map<string, ProjectAssociationRecord>();
    for (const a of associations) map.set(a.project_id, a);
    return map;
  }, [associations]);

  const associateProject = async (projectId: string, localPath: string, masterAnalysis: boolean) => {
    const res = await fetch(`${API_BASE_URL}/projects/associate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ project_id: projectId, local_path: localPath, master_analysis: masterAnalysis }),
    });
    if (res.ok) fetchAssociations();
    return res;
  };

  const openMountModal = (project: { id: string; name: string }) => {
    const current = assocByProjectId.get(project.id);
    setMountPathInput(current?.local_path ?? '');
    setMountModalProject(project);
  };

  const submitMount = async () => {
    if (!mountModalProject) return;
    const path = mountPathInput.trim().replace(/\\/g, '/').replace(/\/+$/, '');
    if (!path) {
      setMountModalProject(null);
      return;
    }
    const current = assocByProjectId.get(mountModalProject.id);
    await associateProject(mountModalProject.id, path, current?.master_analysis ?? true);
    setMountModalProject(null);
    setMountPathInput('');
  };

  const toggleMasterAnalysis = async (projectId: string) => {
    const current = assocByProjectId.get(projectId);
    if (!current) return;
    await associateProject(projectId, current.local_path, !current.master_analysis);
  };

  const documentSession = async (projectId: string) => {
    const thread = state.threads.find(t => t.id === state.activeThreadId);
    if (!thread || thread.projectId !== projectId) {
      window.alert('Select a chat in this project first, then click Document Session.');
      return;
    }
    if (thread.messages.length === 0) {
      window.alert('This chat has no messages to document.');
      return;
    }
    setDocumentingProjectId(projectId);
    try {
      const res = await fetch(`${API_BASE_URL}/projects/document-session`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          project_id: projectId,
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
        window.alert(`Session documented to project folder.\n\nFile: ${data.path ?? 'history/...'}\n\nOpen in Obsidian or VS Code to view diagrams.`);
      } else {
        window.alert(data.message ?? 'Failed to document session.');
      }
    } catch (e) {
      window.alert('Failed to reach gateway. Ensure the gateway is running.');
    } finally {
      setDocumentingProjectId(null);
    }
  };

  return (
    <aside className="w-72 shrink-0 border-r border-zinc-200 dark:border-zinc-800 bg-white/60 dark:bg-zinc-950/60 backdrop-blur-sm flex flex-col">
      <div className="p-3 border-b border-zinc-200 dark:border-zinc-800">
        <button
          type="button"
          onClick={onNewChat}
          className="w-full inline-flex items-center justify-center gap-2 px-3 py-2 rounded-md bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900 hover:opacity-90 transition-opacity text-sm font-medium"
        >
          <Plus size={16} aria-hidden />
          New Chat
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-4">
        <section>
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-zinc-500 dark:text-zinc-500 mb-2">Recent Chats</h3>
          <div className="space-y-1">
            {ungroupedThreads.length === 0 && (
              <div className="text-xs text-zinc-500 dark:text-zinc-600 px-2 py-1">No recent chats</div>
            )}
            {ungroupedThreads.map(t => (
              <div
                key={t.id}
                role="button"
                tabIndex={0}
                onClick={() => actions.setActiveThread(t.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    actions.setActiveThread(t.id);
                  }
                }}
                className={`w-full text-left px-2 py-2 rounded-md border transition-colors group cursor-pointer select-none ${
                  t.id === activeThread?.id
                    ? 'bg-emerald-500/10 border-emerald-500/30 text-zinc-900 dark:text-zinc-100'
                    : 'bg-transparent border-transparent hover:bg-zinc-100 dark:hover:bg-zinc-900/40 text-zinc-700 dark:text-zinc-300'
                }`}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="text-sm font-medium truncate">{t.title}</div>
                    <div className="text-[10px] text-zinc-500 dark:text-zinc-500 font-mono">{formatTime(t.updatedAt)}</div>
                  </div>
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <span className="sr-only">Chat actions</span>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        renameThread(t.id, t.title);
                      }}
                      className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-800 text-zinc-500"
                      title="Rename"
                      aria-label="Rename"
                    >
                      <Pencil size={14} aria-hidden />
                    </button>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        const projectName = t.projectId ? (state.projects.find(p => p.id === t.projectId)?.name ?? '') : '';
                        tagThread(t.id, projectName);
                      }}
                      className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-800 text-zinc-500"
                      title="Tag to Project"
                      aria-label="Tag to project"
                    >
                      <Tag size={14} aria-hidden />
                    </button>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteThread(t.id, t.title);
                      }}
                      className="p-1 rounded hover:bg-red-500/15 text-zinc-500 hover:text-red-600 dark:hover:text-red-400"
                      title="Delete"
                      aria-label="Delete"
                    >
                      <Trash2 size={14} aria-hidden />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-zinc-500 dark:text-zinc-500 mb-2">Project Folders</h3>
          <div className="space-y-2">
            {projectsWithThreads.length === 0 && (
              <div className="text-xs text-zinc-500 dark:text-zinc-600 px-2 py-1">
                Tag a chat to a project to create folders.
              </div>
            )}
            {projectsWithThreads.map(({ project, threads }) => {
              const isExpanded = expandedProjects[project.id] ?? true;
              const assoc = assocByProjectId.get(project.id);
              return (
                <div key={project.id} className="rounded-md border border-zinc-200 dark:border-zinc-800 overflow-hidden">
                  <button
                    type="button"
                    onClick={() => setExpandedProjects(prev => ({ ...prev, [project.id]: !isExpanded }))}
                    className="w-full flex items-center justify-between px-2.5 py-2 bg-zinc-50 dark:bg-zinc-900/30 hover:bg-zinc-100 dark:hover:bg-zinc-900/50 transition-colors"
                  >
                    <span className="flex items-center gap-2 min-w-0">
                      {isExpanded ? <ChevronDown size={14} aria-hidden /> : <ChevronRight size={14} aria-hidden />}
                      <Folder size={14} aria-hidden className="text-emerald-600 dark:text-emerald-400" />
                      <span className="text-sm font-medium truncate">{project.name}</span>
                    </span>
                    <span className="text-[10px] font-mono text-zinc-500">{threads.length}</span>
                  </button>

                  {isExpanded && (
                    <>
                      {/* Project Vault: Mount Local Folder + Master Analysis */}
                      <div className="px-2.5 py-2 border-t border-zinc-200 dark:border-zinc-800 bg-zinc-50/50 dark:bg-zinc-900/20 flex flex-col gap-2">
                        <div className="flex items-center gap-2 flex-wrap">
                          <button
                            type="button"
                            onClick={() => openMountModal(project)}
                            className="inline-flex items-center gap-1.5 px-2 py-1.5 rounded text-xs font-medium bg-zinc-200 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-300 dark:hover:bg-zinc-700 transition-colors"
                            title="Mount local folder for this project"
                          >
                            <FolderPlus size={12} aria-hidden />
                            {assoc ? 'Change folder' : 'Mount folder'}
                          </button>
                          {assoc && (
                            <>
                              <label className="inline-flex items-center gap-1.5 cursor-pointer">
                                <span className="text-xs text-zinc-600 dark:text-zinc-400">Master Analysis</span>
                                <button
                                  type="button"
                                  role="switch"
                                  aria-checked={assoc.master_analysis}
                                  onClick={() => toggleMasterAnalysis(project.id)}
                                  className={`relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors focus:outline-none ${assoc.master_analysis ? 'bg-emerald-600' : 'bg-zinc-300 dark:bg-zinc-700'}`}
                                  title={assoc.master_analysis ? 'Analysis ON: folder content injected into chat' : 'Analysis OFF'}
                                >
                                  <span className={`absolute top-1 h-3 w-3 rounded-full bg-white transition-transform ${assoc.master_analysis ? 'left-5' : 'left-1'}`} />
                                </button>
                              </label>
                              <button
                                type="button"
                                onClick={() => documentSession(project.id)}
                                disabled={documentingProjectId === project.id || activeThread?.projectId !== project.id}
                                className="inline-flex items-center gap-1.5 px-2 py-1.5 rounded text-xs font-medium bg-emerald-600/90 dark:bg-emerald-700/90 text-white hover:bg-emerald-600 dark:hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                                title={activeThread?.projectId !== project.id ? 'Select a chat in this project to document' : 'Export this session as Markdown to the project folder (Mermaid diagrams preserved)'}
                              >
                                <FileText size={12} aria-hidden />
                                {documentingProjectId === project.id ? 'Exporting…' : 'Document Session'}
                              </button>
                            </>
                          )}
                        </div>
                        {assoc && (
                          <div className="text-[10px] font-mono text-zinc-500 dark:text-zinc-500 truncate" title={assoc.local_path}>
                            {assoc.local_path}
                          </div>
                        )}
                      </div>
                    <div className="p-2 space-y-1 bg-white dark:bg-zinc-950">
                       {threads.map(t => (
                         <div
                           key={t.id}
                           role="button"
                           tabIndex={0}
                           onClick={() => actions.setActiveThread(t.id)}
                           onKeyDown={(e) => {
                             if (e.key === 'Enter' || e.key === ' ') {
                               e.preventDefault();
                               actions.setActiveThread(t.id);
                             }
                           }}
                           className={`w-full text-left px-2 py-2 rounded-md border transition-colors group cursor-pointer select-none ${
                             t.id === activeThread?.id
                               ? 'bg-emerald-500/10 border-emerald-500/30 text-zinc-900 dark:text-zinc-100'
                               : 'bg-transparent border-transparent hover:bg-zinc-100 dark:hover:bg-zinc-900/40 text-zinc-700 dark:text-zinc-300'
                           }`}
                         >
                           <div className="flex items-start justify-between gap-2">
                             <div className="min-w-0">
                               <div className="text-sm font-medium truncate">{t.title}</div>
                               <div className="text-[10px] text-zinc-500 dark:text-zinc-500 font-mono">{formatTime(t.updatedAt)}</div>
                             </div>
                             <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                               <button
                                 type="button"
                                 onClick={(e) => {
                                   e.stopPropagation();
                                   renameThread(t.id, t.title);
                                 }}
                                 className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-800 text-zinc-500"
                                 title="Rename"
                                 aria-label="Rename"
                               >
                                 <Pencil size={14} aria-hidden />
                               </button>
                               <button
                                 type="button"
                                 onClick={(e) => {
                                   e.stopPropagation();
                                   tagThread(t.id, project.name);
                                 }}
                                 className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-800 text-zinc-500"
                                 title="Retag"
                                 aria-label="Retag"
                               >
                                 <Tag size={14} aria-hidden />
                               </button>
                               <button
                                 type="button"
                                 onClick={(e) => {
                                   e.stopPropagation();
                                   deleteThread(t.id, t.title);
                                 }}
                                 className="p-1 rounded hover:bg-red-500/15 text-zinc-500 hover:text-red-600 dark:hover:text-red-400"
                                 title="Delete"
                                 aria-label="Delete"
                               >
                                 <Trash2 size={14} aria-hidden />
                               </button>
                             </div>
                           </div>
                         </div>
                       ))}
                    </div>
                    </>
                  )}
                </div>
              );
            })}
          </div>
        </section>
      </div>

      {/* Mount Local Folder modal */}
      {mountModalProject && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50" aria-modal="true" role="dialog">
          <div className="bg-white dark:bg-zinc-900 rounded-lg shadow-xl border border-zinc-200 dark:border-zinc-700 w-full max-w-md p-4">
            <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100 mb-2">Mount local folder — {mountModalProject.name}</h3>
            <p className="text-xs text-zinc-500 dark:text-zinc-400 mb-2">Enter the full path to the folder on this machine (e.g. C:\Projects\PROOFPOINT or /home/user/project).</p>
            <input
              type="text"
              value={mountPathInput}
              onChange={(e) => setMountPathInput(e.target.value)}
              placeholder="C:\Projects\PROOFPOINT"
              className="w-full px-3 py-2 rounded border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 text-sm font-mono placeholder:text-zinc-400 focus:outline-none focus:ring-2 focus:ring-emerald-500"
              onKeyDown={(e) => e.key === 'Enter' && submitMount()}
              autoFocus
            />
            <div className="flex justify-end gap-2 mt-3">
              <button
                type="button"
                onClick={() => { setMountModalProject(null); setMountPathInput(''); }}
                className="px-3 py-1.5 rounded text-sm font-medium text-zinc-600 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={submitMount}
                className="px-3 py-1.5 rounded text-sm font-medium bg-emerald-600 text-white hover:bg-emerald-700"
              >
                Mount
              </button>
            </div>
          </div>
        </div>
      )}

      <footer className="shrink-0 p-2 border-t border-zinc-200 dark:border-zinc-800">
        <div className="text-xs text-zinc-500 dark:text-zinc-500 opacity-50 font-mono truncate" title="Sovereign Core version">
          Sovereign Core {coreVersion != null ? `v${coreVersion}` : '…'}
        </div>
      </footer>
    </aside>
  );
}

