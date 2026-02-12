import React, { useState, useEffect, useCallback } from 'react';
import { RefreshCw, Shield, Flame, Gauge, PieChart } from 'lucide-react';
import { API_BASE_URL } from '../src/api/config';
import type { HealthReport, ConfigStatus, ArchetypeUsageBreakdown } from '../types';

/** Archetype → accent (Bare Metal: no external deps). Virgo=Technical, Pisces=Emotional, Capricorn=Strategy. */
const ARCHETYPE_ACCENT: Record<string, { bar: string; text: string; border: string; chart: string[] }> = {
  virgo:   { bar: 'bg-emerald-500',  text: 'text-emerald-600 dark:text-emerald-400', border: 'border-emerald-500/30', chart: ['#10b981', '#059669'] },
  pisces:  { bar: 'bg-blue-500',    text: 'text-blue-600 dark:text-blue-400',     border: 'border-blue-500/30',     chart: ['#3b82f6', '#2563eb'] },
  capricorn: { bar: 'bg-amber-500', text: 'text-amber-600 dark:text-amber-400',  border: 'border-amber-500/30',    chart: ['#f59e0b', '#d97706'] },
  scorpio: { bar: 'bg-violet-500',   text: 'text-violet-600 dark:text-violet-400', border: 'border-violet-500/30',   chart: ['#8b5cf6', '#7c3aed'] },
  libra:   { bar: 'bg-pink-500',    text: 'text-pink-600 dark:text-pink-400',     border: 'border-pink-500/30',     chart: ['#ec4899', '#db2777'] },
  cancer:  { bar: 'bg-teal-500',    text: 'text-teal-600 dark:text-teal-400',     border: 'border-teal-500/30',     chart: ['#14b8a6', '#0d9488'] },
  leo:     { bar: 'bg-orange-500',  text: 'text-orange-600 dark:text-orange-400', border: 'border-orange-500/30',   chart: ['#f97316', '#ea580c'] },
};
const DEFAULT_ACCENT = { bar: 'bg-emerald-500', text: 'text-emerald-600 dark:text-emerald-400', border: 'border-emerald-500/30', chart: ['#10b981', '#059669'] };

