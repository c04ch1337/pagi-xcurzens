import React, { useState, useEffect, useCallback } from 'react';
import { Shield, Lock, Compass, Wallet, Cloud, Layers } from 'lucide-react';
import { API_BASE_URL } from '../src/api/config';

/** Skill entry from GET /api/v1/skills (3-Tier: core / import / generated). */
export interface SkillInventoryItem {
  skill_id: string;
  trust_tier: string;
  trust_status?: string;
  kb_layers_allowed?: number[];
  description?: string | null;
}

export type SentinelStatus = 'calm' | 'high' | 'rage';

/** System Vitality from GET /api/v1/domain/vitality (generic capacity/load/status). */
export type VitalityStatus = 'stable' | 'draining' | 'critical';

export interface DomainIntegrity {
  absurdity_log_count: number;
  resource_drain_alerts: string[];
  strategic_alignment?: {
    score: number;
    level: string;
    divergence_warning?: string;
  };
}

export interface VitalitySummaryResponse {
  status: string;
  vitality: string;
  message?: string;
}

/** Astro-Weather from GET /api/v1/astro-weather (transit vs KB-01). */
export interface AstroWeatherResponse {
  status: string;
  risk: 'stable' | 'elevated' | 'high_risk';
  transit_summary: string;
  advice: string;
  updated_at_ms?: number;
}

interface WardenSidebarProps {
  /** 0–100 velocity score from Sentinel (placeholder until API). */
  velocityScore: number | null;
  /** Derived from velocity or from backend event. */
  sentinelStatus: SentinelStatus;
  /** Current birth sign for archetype icon label. */
  birthSign?: string | null;
  /** Optional class for theme accent (counselor vs companion). */
  accentClass?: string;
}

function domainHealthColor(status: SentinelStatus, isCritical: boolean): string {
  if (isCritical) {
    return 'bg-red-500/20 text-red-600 dark:text-red-400 border-red-500/30';
  }
  switch (status) {
    case 'calm':
      return 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border-emerald-500/30';
    case 'high':
      return 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border-amber-500/30';
    case 'rage':
      return 'bg-red-500/20 text-red-600 dark:text-red-400 border-red-500/30';
    default:
      return 'bg-zinc-500/20 text-zinc-600 dark:text-zinc-400 border-zinc-500/30';
  }
}

function domainHealthLabel(status: SentinelStatus, isCritical: boolean): string {
  if (isCritical) return 'Critical';
  switch (status) {
    case 'calm': return 'Healthy';
    case 'high': return 'Elevated';
    case 'rage': return 'Critical';
    default: return 'Unknown';
  }
}

