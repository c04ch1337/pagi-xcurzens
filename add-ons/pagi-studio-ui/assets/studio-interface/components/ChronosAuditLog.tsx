import React, { useState, useEffect, useCallback } from 'react';
import { ScrollText, RefreshCw, ChevronDown, ChevronUp, Brain } from 'lucide-react';
import { API_BASE_URL } from '../src/api/config';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ChronosEvent {
  timestamp_ms: number;
  source_kb: string;
  skill_name: string | null;
  reflection: string;
  outcome: string | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTimestamp(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function outcomeColor(outcome: string | null): string {
  if (!outcome) return 'text-zinc-500';
  const lower = outcome.toLowerCase();
  if (lower.includes('healthy') || lower.includes('saved') || lower.includes('resolved') || lower.includes('validation_passed')) return 'text-emerald-400';
  if (lower.includes('syntactic_hallucination')) return 'text-red-500';
  if (lower.includes('fail') || lower.includes('error') || lower.includes('declined')) return 'text-red-400';
  if (lower.includes('no_patch') || lower.includes('no_code') || lower.includes('no_api')) return 'text-yellow-400';
  return 'text-cyan-400';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

const ChronosAuditLog: React.FC<Props> = ({ isOpen, onClose }) => {
  const [events, setEvents] = useState<ChronosEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [limit, setLimit] = useState(50);
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null);

  const fetchEvents = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE_URL}/maintenance/audit-log?limit=${limit}`);
      if (res.ok) {
        const data = await res.json();
        setEvents(data.events ?? []);
      }
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, [limit]);

  useEffect(() => {
    if (isOpen) fetchEvents();
  }, [isOpen, fetchEvents]);

  if (!isOpen) return null;

  return (
    <div className="absolute right-0 top-0 bottom-0 w-[420px] max-w-full bg-white dark:bg-zinc-950 border-l border-zinc-200 dark:border-zinc-800 z-50 flex flex-col shadow-2xl">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-200 dark:border-zinc-800">
        <div className="flex items-center gap-2">
          <Brain size={16} className="text-purple-400" />
          <span className="font-semibold text-sm">Maintenance Audit Log</span>
          <span className="text-xs text-zinc-500 bg-zinc-100 dark:bg-zinc-900 px-1.5 py-0.5 rounded">
            MAINTENANCE_LOOP
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={fetchEvents}
            disabled={loading}
            className="p-1 text-zinc-500 hover:text-zinc-300 transition-colors"
            title="Refresh"
          >
            <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          </button>
          <button
            onClick={onClose}
            className="p-1 text-zinc-500 hover:text-zinc-300 transition-colors text-lg leading-none"
          >
            ×
          </button>
        </div>
      </div>

      {/* Controls */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-zinc-100 dark:border-zinc-800/50 text-xs">
        <label className="text-zinc-500">Show:</label>
        <select
          value={limit}
          onChange={(e) => setLimit(Number(e.target.value))}
          className="bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded px-2 py-0.5 text-xs"
        >
          <option value={25}>25</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
          <option value={200}>200</option>
        </select>
        <span className="text-zinc-500 ml-auto">{events.length} events</span>
      </div>

      {/* Event list */}
      <div className="flex-1 overflow-y-auto">
        {events.length === 0 && !loading && (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500 text-sm">
            <ScrollText size={32} className="mb-2 opacity-30" />
            <p>No maintenance events yet.</p>
            <p className="text-xs mt-1">The maintenance loop records events here when it runs.</p>
          </div>
        )}

        {events.map((event, idx) => {
          const isExpanded = expandedIdx === idx;
          return (
            <div
              key={`${event.timestamp_ms}-${idx}`}
              className="border-b border-zinc-100 dark:border-zinc-800/30 hover:bg-zinc-50 dark:hover:bg-zinc-900/50 transition-colors"
            >
              <button
                onClick={() => setExpandedIdx(isExpanded ? null : idx)}
                className="w-full flex items-start gap-2 px-4 py-2 text-left text-xs"
              >
                <span className="text-zinc-500 font-mono whitespace-nowrap mt-0.5">
                  {formatTimestamp(event.timestamp_ms)}
                </span>
                <span className="flex-1 text-zinc-300 leading-relaxed truncate">
                  {event.reflection}
                </span>
                <span className={`${outcomeColor(event.outcome)} whitespace-nowrap font-mono`}>
                  {event.outcome ?? '—'}
                </span>
                {isExpanded ? <ChevronUp size={12} className="mt-0.5 text-zinc-600" /> : <ChevronDown size={12} className="mt-0.5 text-zinc-600" />}
              </button>

              {isExpanded && (
                <div className="px-4 pb-3 space-y-1 text-xs">
                  <div className="flex gap-2">
                    <span className="text-zinc-500 w-16">Source:</span>
                    <span className="text-zinc-300">{event.source_kb}</span>
                  </div>
                  <div className="flex gap-2">
                    <span className="text-zinc-500 w-16">Skill:</span>
                    <span className="text-cyan-400 font-mono">{event.skill_name ?? '—'}</span>
                  </div>
                  <div className="flex gap-2">
                    <span className="text-zinc-500 w-16">Outcome:</span>
                    <span className={`${outcomeColor(event.outcome)} font-mono`}>{event.outcome ?? '—'}</span>
                  </div>
                  <div className="mt-1">
                    <span className="text-zinc-500">Reflection:</span>
                    <p className="text-zinc-300 mt-0.5 leading-relaxed whitespace-pre-wrap bg-zinc-900/50 rounded p-2">
                      {event.reflection}
                    </p>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default ChronosAuditLog;
