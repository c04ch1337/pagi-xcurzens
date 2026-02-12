import React, { useState, useEffect, useCallback } from 'react';
import { Activity, AlertTriangle, RefreshCw } from 'lucide-react';
import { API_BASE_URL } from '../src/api/config';
import type { WellnessReport } from '../types';

const PILLAR_ORDER = ['Spirit', 'Mind', 'Body'] as const;
const RADIUS = 80;
const CX = 100;
const CY = 100;
const MAX_VAL = 10;

function polarToCartesian(angleRad: number, r: number) {
  return { x: CX + r * Math.cos(angleRad), y: CY - r * Math.sin(angleRad) };
}

interface WellnessTabProps {
  onReportLoaded?: (report: WellnessReport | null) => void;
  /** Counselor = emerald/sage, Companion = amber */
  accentMode?: 'counselor' | 'companion';
}

const WellnessTab: React.FC<WellnessTabProps> = ({
  onReportLoaded,
  accentMode = 'counselor',
}) => {
  const [report, setReport] = useState<WellnessReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchReport = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`${API_BASE_URL}/skills/wellness-report`);
      const data = await res.json().catch(() => ({}));
      if (res.ok && data.status === 'ok' && data.report) {
        setReport(data.report as WellnessReport);
        onReportLoaded?.(data.report);
      } else {
        setError(data.error || 'Failed to load wellness report');
        onReportLoaded?.(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Network error');
      setReport(null);
      onReportLoaded?.(null);
    } finally {
      setLoading(false);
    }
  }, [onReportLoaded]);

  useEffect(() => {
    fetchReport();
  }, [fetchReport]);

  const accent = accentMode === 'counselor'
    ? { border: 'border-emerald-500/40', gauge: 'text-emerald-500', fill: '#10b981' }
    : { border: 'border-amber-500/40', gauge: 'text-amber-500', fill: '#f59e0b' };

  if (loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6 bg-zinc-50 dark:bg-zinc-900/50">
        <div className="w-full max-w-lg space-y-4">
          <div className="h-8 w-48 bg-zinc-200 dark:bg-zinc-800 rounded animate-pulse" />
          <div className="h-64 bg-zinc-200 dark:bg-zinc-800 rounded-xl animate-pulse" />
          <div className="h-24 bg-zinc-200 dark:bg-zinc-800 rounded-lg animate-pulse" />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6">
        <p className="text-sm text-red-600 dark:text-red-400 mb-4">{error}</p>
        <button
          onClick={fetchReport}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-zinc-200 dark:bg-zinc-700 text-zinc-800 dark:text-zinc-200 hover:bg-zinc-300 dark:hover:bg-zinc-600 transition-colors"
        >
          <RefreshCw size={16} />
          Retry
        </button>
      </div>
    );
  }

  const spirit = report?.pillars?.Spirit ?? 0;
  const mind = report?.pillars?.Mind ?? 0;
  const body = report?.pillars?.Body ?? 0;
  const score = Math.min(1, Math.max(0, report?.individuation_score ?? 0));
  const isCritical = report?.is_critical ?? false;
  const flags = report?.flags ?? [];
  const entriesUsed = report?.entries_used ?? 0;

  // Radar: Spirit top (-90°), Mind 30°, Body 150°
  const angles = [-Math.PI / 2, Math.PI / 6, 5 * Math.PI / 6];
  const values = [spirit, mind, body];
  const points = values.map((v, i) => {
    const r = RADIUS * (v / MAX_VAL);
    return polarToCartesian(angles[i], r);
  });
  const polygonPoints = points.map(p => `${p.x},${p.y}`).join(' ');
  const axisEnds = angles.map(a => polarToCartesian(a, RADIUS));

  return (
    <div
      className={`h-full overflow-auto p-6 bg-zinc-50 dark:bg-zinc-900/50 ${
        isCritical
          ? 'ring-2 ring-red-500/60 rounded-xl border-2 border-red-500/40'
          : ''
      }`}
    >
      {isCritical && (
        <div className="flex items-center gap-2 mb-4 px-4 py-2 rounded-lg bg-red-500/10 border border-red-500/30 text-red-700 dark:text-red-300">
          <AlertTriangle size={20} />
          <span className="font-semibold text-sm">Warning: Low Vitality Detected</span>
        </div>
      )}

      <div className="max-w-2xl mx-auto space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100 flex items-center gap-2">
            <Activity size={20} className={accent.gauge} />
            Wellness (7-day Soma)
          </h2>
          <button
            onClick={fetchReport}
            className="p-2 text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 rounded-lg hover:bg-zinc-200 dark:hover:bg-zinc-800 transition-colors"
            title="Refresh"
          >
            <RefreshCw size={18} />
          </button>
        </div>

        {/* Radar + Gauge row */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
          {/* Radar */}
          <div className={`rounded-xl border ${accent.border} bg-white dark:bg-zinc-950 p-4`}>
            <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wider mb-3">
              Spirit / Mind / Body
            </p>
            <svg viewBox="0 0 200 200" className="w-full max-w-[200px] mx-auto">
              <circle cx={CX} cy={CY} r={RADIUS} fill="none" stroke="currentColor" strokeOpacity={0.15} className="text-zinc-400 dark:text-zinc-600" />
              {[0.25, 0.5, 0.75, 1].map(scale => (
                <circle key={scale} cx={CX} cy={CY} r={RADIUS * scale} fill="none" stroke="currentColor" strokeOpacity={0.08} className="text-zinc-400 dark:text-zinc-600" />
              ))}
              {angles.map((a, i) => (
                <line
                  key={i}
                  x1={CX}
                  y1={CY}
                  x2={axisEnds[i].x}
                  y2={axisEnds[i].y}
                  stroke="currentColor"
                  strokeOpacity={0.2}
                  className="text-zinc-400 dark:text-zinc-600"
                />
              ))}
              <polygon
                points={polygonPoints}
                fill={accent.fill}
                fillOpacity={0.35}
                stroke={accent.fill}
                strokeWidth={1.5}
                strokeOpacity={0.8}
              />
              {PILLAR_ORDER.map((label, i) => {
                const end = polarToCartesian(angles[i], RADIUS + 14);
                return (
                  <text key={label} x={end.x} y={end.y} textAnchor="middle" className="fill-current text-zinc-600 dark:text-zinc-400" style={{ fontSize: 11 }}>
                    {label} {values[i].toFixed(1)}
                  </text>
                );
              })}
            </svg>
          </div>

          {/* Individuation gauge */}
          <div className={`rounded-xl border ${accent.border} bg-white dark:bg-zinc-950 p-4 flex flex-col items-center justify-center`}>
            <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wider mb-2">
              Individuation
            </p>
            <div className="relative w-32 h-32">
              <svg viewBox="0 0 36 36" className="w-full h-full -rotate-90">
                <path
                  d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                  fill="none"
                  stroke="currentColor"
                  strokeOpacity={0.1}
                  strokeWidth={2}
                  className="text-zinc-400 dark:text-zinc-600"
                />
                <path
                  d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
                  fill="none"
                  stroke={accent.fill}
                  strokeWidth={2}
                  strokeDasharray={`${score * 100} 100`}
                  strokeLinecap="round"
                  className="transition-all duration-500"
                />
              </svg>
              <div className="absolute inset-0 flex items-center justify-center">
                <span className={`text-2xl font-bold ${accent.gauge}`}>
                  {Math.round(score * 100)}%
                </span>
              </div>
            </div>
            <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-1">
              Based on {entriesUsed} balance check{entriesUsed !== 1 ? 's' : ''}
            </p>
          </div>
        </div>

        {/* Summary card */}
        <div className={`rounded-xl border ${accent.border} bg-white dark:bg-zinc-950 p-4`}>
          <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wider mb-2">
            Summary
          </p>
          <p className="text-sm text-zinc-700 dark:text-zinc-300 leading-relaxed">
            {report?.summary ?? 'No summary available.'}
          </p>
          {flags.length > 0 && (
            <div className="mt-3 flex flex-wrap gap-2">
              {flags.map(f => (
                <span
                  key={f}
                  className="px-2 py-1 rounded text-xs font-medium bg-amber-500/15 text-amber-700 dark:text-amber-400 border border-amber-500/30"
                >
                  {f}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Strategic Maneuvers */}
        {report?.root_cause && (
          <div className={`rounded-xl border ${accent.border} bg-white dark:bg-zinc-950 p-4`}>
            <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wider mb-2">
              Strategic Maneuvers
            </p>
            <div className="text-sm text-zinc-700 dark:text-zinc-300 leading-relaxed">
              <p className="font-medium text-zinc-800 dark:text-zinc-200 mb-1">Root Cause Analysis:</p>
              <p className="italic">{report.root_cause}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default WellnessTab;
