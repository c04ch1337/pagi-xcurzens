import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Activity, Brain, Shield, CheckCircle, AlertTriangle, Clock, Zap, History, RotateCcw, GitBranch, Dna, ShieldCheck, ShieldAlert, ShieldX, Eye } from 'lucide-react';
import { API_BASE_URL } from '../src/api/config';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SecurityAudit {
  passed: boolean;
  overall_severity: string;
  reviewer_model: string;
  findings_count: number;
  summary: string;
  memory_warning: string | null;
}

interface PerformanceDelta {
  cpu: string;
  mem: string;
  compiled: boolean;
  smoke_test_passed: boolean;
  detail: string;
  security_audit?: SecurityAudit | null;
}

interface MaintenancePulse {
  phase: string;
  target: string;
  details: string;
  timestamp_ms: number;
  applied_patches: number;
  failure_count: number;
  performance_delta?: PerformanceDelta | null;
}

interface MaintenanceStatus {
  idle_secs: number;
  pending_approval: PendingApproval | null;
  applied_patches: number;
  maintenance_agent_id: string;
}

interface PendingApproval {
  id: string;
  description: string;
  patch_name: string;
  skill: string;
  created_ms: number;
}

interface PatchHistoryEntry {
  skill_name: string;
  timestamp_ms: number;
  filename: string;
  is_active: boolean;
  file_size: number;
  path: string;
}

// ---------------------------------------------------------------------------
// Phase → visual mapping
// ---------------------------------------------------------------------------

const PHASE_CONFIG: Record<string, { icon: React.ReactNode; color: string; label: string; animate: boolean }> = {
  idle:              { icon: <Clock size={14} />,          color: 'text-zinc-400',   label: 'Idle',                animate: false },
  starting:          { icon: <Zap size={14} />,            color: 'text-blue-400',   label: 'Starting',            animate: true },
  telemetry:         { icon: <Activity size={14} />,       color: 'text-cyan-400',   label: 'Telemetry',           animate: true },
  audit:             { icon: <Shield size={14} />,         color: 'text-yellow-400', label: 'Auditing',            animate: true },
  reflexion:         { icon: <Brain size={14} />,          color: 'text-purple-400', label: 'Reflexion',           animate: true },
  patching:          { icon: <Zap size={14} />,            color: 'text-orange-400', label: 'Patching',            animate: true },
  validation:        { icon: <Activity size={14} />,       color: 'text-indigo-400', label: 'Validating',          animate: true },
  peer_review:       { icon: <Eye size={14} />,             color: 'text-rose-400',   label: 'Peer Review',         animate: true },
  auto_rejected:     { icon: <AlertTriangle size={14} />,  color: 'text-red-400',    label: 'Auto-Rejected',       animate: false },
  red_team_rejected: { icon: <ShieldX size={14} />,        color: 'text-red-500',    label: 'Red-Team Rejected',   animate: false },
  awaiting_approval: { icon: <AlertTriangle size={14} />,  color: 'text-amber-400',  label: 'Awaiting Approval',   animate: true },
  applying:          { icon: <Zap size={14} />,            color: 'text-green-400',  label: 'Applying',            animate: true },
  complete:          { icon: <CheckCircle size={14} />,    color: 'text-green-400',  label: 'Complete',            animate: false },
  healthy:           { icon: <CheckCircle size={14} />,    color: 'text-emerald-400',label: 'Healthy',             animate: false },
};