const WardenSidebar: React.FC<WardenSidebarProps> = ({
  velocityScore,
  sentinelStatus,
  birthSign,
  accentClass = 'border-l-amber-500/50',
}) => {
  const [domainIntegrity, setDomainIntegrity] = useState<DomainIntegrity | null>(null);
  const [vitalitySummary, setVitalitySummary] = useState<VitalitySummaryResponse | null>(null);
  const [activeArchetype, setActiveArchetype] = useState<string | null>(null);
  const [astroWeather, setAstroWeather] = useState<AstroWeatherResponse | null>(null);
  const [skillsInventory, setSkillsInventory] = useState<SkillInventoryItem[]>([]);
  const [skillsExpanded, setSkillsExpanded] = useState(false);
  const isCritical = (domainIntegrity?.resource_drain_alerts?.length ?? 0) > 0;

  const fetchSkills = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/skills`);
      if (res.ok) {
        const data = await res.json();
        setSkillsInventory(Array.isArray(data.skills) ? data.skills : []);
      }
    } catch {
      setSkillsInventory([]);
    }
  }, []);

  const promoteSkill = useCallback(async (skillId: string) => {
    if (!window.confirm(`Promote "${skillId}" to Core? This moves the skill from Ephemeral to Core (human-in-the-loop).`)) {
      return;
    }
    try {
      const res = await fetch(`${API_BASE_URL}/skills/promote`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ skill_id: skillId, confirmed: true }),
      });
      const data = await res.json().catch(() => ({}));
      if (res.ok && data.status === 'ok') {
        await fetchSkills();
      } else {
        window.alert(data.message || data.error || 'Promotion failed.');
      }
    } catch (e) {
      window.alert('Promotion request failed.');
    }
  }, [fetchSkills]);

  useEffect(() => {
    fetchSkills();
    const t = setInterval(fetchSkills, 60000);
    return () => clearInterval(t);
  }, [fetchSkills]);

  const tierColor = (tier: string): string => {
    switch (tier?.toLowerCase()) {
      case 'core': return 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border-emerald-500/30';
      case 'import': return 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border-amber-500/30';
      case 'generated': return 'bg-violet-500/20 text-violet-600 dark:text-violet-400 border-violet-500/30';
      default: return 'bg-zinc-500/20 text-zinc-600 dark:text-zinc-400 border-zinc-500/30';
    }
  };

  const fetchVitalitySummary = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/domain/vitality`);
      if (res.ok) {
        const data = await res.json();
        setVitalitySummary({
          status: data.status ?? 'ok',
          vitality: (data.vitality ?? 'stable').toLowerCase(),
          message: data.message,
        });
      }
    } catch {
      setVitalitySummary(null);
    }
  }, []);

  useEffect(() => {
    fetchVitalitySummary();
    const t = setInterval(fetchVitalitySummary, 30000);
    return () => clearInterval(t);
  }, [fetchVitalitySummary]);

  // Strategic alignment helpers
  const alignmentLevel = domainIntegrity?.strategic_alignment?.level ?? 'Unknown';
  const alignmentScore = domainIntegrity?.strategic_alignment?.score ?? 0;
  const alignmentColor = alignmentLevel === 'High'
    ? 'text-emerald-600 dark:text-emerald-400'
    : alignmentLevel === 'Medium'
    ? 'text-amber-600 dark:text-amber-400'
    : 'text-red-600 dark:text-red-400';

  const fetchDomainIntegrity = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/sentinel/domain-integrity`);
      if (res.ok) {
        const data = await res.json();
        setDomainIntegrity({
          absurdity_log_count: data.absurdity_log_count ?? 0,
          resource_drain_alerts: Array.isArray(data.resource_drain_alerts) ? data.resource_drain_alerts : [],
        });
      }
    } catch {
      setDomainIntegrity(null);
    }
  }, []);

  useEffect(() => {
    fetchDomainIntegrity();
    const t = setInterval(fetchDomainIntegrity, 30000);
    return () => clearInterval(t);
  }, [fetchDomainIntegrity]);

  const fetchActiveArchetype = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/archetype`);
      if (res.ok) {
        const data = await res.json();
        setActiveArchetype(data.active_archetype ?? null);
      }
    } catch {
      setActiveArchetype(null);
    }
  }, []);

  useEffect(() => {
    fetchActiveArchetype();
    const t = setInterval(fetchActiveArchetype, 60000);
    return () => clearInterval(t);
  }, [fetchActiveArchetype]);

  const fetchAstroWeather = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/astro-weather`);
      if (res.ok) {
        const data = await res.json();
        setAstroWeather({
          status: data.status ?? 'ok',
          risk: (data.risk ?? 'stable').toLowerCase(),
          transit_summary: data.transit_summary ?? '',
          advice: data.advice ?? '',
          updated_at_ms: data.updated_at_ms,
        });
      }
    } catch {
      setAstroWeather(null);
    }
  }, []);

  useEffect(() => {
    fetchAstroWeather();
    const t = setInterval(fetchAstroWeather, 60000);
    return () => clearInterval(t);
  }, [fetchAstroWeather]);

  return (
    <div className={`flex flex-col gap-2 p-3 rounded-lg border border-zinc-200 dark:border-zinc-800 ${accentClass}`}>
      {/* Active Archetype (KB-01) – e.g. Mode: Pisces-Protector */}
      {activeArchetype != null && (
        <div
          className="flex items-center justify-between gap-2 px-2 py-1.5 rounded border border-amber-500/20 bg-amber-500/5 text-amber-700 dark:text-amber-400 text-xs font-medium"
          title="Active Archetype from KB-01 (Astro-Logic)"
        >
          <span className="truncate">Mode: {activeArchetype}</span>
        </div>
      )}

      {/* Astro-Weather: Today's Transit (High Risk / Stable) */}
      {astroWeather != null && (
        <div
          className={`flex items-center justify-between gap-2 px-2 py-1.5 rounded border text-xs font-medium ${
            astroWeather.risk === 'high_risk'
              ? 'bg-red-500/20 text-red-600 dark:text-red-400 border-red-500/30'
              : astroWeather.risk === 'elevated'
                ? 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border-amber-500/30'
                : 'bg-zinc-500/10 text-zinc-600 dark:text-zinc-400 border-zinc-500/20'
          }`}
          title={astroWeather.transit_summary ? `${astroWeather.transit_summary}. ${astroWeather.advice}` : astroWeather.advice}
        >
          <span className="flex items-center gap-1.5">
            <Cloud size={12} />
            Astro-Weather
          </span>
          <span className="font-semibold">
            {astroWeather.risk === 'high_risk' ? 'High Risk' : astroWeather.risk === 'elevated' ? 'Elevated' : 'Stable'}
          </span>
        </div>
      )}

      {/* Phoenix Vitality badge (capacity/load/status) */}
      <div className={`flex items-center justify-between gap-2 px-2 py-1.5 rounded border text-xs font-medium ${domainHealthColor(sentinelStatus, isCritical)}`} title="Phoenix Vitality – System capacity/load">
        <div className="flex items-center gap-1.5">
          <Shield size={12} />
          <span>System Vitality</span>
        </div>
        <span className="font-semibold">{domainHealthLabel(sentinelStatus, isCritical)}</span>
      </div>

      {/* Strategic Alignment - North Star Compass */}
      {domainIntegrity?.strategic_alignment && (
        <div
          className="flex items-center justify-between text-[10px] px-1 cursor-help"
          title={domainIntegrity.strategic_alignment.divergence_warning || `Strategic Alignment: ${alignmentLevel} (${alignmentScore.toFixed(1)}%)`}
        >
          <span className="flex items-center gap-1 text-zinc-600 dark:text-zinc-400">
            <Compass size={9} />
            North Star
          </span>
          <span className={`font-semibold ${alignmentColor}`}>
            {alignmentLevel}
          </span>
        </div>
      )}

      {/* System Vitality (generic capacity/load/status) - Single line */}
      {vitalitySummary != null && (
        <div
          className={`flex items-center justify-between text-[10px] px-1 font-medium ${
            vitalitySummary.vitality === 'critical'
              ? 'text-red-600 dark:text-red-400'
              : vitalitySummary.vitality === 'draining'
                ? 'text-amber-600 dark:text-amber-400'
                : 'text-zinc-600 dark:text-zinc-400'
          }`}
          title={vitalitySummary.message ?? 'Phoenix Vitality – sovereign attributes'}
        >
          <span className="flex items-center gap-1">
            <Wallet size={9} />
            Phoenix Vitality
          </span>
          <span className="capitalize">{vitalitySummary.vitality}</span>
        </div>
      )}

      {/* Domain Integrity (KB-08 absurdity log count) */}
      {domainIntegrity != null && (
        <div className="flex items-center justify-between text-[10px] text-zinc-600 dark:text-zinc-400 px-1">
          <span className="flex items-center gap-1">
            <Lock size={9} />
            Domain Integrity
          </span>
          <span className="font-mono">{domainIntegrity.absurdity_log_count}</span>
        </div>
      )}

      {/* Skills Inventory (3-Tier: Green Core, Amber Import, Purple Generated) */}
      <div className="border-t border-zinc-200 dark:border-zinc-800 pt-2 mt-1">
        <button
          type="button"
          onClick={() => setSkillsExpanded(!skillsExpanded)}
          className="flex items-center justify-between w-full text-[10px] font-medium text-zinc-600 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 px-1 py-0.5 rounded"
          title="3-Tier: Core (green), Import (amber), Generated (purple)"
        >
          <span className="flex items-center gap-1">
            <Layers size={9} />
            Skills Inventory
          </span>
          <span className="font-mono text-zinc-500">{skillsInventory.length}</span>
        </button>
        {skillsExpanded && skillsInventory.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1.5 max-h-32 overflow-y-auto">
            {skillsInventory.map((s) => (
              <span
                key={s.skill_id}
                className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-[10px] font-medium ${tierColor(s.trust_tier)}`}
                title={s.description ?? `${s.skill_id} (${s.trust_tier})`}
              >
                {s.skill_id}
                {s.trust_tier?.toLowerCase() === 'generated' && (
                  <button
                    type="button"
                    onClick={(ev) => {
                      ev.stopPropagation();
                      promoteSkill(s.skill_id);
                    }}
                    className="ml-0.5 px-1 py-0 rounded bg-violet-600 hover:bg-violet-500 text-white text-[9px] font-semibold"
                    title="Promote to Core (requires confirmation)"
                  >
                    Promote
                  </button>
                )}
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default WardenSidebar;