/** Donut segment color order for Identity Mix. */
const DONUT_COLORS = ['#10b981', '#3b82f6', '#f59e0b', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316', '#64748b'];

interface SovereignReportProps {
  accentMode?: 'counselor' | 'companion';
}

const SovereignReport: React.FC<SovereignReportProps> = ({ accentMode = 'counselor' }) => {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [configStatus, setConfigStatus] = useState<ConfigStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchReport = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [reportRes, configRes] = await Promise.all([
        fetch(`${API_BASE_URL}/health-report`),
        fetch(`${API_BASE_URL}/config/status`),
      ]);
      const reportData = await reportRes.json().catch(() => ({}));
      const configData = await configRes.json().catch(() => ({}));

      if (reportRes.ok && reportData.status === 'ok' && reportData.report) {
        setReport(reportData.report as HealthReport);
      } else {
        setError(reportData.error || 'Failed to load Sovereign Health Report');
        setReport(null);
      }
      if (configRes.ok && configData.humanity_ratio !== undefined) {
        setConfigStatus({
          humanity_ratio: configData.humanity_ratio,
          current_active_archetype: configData.current_active_archetype ?? 'pisces',
          persona_blend: configData.persona_blend,
        });
      } else {
        setConfigStatus(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Network error');
      setReport(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchReport();
  }, [fetchReport]);

  const archetypeKey = configStatus?.current_active_archetype?.toLowerCase().replace(/\s+/g, '') ?? '';
  const themeAccent = ARCHETYPE_ACCENT[archetypeKey] ?? (accentMode === 'companion' ? ARCHETYPE_ACCENT.capricorn : DEFAULT_ACCENT);

  if (loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6 bg-zinc-50 dark:bg-zinc-900/50">
        <div className="w-full max-w-2xl space-y-4">
          <div className="h-6 w-64 bg-zinc-200 dark:bg-zinc-800 rounded animate-pulse" />
          <div className="h-32 bg-zinc-200 dark:bg-zinc-800 rounded-xl animate-pulse" />
          <div className="h-48 bg-zinc-200 dark:bg-zinc-800 rounded-xl animate-pulse" />
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

  if (!report) {
    return null;
  }

  const vitalityPct = Math.min(100, Math.round(
    (report.leak_stats.total_shielded > 0 ? 40 : 0) +
    (report.transit_correlations.length > 0 ? 25 : 0) +
    (report.efficiency_score >= 0.5 ? 35 : report.efficiency_score * 70)
  ));

  const humanityPct = configStatus ? Math.round(configStatus.humanity_ratio * 100) : 50;

  return (
    <div className="h-full overflow-auto bg-zinc-50 dark:bg-zinc-900/50">
      <div className="max-w-3xl mx-auto p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-xl font-semibold text-zinc-900 dark:text-zinc-100 tracking-tight">
            The Briefing Room
          </h1>
          <button
            onClick={fetchReport}
            className="p-2 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-200 hover:bg-zinc-200 dark:hover:bg-zinc-800 rounded-lg transition-colors"
            title="Refresh report"
          >
            <RefreshCw size={18} />
          </button>
        </div>

        <p className="text-xs text-zinc-500 dark:text-zinc-500">
          {report.window_start} → {report.window_end}
          {report.user_name && ` · ${report.user_name}`}
          {report.archetype_label && ` · ${report.archetype_label}`}
          {configStatus?.current_active_archetype && (
            <span className={`ml-1 capitalize ${themeAccent.text}`}>
              · Theme: {configStatus.current_active_archetype}
            </span>
          )}
        </p>

        {/* Identity Mix – Archetype Donut */}
        {report.archetype_usage_breakdown && report.archetype_usage_breakdown.total_turns > 0 && (
          <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
            <div className="flex items-center gap-2 mb-3">
              <PieChart size={18} className={themeAccent.text} />
              <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200">Identity Mix (Mirror of Intent)</h2>
            </div>
            <div className="flex flex-col sm:flex-row items-center gap-4">
              <ArchetypeDonut breakdown={report.archetype_usage_breakdown} colors={DONUT_COLORS} />
              <div className="flex-1 min-w-0">
                <p className="text-xs text-zinc-600 dark:text-zinc-400 mb-2">{report.archetype_usage_breakdown.summary}</p>
                <div className="flex flex-wrap gap-2">
                  {Object.entries(report.archetype_usage_breakdown.by_archetype)
                    .sort(([, a], [, b]) => b - a)
                    .map(([name, count], i) => {
                      const pct = report.archetype_usage_breakdown!.total_turns > 0
                        ? Math.round((count / report.archetype_usage_breakdown!.total_turns) * 100)
                        : 0;
                      return (
                        <span
                          key={name}
                          className="px-2 py-1 rounded-md text-xs font-medium text-zinc-700 dark:text-zinc-300 border"
                          style={{
                            borderColor: DONUT_COLORS[i % DONUT_COLORS.length],
                            backgroundColor: `${DONUT_COLORS[i % DONUT_COLORS.length]}18`,
                          }}
                        >
                          {name}: {pct}%
                        </span>
                      );
                    })}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Humanity Level Gauge */}
        {configStatus && (
          <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
            <div className="flex items-center gap-2 mb-2">
              <Gauge size={18} className={themeAccent.text} />
              <span className="text-sm font-medium text-zinc-700 dark:text-zinc-300">Humanity Level (Vibe)</span>
            </div>
            <div className="h-3 w-full bg-zinc-200 dark:bg-zinc-800 rounded-full overflow-hidden">
              <div
                className={`h-full ${themeAccent.bar} transition-all duration-500 rounded-full`}
                style={{ width: `${humanityPct}%` }}
              />
            </div>
            <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">
              {configStatus.persona_blend ?? `${humanityPct}%`} — Adjust in settings to shift Phoenix between technical and emotional tone.
            </p>
          </div>
        )}

        {/* Phoenix Vitality bar */}
        <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
          <div className="flex items-center gap-2 mb-2">
            <Flame size={18} className={themeAccent.text} />
            <span className="text-sm font-medium text-zinc-700 dark:text-zinc-300">Phoenix Vitality (week)</span>
          </div>
          <div className="h-3 w-full bg-zinc-200 dark:bg-zinc-800 rounded-full overflow-hidden">
            <div
              className={`h-full ${themeAccent.bar} transition-all duration-500 rounded-full`}
              style={{ width: `${vitalityPct}%` }}
            />
          </div>
          <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">
            Based on shielded events, transit awareness, and boundary efficiency.
          </p>
        </div>

        {/* Phoenix Insights – Narrative Summary with Gatekeeper highlight */}
        <div className={`rounded-xl border ${themeAccent.border} bg-white dark:bg-zinc-900 p-4`}>
          <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200 mb-2">Phoenix Insights</h2>
          <div className="text-sm text-zinc-700 dark:text-zinc-300 leading-relaxed whitespace-pre-line">
            <PhoenixSummaryWithHighlights text={report.phoenix_summary} highlightClass={themeAccent.text} />
          </div>
        </div>

        {/* Rest vs. Output correlation (Vitality Shield) */}
        {(report.rest_vs_output?.length ?? 0) > 0 && (
          <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
            <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200 mb-2">Rest vs. Output</h2>
            {report.vitality_score != null && (
              <p className="text-xs text-zinc-600 dark:text-zinc-400 mb-3">
                Avg rest (sleep) this week: {report.vitality_score.toFixed(1)}h. Correlation with chat + shielded events below.
              </p>
            )}
            <div className="space-y-2">
              {report.rest_vs_output!.map((row) => {
                const maxOut = Math.max(1, ...report.rest_vs_output!.map((r) => r.output_score));
                const restW = Math.min(100, (row.rest_score / 12) * 100);
                const outW = maxOut > 0 ? (row.output_score / maxOut) * 100 : 0;
                return (
                  <div key={row.date} className="flex items-center gap-2 text-xs">
                    <span className="w-24 text-zinc-500 dark:text-zinc-500 shrink-0">{row.date}</span>
                    <div className="flex-1 flex gap-1">
                      <div
                        className="h-5 rounded bg-blue-500/40 dark:bg-blue-500/30 min-w-[2px]"
                        style={{ width: `${restW}%` }}
                        title={`Rest: ${row.rest_score.toFixed(1)}h`}
                      />
                      <div
                        className="h-5 rounded bg-emerald-500/40 dark:bg-emerald-500/30 min-w-[2px]"
                        style={{ width: `${outW}%` }}
                        title={`Output: ${row.output_score.toFixed(0)}`}
                      />
                    </div>
                    <span className="text-zinc-500 dark:text-zinc-500 shrink-0">
                      {row.rest_score.toFixed(1)}h / {row.output_score.toFixed(0)}
                    </span>
                  </div>
                );
              })}
            </div>
            <p className="mt-2 text-xs text-zinc-500 dark:text-zinc-500">Blue = rest (sleep h), Green = output (turns + shielded).</p>
          </div>
        )}

        {/* Cognitive Load & Focus placeholder */}
        <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
          <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200 mb-2">Cognitive Load & Focus</h2>
          <p className="text-sm text-zinc-600 dark:text-zinc-400">
            Focus and calendar context (meeting density, Gatekeeper Mode) are applied in chat. Rest vs. Output appears above when Vitality Shield data is present.
          </p>
        </div>

        {/* Shielded Events */}
        <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
          <div className="flex items-center gap-2 mb-3">
            <Shield size={18} className={themeAccent.text} />
            <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200">Shielded Events</h2>
          </div>
          {report.shielded_events.length === 0 ? (
            <p className="text-sm text-zinc-500 dark:text-zinc-500">No events in this window. Log success metrics via KB-08 when you enforce a boundary.</p>
          ) : (
            <ul className="space-y-2">
              {report.shielded_events.map((ev, i) => (
                <li
                  key={`${ev.timestamp_ms}-${i}`}
                  className="text-sm text-zinc-700 dark:text-zinc-300 border-l-2 border-zinc-200 dark:border-zinc-700 pl-3 py-1"
                >
                  <span className="text-zinc-500 dark:text-zinc-500 text-xs">{ev.date}</span>
                  <span className="ml-2 font-medium text-zinc-500 dark:text-zinc-500">{ev.category}</span>
                  <p className="mt-0.5">{ev.message}</p>
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Transit impact */}
        {report.transit_correlations.length > 0 && (
          <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
            <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200 mb-2">Transit impact (high-risk days with events)</h2>
            <ul className="space-y-1.5 text-sm text-zinc-600 dark:text-zinc-400">
              {report.transit_correlations.map((t, i) => (
                <li key={`${t.date}-${i}`}>
                  <span className="text-zinc-500 dark:text-zinc-500">{t.date}</span>
                  {' · '}
                  {t.transit_summary}
                  {t.event_kind && ` (${t.event_kind})`}
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Leak stats by category */}
        {report.leak_stats.total_shielded > 0 && Object.keys(report.leak_stats.by_category).length > 0 && (
          <div className="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 p-4">
            <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200 mb-2">By category</h2>
            <div className="flex flex-wrap gap-2">
              {Object.entries(report.leak_stats.by_category).map(([cat, count]) => (
                <span
                  key={cat}
                  className="px-2 py-1 rounded-md bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 text-xs"
                >
                  {cat}: {count}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

/** Renders phoenix_summary and highlights "Shield" / "Gatekeeper" phrases. */
function PhoenixSummaryWithHighlights({ text, highlightClass }: { text: string; highlightClass: string }) {
  const parts = text.split(/(\b(?:Shield|Gatekeeper|Gatekeeper Mode|Shield was|shield)\b)/gi);
  return (
    <>
      {parts.map((part, i) =>
        /shield|gatekeeper/i.test(part) ? (
          <span key={i} className={`font-semibold ${highlightClass}`}>{part}</span>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </>
  );
}

/** Donut chart from archetype percentages (CSS conic-gradient). */
function ArchetypeDonut({ breakdown, colors }: { breakdown: ArchetypeUsageBreakdown; colors: string[] }) {
  const total = breakdown.total_turns || 1;
  const entries = Object.entries(breakdown.by_archetype)
    .map(([name, count]) => ({ name, count, pct: (count / total) * 100 }))
    .filter((e) => e.pct > 0)
    .sort((a, b) => b.pct - a.pct);

  if (entries.length === 0) {
    return (
      <div className="w-32 h-32 rounded-full bg-zinc-200 dark:bg-zinc-800 flex items-center justify-center text-zinc-500 text-xs">
        No data
      </div>
    );
  }

  let acc = 0;
  const conicParts = entries.map((e, i) => {
    const start = acc;
    acc += e.pct;
    return `${colors[i % colors.length]} ${start}% ${acc}%`;
  });
  const conic = `conic-gradient(${conicParts.join(', ')})`;

  return (
    <div className="relative w-32 h-32 flex-shrink-0">
      <div
        className="absolute inset-0 rounded-full bg-zinc-100 dark:bg-zinc-800"
        style={{ background: conic }}
      />
      <div className="absolute inset-[20%] rounded-full bg-white dark:bg-zinc-900" />
      <div className="absolute inset-0 flex items-center justify-center">
        <span className="text-xs font-medium text-zinc-600 dark:text-zinc-400">{breakdown.total_turns} turns</span>
      </div>
    </div>
  );
}

export default SovereignReport;