const DEFAULT_PHASE = { icon: <Activity size={14} />, color: 'text-zinc-500', label: 'Unknown', animate: false };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTimestamp(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleString(undefined, {
    month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
  });
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

type Tab = 'status' | 'history' | 'security';

const SystemHealth: React.FC = () => {
  const [pulse, setPulse] = useState<MaintenancePulse | null>(null);
  const [status, setStatus] = useState<MaintenanceStatus | null>(null);
  const [patchCount, setPatchCount] = useState<number>(0);
  const [expanded, setExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>('status');
  const [patchHistory, setPatchHistory] = useState<PatchHistoryEntry[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [rollbackInProgress, setRollbackInProgress] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  // Fetch initial status + patch count
  const fetchStatus = useCallback(async () => {
    try {
      const [statusRes, patchRes] = await Promise.all([
        fetch(`${API_BASE_URL}/maintenance/status`),
        fetch(`${API_BASE_URL}/maintenance/patches`),
      ]);
      if (statusRes.ok) setStatus(await statusRes.json());
      if (patchRes.ok) {
        const data = await patchRes.json();
        setPatchCount(data.count ?? 0);
      }
    } catch {
      // Gateway not reachable — silent
    }
  }, []);

  // Fetch patch version history
  const fetchHistory = useCallback(async () => {
    setHistoryLoading(true);
    try {
      const res = await fetch(`${API_BASE_URL}/maintenance/patch-history?limit=50`);
      if (res.ok) {
        const data = await res.json();
        setPatchHistory(data.history ?? []);
      }
    } catch {
      // silent
    } finally {
      setHistoryLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 30_000); // refresh every 30s
    return () => clearInterval(interval);
  }, [fetchStatus]);

  // Fetch history when tab switches to 'history'
  useEffect(() => {
    if (activeTab === 'history' && expanded) {
      fetchHistory();
    }
  }, [activeTab, expanded, fetchHistory]);

  // SSE: maintenance_pulse stream
  useEffect(() => {
    const es = new EventSource(`${API_BASE_URL}/maintenance/pulse`);
    eventSourceRef.current = es;

    es.addEventListener('maintenance_pulse', (e: MessageEvent) => {
      try {
        const data: MaintenancePulse = JSON.parse(e.data);
        setPulse(data);
        setPatchCount(data.applied_patches);
      } catch {
        // ignore parse errors
      }
    });

    es.onerror = () => {
      // Reconnect handled by browser EventSource
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, []);

  // Approve / decline from UI
  const handleApproval = async (approved: boolean) => {
    if (!status?.pending_approval) return;
    try {
      await fetch(`${API_BASE_URL}/maintenance/approval`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: status.pending_approval.id, approved }),
      });
      // Refresh status
      fetchStatus();
    } catch {
      // silent
    }
  };

  // Rollback a skill to a specific version
  const handleRollback = async (skillName: string, timestamp: number) => {
    setRollbackInProgress(`${skillName}_${timestamp}`);
    try {
      const res = await fetch(`${API_BASE_URL}/maintenance/rollback`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          skill: skillName,
          target_timestamp: timestamp,
          reason: 'UI-initiated rollback',
        }),
      });
      if (res.ok) {
        // Refresh history after rollback
        await fetchHistory();
        await fetchStatus();
      }
    } catch {
      // silent
    } finally {
      setRollbackInProgress(null);
    }
  };

  const phase = pulse?.phase ?? 'idle';
  const config = PHASE_CONFIG[phase] ?? DEFAULT_PHASE;

  // Group history by skill
  const skillGroups = patchHistory.reduce<Record<string, PatchHistoryEntry[]>>((acc, entry) => {
    if (!acc[entry.skill_name]) acc[entry.skill_name] = [];
    acc[entry.skill_name].push(entry);
    return acc;
  }, {});

  return (
    <div className="border-t border-zinc-200 dark:border-zinc-800 mt-auto">
      {/* Compact bar */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
        aria-label={expanded ? "Collapse maintenance panel" : "Expand maintenance panel"}
        className="w-full flex items-center gap-2 px-4 py-2 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-blue-500"
        title="SAO Orchestrator Core – Maintenance & Reflexion"
      >
        {/* Brain pulse animation */}
        <span className={`${config.color} ${config.animate ? 'animate-pulse' : ''}`}>
          {config.icon}
        </span>
        <span className={`font-medium ${config.color}`}>{config.label}</span>
        <span className="text-zinc-500 dark:text-zinc-600 truncate flex-1 text-left">
          {pulse?.details ?? 'Maintenance loop active'}
        </span>
        <span className="text-[10px] text-zinc-400 dark:text-zinc-500 shrink-0">SAO Core</span>
        <span className="flex items-center gap-1 text-zinc-500 dark:text-zinc-600">
          <Zap size={10} />
          {patchCount}
        </span>
      </button>

      {/* Expanded panel */}
      {expanded && (
        <div className="border-t border-zinc-100 dark:border-zinc-800/50">
          {/* Tab bar */}
          <div className="flex border-b border-zinc-100 dark:border-zinc-800/50" role="tablist">
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'status'}
              onClick={() => setActiveTab('status')}
              className={`flex items-center gap-1 px-4 py-1.5 text-xs font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-inset ${
                activeTab === 'status'
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              <Activity size={12} aria-hidden />
              Status
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'history'}
              onClick={() => setActiveTab('history')}
              className={`flex items-center gap-1 px-4 py-1.5 text-xs font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-inset ${
                activeTab === 'history'
                  ? 'text-purple-400 border-b-2 border-purple-400'
                  : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              <History size={12} aria-hidden />
              History
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'security'}
              onClick={() => setActiveTab('security')}
              className={`flex items-center gap-1 px-4 py-1.5 text-xs font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-inset ${
                activeTab === 'security'
                  ? 'text-rose-400 border-b-2 border-rose-400'
                  : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              <Shield size={12} aria-hidden />
              Security
            </button>
          </div>

          {/* Status Tab */}
          {activeTab === 'status' && (
            <div className="px-4 pb-3 space-y-2 text-xs">
              {/* Status row */}
              <div className="flex items-center justify-between pt-2">
                <span className="text-zinc-500">Phase</span>
                <span className={`font-mono ${config.color}`}>{phase}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-500">Target</span>
                <span className="font-mono text-zinc-300">{pulse?.target ?? '—'}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-500">Idle</span>
                <span className="font-mono text-zinc-300">{status?.idle_secs ?? '?'}s</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-500">Failures (24h)</span>
                <span className={`font-mono ${(pulse?.failure_count ?? 0) > 0 ? 'text-amber-400' : 'text-emerald-400'}`}>
                  {pulse?.failure_count ?? 0}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-500">Applied Patches</span>
                <span className="font-mono text-cyan-400">{patchCount}</span>
              </div>

              {/* Performance Delta (Phase 4.5 Validation) */}
              {pulse?.performance_delta && (
                <div className="mt-2 p-2 rounded bg-indigo-500/10 border border-indigo-500/30">
                  <div className="flex items-center gap-1 text-indigo-400 font-medium mb-1">
                    <Activity size={12} />
                    Validation Benchmark
                  </div>
                  <div className="grid grid-cols-2 gap-1 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-zinc-500">Compiled</span>
                      <span className={pulse.performance_delta.compiled ? 'text-emerald-400' : 'text-red-400'}>
                        {pulse.performance_delta.compiled ? '✓' : '✗'}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-zinc-500">Smoke Test</span>
                      <span className={pulse.performance_delta.smoke_test_passed ? 'text-emerald-400' : 'text-red-400'}>
                        {pulse.performance_delta.smoke_test_passed ? '✓' : '✗'}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-zinc-500">CPU Δ</span>
                      <span className={`font-mono ${pulse.performance_delta.cpu.startsWith('-') ? 'text-emerald-400' : pulse.performance_delta.cpu.startsWith('+') ? 'text-amber-400' : 'text-zinc-400'}`}>
                        {pulse.performance_delta.cpu}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-zinc-500">Mem Δ</span>
                      <span className={`font-mono ${pulse.performance_delta.mem.startsWith('-') ? 'text-emerald-400' : pulse.performance_delta.mem.startsWith('+') ? 'text-amber-400' : 'text-zinc-400'}`}>
                        {pulse.performance_delta.mem}
                      </span>
                    </div>
                  </div>
                  {pulse.performance_delta.detail && (
                    <p className="text-zinc-500 text-[10px] mt-1 truncate" title={pulse.performance_delta.detail}>
                      {pulse.performance_delta.detail}
                    </p>
                  )}
                  {/* Efficiency summary for approval context */}
                  {pulse.performance_delta.compiled && pulse.performance_delta.smoke_test_passed && (
                    <p className="text-indigo-300 text-[11px] mt-1 font-medium">
                      {pulse.performance_delta.cpu.startsWith('-')
                        ? `This patch is ${pulse.performance_delta.cpu.replace('-', '')} more CPU-efficient. Apply? (y/n)`
                        : pulse.performance_delta.cpu.startsWith('+')
                        ? `This patch uses ${pulse.performance_delta.cpu.replace('+', '')} more CPU. Apply? (y/n)`
                        : 'Performance is comparable. Apply? (y/n)'}
                    </p>
                  )}
                </div>
              )}

              {/* Security Audit (Phase 4.75 Peer Review) */}
              {pulse?.performance_delta?.security_audit && (
                <div className={`mt-2 p-2 rounded border ${
                  pulse.performance_delta.security_audit.passed
                    ? 'bg-emerald-500/10 border-emerald-500/30'
                    : 'bg-red-500/10 border-red-500/30'
                }`}>
                  <div className={`flex items-center gap-1 font-medium mb-1 ${
                    pulse.performance_delta.security_audit.passed
                      ? 'text-emerald-400'
                      : 'text-red-400'
                  }`}>
                    {pulse.performance_delta.security_audit.passed
                      ? <ShieldCheck size={12} />
                      : <ShieldAlert size={12} />
                    }
                    Security Audit
                    <span className={`ml-1 text-[10px] px-1.5 py-0.5 rounded font-mono ${
                      pulse.performance_delta.security_audit.overall_severity === 'critical'
                        ? 'bg-red-600/30 text-red-300'
                        : pulse.performance_delta.security_audit.overall_severity === 'high'
                        ? 'bg-orange-600/30 text-orange-300'
                        : pulse.performance_delta.security_audit.overall_severity === 'medium'
                        ? 'bg-yellow-600/30 text-yellow-300'
                        : pulse.performance_delta.security_audit.overall_severity === 'low'
                        ? 'bg-blue-600/30 text-blue-300'
                        : 'bg-emerald-600/30 text-emerald-300'
                    }`}>
                      {pulse.performance_delta.security_audit.overall_severity.toUpperCase()}
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-1 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="text-zinc-500">Peer Review</span>
                      <span className={pulse.performance_delta.security_audit.passed ? 'text-emerald-400' : 'text-red-400'}>
                        {pulse.performance_delta.security_audit.passed ? '✅ Passed' : '❌ Rejected'}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-zinc-500">Findings</span>
                      <span className={`font-mono ${
                        pulse.performance_delta.security_audit.findings_count > 0
                          ? 'text-amber-400'
                          : 'text-emerald-400'
                      }`}>
                        {pulse.performance_delta.security_audit.findings_count}
                      </span>
                    </div>
                  </div>
                  <p className="text-zinc-400 text-[10px] mt-1 leading-relaxed" title={pulse.performance_delta.security_audit.summary}>
                    {pulse.performance_delta.security_audit.passed
                      ? `✅ Passed Peer Review (${pulse.performance_delta.security_audit.reviewer_model} analysis: ${pulse.performance_delta.security_audit.summary})`
                      : `🛡️ ${pulse.performance_delta.security_audit.reviewer_model}: ${pulse.performance_delta.security_audit.summary}`
                    }
                  </p>
                  {pulse.performance_delta.security_audit.memory_warning && (
                    <p className="text-amber-400 text-[10px] mt-1 flex items-center gap-1">
                      <AlertTriangle size={10} />
                      Memory: {pulse.performance_delta.security_audit.memory_warning}
                    </p>
                  )}
                </div>
              )}

              {/* Pending approval */}
              {status?.pending_approval && (
                <div className="mt-2 p-2 rounded bg-amber-500/10 border border-amber-500/30">
                  <div className="flex items-center gap-1 text-amber-400 font-medium mb-1">
                    <AlertTriangle size={12} />
                    Patch Approval Required
                  </div>
                  <p className="text-zinc-400 mb-2 leading-relaxed">
                    {status.pending_approval.description}
                  </p>
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleApproval(true)}
                      className="px-3 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium transition-colors"
                    >
                      ✓ Approve
                    </button>
                    <button
                      onClick={() => handleApproval(false)}
                      className="px-3 py-1 rounded bg-red-600 hover:bg-red-500 text-white text-xs font-medium transition-colors"
                    >
                      ✗ Decline
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* History Tab */}
          {activeTab === 'history' && (
            <div className="px-4 pb-3 text-xs">
              <div className="flex items-center justify-between pt-2 mb-2">
                <div className="flex items-center gap-1 text-purple-400 font-medium">
                  <GitBranch size={12} />
                  Evolutionary Timeline
                </div>
                <button
                  onClick={fetchHistory}
                  className="text-zinc-500 hover:text-zinc-300 transition-colors"
                  title="Refresh history"
                >
                  <RotateCcw size={12} className={historyLoading ? 'animate-spin' : ''} />
                </button>
              </div>

              {patchHistory.length === 0 && !historyLoading && (
                <div className="text-zinc-500 text-center py-4">
                  <Dna size={20} className="mx-auto mb-1 opacity-50" />
                  No patch versions found yet.
                  <br />
                  <span className="text-[10px]">Patches will appear here after the maintenance loop generates them.</span>
                </div>
              )}

              {historyLoading && (
                <div className="text-zinc-500 text-center py-4 animate-pulse">
                  Loading evolutionary timeline...
                </div>
              )}

              {/* Grouped by skill */}
              {Object.entries(skillGroups).map(([skillName, versions]) => (
                <div key={skillName} className="mb-3">
                  <div className="flex items-center gap-1 text-zinc-400 font-medium mb-1">
                    <Dna size={10} />
                    <span className="font-mono">{skillName}</span>
                    <span className="text-zinc-600 ml-1">({versions.length} version{versions.length !== 1 ? 's' : ''})</span>
                  </div>

                  <div className="space-y-1 ml-3 border-l border-zinc-800 pl-2">
                    {versions.map((entry) => {
                      const isRollingBack = rollbackInProgress === `${entry.skill_name}_${entry.timestamp_ms}`;
                      return (
                        <div
                          key={`${entry.skill_name}_${entry.timestamp_ms}`}
                          className={`flex items-center justify-between py-1 px-2 rounded ${
                            entry.is_active
                              ? 'bg-emerald-500/10 border border-emerald-500/30'
                              : 'hover:bg-zinc-800/50'
                          }`}
                        >
                          <div className="flex items-center gap-2 flex-1 min-w-0">
                            {/* Active indicator */}
                            <span
                              className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                                entry.is_active ? 'bg-emerald-400' : 'bg-zinc-600'
                              }`}
                            />
                            {/* Timestamp */}
                            <span className="font-mono text-zinc-400 shrink-0">
                              {formatTimestamp(entry.timestamp_ms)}
                            </span>
                            {/* File size */}
                            <span className="text-zinc-600 shrink-0">
                              {formatBytes(entry.file_size)}
                            </span>
                            {/* Active badge */}
                            {entry.is_active && (
                              <span className="text-emerald-400 text-[10px] font-medium shrink-0">
                                ACTIVE
                              </span>
                            )}
                          </div>

                          {/* Revert button (only for non-active versions) */}
                          {!entry.is_active && (
                            <button
                              onClick={() => handleRollback(entry.skill_name, entry.timestamp_ms)}
                              disabled={isRollingBack}
                              className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium transition-colors shrink-0 ${
                                isRollingBack
                                  ? 'bg-zinc-700 text-zinc-500 cursor-wait'
                                  : 'bg-amber-600/20 text-amber-400 hover:bg-amber-600/40 border border-amber-600/30'
                              }`}
                              title={`Revert to version from ${formatTimestamp(entry.timestamp_ms)}`}
                            >
                              <RotateCcw size={10} className={isRollingBack ? 'animate-spin' : ''} />
                              {isRollingBack ? 'Reverting...' : 'Revert'}
                            </button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Security Tab */}
          {activeTab === 'security' && (
            <div className="px-4 pb-3 text-xs">
              <div className="flex items-center gap-1 pt-2 mb-2 text-rose-400 font-medium">
                <Shield size={12} />
                Adversarial Peer Review (Phase 4.75)
              </div>

              {/* Current audit status */}
              {pulse?.performance_delta?.security_audit ? (
                <div className="space-y-2">
                  {/* Verdict card */}
                  <div className={`p-3 rounded border ${
                    pulse.performance_delta.security_audit.passed
                      ? 'bg-emerald-500/10 border-emerald-500/30'
                      : 'bg-red-500/10 border-red-500/30'
                  }`}>
                    <div className="flex items-center justify-between mb-2">
                      <div className={`flex items-center gap-1.5 font-medium ${
                        pulse.performance_delta.security_audit.passed
                          ? 'text-emerald-400'
                          : 'text-red-400'
                      }`}>
                        {pulse.performance_delta.security_audit.passed
                          ? <ShieldCheck size={14} />
                          : <ShieldX size={14} />
                        }
                        {pulse.performance_delta.security_audit.passed
                          ? 'Peer Review Passed'
                          : 'Peer Review Failed'
                        }
                      </div>
                      <span className={`text-[10px] px-2 py-0.5 rounded font-mono font-medium ${
                        pulse.performance_delta.security_audit.overall_severity === 'critical'
                          ? 'bg-red-600/30 text-red-300'
                          : pulse.performance_delta.security_audit.overall_severity === 'high'
                          ? 'bg-orange-600/30 text-orange-300'
                          : pulse.performance_delta.security_audit.overall_severity === 'medium'
                          ? 'bg-yellow-600/30 text-yellow-300'
                          : pulse.performance_delta.security_audit.overall_severity === 'low'
                          ? 'bg-blue-600/30 text-blue-300'
                          : 'bg-emerald-600/30 text-emerald-300'
                      }`}>
                        {pulse.performance_delta.security_audit.overall_severity.toUpperCase()}
                      </span>
                    </div>

                    <div className="space-y-1">
                      <div className="flex items-center justify-between">
                        <span className="text-zinc-500">Reviewer</span>
                        <span className="font-mono text-zinc-300">
                          {pulse.performance_delta.security_audit.reviewer_model}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-zinc-500">Findings</span>
                        <span className={`font-mono ${
                          pulse.performance_delta.security_audit.findings_count > 0
                            ? 'text-amber-400'
                            : 'text-emerald-400'
                        }`}>
                          {pulse.performance_delta.security_audit.findings_count}
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-zinc-500">Verdict</span>
                        <span className={pulse.performance_delta.security_audit.passed ? 'text-emerald-400' : 'text-red-400'}>
                          {pulse.performance_delta.security_audit.passed ? '✅ Approved' : '❌ Rejected'}
                        </span>
                      </div>
                    </div>

                    <p className="text-zinc-400 text-[10px] mt-2 leading-relaxed border-t border-zinc-700/50 pt-2">
                      {pulse.performance_delta.security_audit.summary}
                    </p>

                    {pulse.performance_delta.security_audit.memory_warning && (
                      <div className="mt-2 p-1.5 rounded bg-amber-500/10 border border-amber-500/20">
                        <p className="text-amber-400 text-[10px] flex items-center gap-1">
                          <AlertTriangle size={10} />
                          <span className="font-medium">Memory Warning:</span>
                          {pulse.performance_delta.security_audit.memory_warning}
                        </p>
                      </div>
                    )}
                  </div>

                  {/* Consensus pipeline visualization */}
                  <div className="p-2 rounded bg-zinc-800/50 border border-zinc-700/50">
                    <div className="text-zinc-500 text-[10px] font-medium mb-1.5">Consensus Pipeline</div>
                    <div className="flex items-center gap-1 text-[10px]">
                      <span className="px-1.5 py-0.5 rounded bg-cyan-500/20 text-cyan-400">Synthesis</span>
                      <span className="text-zinc-600">→</span>
                      <span className="px-1.5 py-0.5 rounded bg-indigo-500/20 text-indigo-400">Validation</span>
                      <span className="text-zinc-600">→</span>
                      <span className={`px-1.5 py-0.5 rounded ${
                        pulse.performance_delta.security_audit.passed
                          ? 'bg-emerald-500/20 text-emerald-400'
                          : 'bg-red-500/20 text-red-400'
                      }`}>
                        Peer Review {pulse.performance_delta.security_audit.passed ? '✓' : '✗'}
                      </span>
                      <span className="text-zinc-600">→</span>
                      <span className="px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-400">
                        {pulse.performance_delta.security_audit.passed ? 'Approval' : 'Blocked'}
                      </span>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-zinc-500 text-center py-4">
                  <Shield size={20} className="mx-auto mb-1 opacity-50" />
                  No security audit data yet.
                  <br />
                  <span className="text-[10px]">
                    Security audits run during Phase 4.75 of the maintenance loop
                    when a patch is proposed.
                  </span>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default SystemHealth;
