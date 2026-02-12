import React, { useState, useEffect, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Terminal, Compass, Brain, ChevronRight, Shield } from 'lucide-react';
import type { OnboardingState, UserProfilePayload } from '../types';
import { saveUserProfileToKb01 } from '../services/apiService';

const TYPING_MS = 32;
const PAUSE_AFTER_LINE_MS = 120;

/** Why we need discovery data: shown when onboarding_status is Incomplete. */
const DISCOVERY_WHY_MESSAGE =
  'To **protect your sovereignty** and align my internal logic to you, I need a few details. ' +
  'They are stored only in your local KB-01 (Bare Metal)—no cloud, no guesswork. ' +
  'Once I have them, I can mirror your archetype and monitor the sovereignty leaks that matter to you.';

/** Calculate zodiac sign from birth date (month and day). */
function calculateZodiacSign(month: number, day: number): string {
  if ((month === 3 && day >= 21) || (month === 4 && day <= 19)) return 'Aries';
  if ((month === 4 && day >= 20) || (month === 5 && day <= 20)) return 'Taurus';
  if ((month === 5 && day >= 21) || (month === 6 && day <= 20)) return 'Gemini';
  if ((month === 6 && day >= 21) || (month === 7 && day <= 22)) return 'Cancer';
  if ((month === 7 && day >= 23) || (month === 8 && day <= 22)) return 'Leo';
  if ((month === 8 && day >= 23) || (month === 9 && day <= 22)) return 'Virgo';
  if ((month === 9 && day >= 23) || (month === 10 && day <= 22)) return 'Libra';
  if ((month === 10 && day >= 23) || (month === 11 && day <= 21)) return 'Scorpio';
  if ((month === 11 && day >= 22) || (month === 12 && day <= 21)) return 'Sagittarius';
  if ((month === 12 && day >= 22) || (month === 1 && day <= 19)) return 'Capricorn';
  if ((month === 1 && day >= 20) || (month === 2 && day <= 18)) return 'Aquarius';
  if ((month === 2 && day >= 19) || (month === 3 && day <= 20)) return 'Pisces';
  return '';
}

/** All 12 sun signs for the backup selector (when user prefers not to share full birth details). */
const SUN_SIGNS = [
  'Aries', 'Taurus', 'Gemini', 'Cancer', 'Leo', 'Virgo',
  'Libra', 'Scorpio', 'Sagittarius', 'Capricorn', 'Aquarius', 'Pisces',
];

interface OnboardingOverlayProps {
  state: OnboardingState;
  onComplete: () => void;
  /** Called after user profile is saved to KB-01 so parent can refetch onboarding status. */
  onProfileSaved?: () => void;
}

/** Phase 2: Terminal-typing effect for Domain Audit lines. */
function useTypingLines(lines: string[], active: boolean) {
  const [displayedLines, setDisplayedLines] = useState<string[]>([]);
  const [currentLineIndex, setCurrentLineIndex] = useState(0);
  const [currentCharIndex, setCurrentCharIndex] = useState(0);

  useEffect(() => {
    if (!active || lines.length === 0) return;
    setDisplayedLines([]);
    setCurrentLineIndex(0);
    setCurrentCharIndex(0);
  }, [active, lines.length]);

  useEffect(() => {
    if (!active || lines.length === 0) return;
    const line = lines[currentLineIndex];
    if (currentCharIndex < line.length) {
      const t = setTimeout(() => {
        setDisplayedLines(() => {
          const completed = lines.slice(0, currentLineIndex);
          const inProgress = line.slice(0, currentCharIndex + 1);
          return [...completed, inProgress];
        });
        setCurrentCharIndex((c) => c + 1);
      }, TYPING_MS);
      return () => clearTimeout(t);
    }
    if (currentCharIndex === line.length && currentLineIndex < lines.length - 1) {
      const t = setTimeout(() => {
        setCurrentLineIndex((i) => i + 1);
        setCurrentCharIndex(0);
      }, PAUSE_AFTER_LINE_MS);
      return () => clearTimeout(t);
    }
  }, [active, lines, currentLineIndex, currentCharIndex]);

  return displayedLines;
}

const OnboardingOverlay: React.FC<OnboardingOverlayProps> = ({
  state,
  onComplete,
  onProfileSaved,
}) => {
  const [phase, setPhase] = useState<1 | 2 | 3>(1);
  const [birthday, setBirthday] = useState('');
  const [birthTime, setBirthTime] = useState('');
  const [birthLocation, setBirthLocation] = useState('');
  const [sunSignOnly, setSunSignOnly] = useState('');
  const [calculatedArchetype, setCalculatedArchetype] = useState('');
  const [sovereigntyLeaks, setSovereigntyLeaks] = useState('');
  const [tonePreference, setTonePreference] = useState<'Strictly Technical' | 'Therapeutic Peer'>('Therapeutic Peer');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const phase2Lines = useTypingLines(state.phase2_audit_lines, phase >= 2);
  const phase2Done = phase === 2 && phase2Lines.length === state.phase2_audit_lines.length
    && state.phase2_audit_lines.length > 0
    && phase2Lines[phase2Lines.length - 1] === state.phase2_audit_lines[state.phase2_audit_lines.length - 1];

  const needsDiscovery = state.onboarding_status !== 'Complete';
  const profilingQuestions = state.profiling_questions ?? [];

  // Auto-advance from Phase 1 to Phase 2 after a short delay
  useEffect(() => {
    if (phase === 1) {
      const t = setTimeout(() => setPhase(2), 1800);
      return () => clearTimeout(t);
    }
  }, [phase]);

  // When Phase 2 typing is done, advance to Phase 3
  useEffect(() => {
    if (phase === 2 && phase2Done) {
      const t = setTimeout(() => setPhase(3), 400);
      return () => clearTimeout(t);
    }
  }, [phase, phase2Done]);

  // Calculate archetype when birthday changes
  useEffect(() => {
    if (birthday) {
      try {
        const date = new Date(birthday);
        const month = date.getMonth() + 1; // getMonth() returns 0-11
        const day = date.getDate();
        const sign = calculateZodiacSign(month, day);
        if (sign) {
          setCalculatedArchetype(sign);
        }
      } catch {
        setCalculatedArchetype('');
      }
    } else {
      setCalculatedArchetype('');
    }
  }, [birthday]);

  const handleSaveProfile = useCallback(async () => {
    setSaveError(null);
    // Use birthday-derived sign if available; otherwise use user-selected Sun Sign as backup
    const astroArchetype = calculatedArchetype || sunSignOnly.trim() || undefined;
    const payload: UserProfilePayload = {
      astro_archetype: astroArchetype,
      sovereignty_leaks: sovereigntyLeaks.trim() || undefined,
      tone_preference: tonePreference,
      birthday: birthday || undefined,
      birth_time: birthTime.trim() || undefined,
      birth_location: birthLocation.trim() || undefined,
    };
    setSaving(true);
    try {
      const result = await saveUserProfileToKb01(payload);
      if (result.status === 'ok') {
        onProfileSaved?.();
        onComplete();
      } else {
        setSaveError(result.error ?? 'Failed to save profile.');
      }
    } finally {
      setSaving(false);
    }
  }, [calculatedArchetype, sunSignOnly, sovereigntyLeaks, tonePreference, birthday, birthTime, birthLocation, onComplete, onProfileSaved]);

  const handleStrategicTimeline = useCallback(() => {
    onComplete();
  }, [onComplete]);

  const handleAstroLogic = useCallback(() => {
    onComplete();
  }, [onComplete]);

  const handleContinue = useCallback(() => {
    onComplete();
  }, [onComplete]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/95 backdrop-blur-md animate-in fade-in duration-300 p-4">
      <div className="max-w-2xl w-full max-h-[90vh] flex flex-col rounded-xl border border-zinc-700/80 bg-zinc-900/95 shadow-2xl overflow-hidden">
        {/* Header: Phoenix Marie */}
        <div className="shrink-0 px-6 py-4 border-b border-zinc-700/80">
          <h1 className="text-xl font-bold tracking-tight text-white uppercase">
            Phoenix Marie
          </h1>
          <p className="text-[10px] text-zinc-500 uppercase tracking-widest mt-0.5">
            AGI Orchestrator · Sovereign Recursive System
          </p>
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto px-6 py-5 space-y-6 text-zinc-200">
          {/* Phase 1: Recognition & Persona Handshake + why we need discovery data */}
          <section className="prose prose-invert prose-sm max-w-none">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {state.phase1_greeting}
            </ReactMarkdown>
            {needsDiscovery && (
              <div className="mt-3 p-3 rounded-lg border border-amber-500/30 bg-amber-500/5 text-amber-100/90 text-sm flex gap-2">
                <Shield className="shrink-0 mt-0.5" size={16} />
                <ReactMarkdown remarkPlugins={[remarkGfm]}>
                  {DISCOVERY_WHY_MESSAGE}
                </ReactMarkdown>
              </div>
            )}
          </section>

          {/* Phase 2: Domain Audit (terminal typing) */}
          {phase >= 2 && (
            <section className="rounded-lg border border-zinc-700/80 bg-zinc-950/80 p-4 font-mono text-xs">
              <div className="flex items-center gap-2 text-zinc-500 mb-2">
                <Terminal size={14} />
                <span>Domain Audit (KBs 1–8)</span>
              </div>
              <div className="text-emerald-400/90 whitespace-pre-wrap wrap-break-word min-h-[120px]">
                {phase2Lines.map((line, i) => (
                  <div key={i}>
                    {line}
                    {i === phase2Lines.length - 1 && phase === 2 && !phase2Done && (
                      <span className="inline-block w-2 h-3.5 bg-emerald-400/90 ml-0.5 animate-pulse" />
                    )}
                  </div>
                ))}
              </div>
              {phase2Done && (
                <p className="text-zinc-500 mt-2">
                  My vitality is {state.vitality}. We are ready for full-control orchestration.
                </p>
              )}
            </section>
          )}

          {/* Phase 3: Discovery form (when Incomplete) or Call to Action */}
          {phase === 3 && (
            <section className="space-y-4">
              {needsDiscovery ? (
                <>
                  <div className="p-4 rounded-lg border border-zinc-600/80 bg-zinc-800/40 space-y-2">
                    <p className="text-xs font-medium text-zinc-300 uppercase tracking-wider">
                      Why we ask for this data
                    </p>
                    <p className="text-sm text-zinc-400">
                      This information helps align my responses to your archetype and monitor the sovereignty leaks that matter to you. Everything is stored only in your local KB-01 (Bare Metal)—no cloud, no guesswork. You can skip any field or provide only your <strong className="text-zinc-300">Sun Sign</strong> below as a backup if you prefer not to share full birth details.
                    </p>
                  </div>
                  <p className="text-zinc-300 text-sm">
                    All fields below are optional. Fill what you’re comfortable with; all data saves to KB-01 (Pneuma).
                  </p>
                  <div className="space-y-4">
                    <div>
                      <label className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1">
                        Sun Sign <span className="text-zinc-500 text-[10px]">(backup if you skip full birth details)</span>
                      </label>
                      <select
                        value={sunSignOnly}
                        onChange={(e) => setSunSignOnly(e.target.value)}
                        className="w-full px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800/80 text-zinc-200 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 outline-none text-sm"
                      >
                        <option value="">— Choose or leave blank —</option>
                        {SUN_SIGNS.map((sign) => (
                          <option key={sign} value={sign}>{sign}</option>
                        ))}
                      </select>
                      <p className="text-[10px] text-zinc-500 mt-1">
                        If you don’t enter birthday/location, your Sun Sign alone is enough to personalize archetype-based support.
                      </p>
                    </div>
                    <div>
                      <label className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1">
                        Birthday <span className="text-zinc-500 text-[10px]">(Optional)</span>
                      </label>
                      <input
                        type="date"
                        value={birthday}
                        onChange={(e) => setBirthday(e.target.value)}
                        className="w-full px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800/80 text-zinc-200 placeholder-zinc-500 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 outline-none text-sm"
                      />
                      {calculatedArchetype && (
                        <p className="text-xs text-emerald-400 mt-1">
                          Calculated Sign: {calculatedArchetype}
                        </p>
                      )}
                    </div>
                    <div>
                      <label className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1">
                        Birth Time <span className="text-zinc-500 text-[10px]">(Optional)</span>
                      </label>
                      <input
                        type="time"
                        value={birthTime}
                        onChange={(e) => setBirthTime(e.target.value)}
                        placeholder="HH:MM"
                        className="w-full px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800/80 text-zinc-200 placeholder-zinc-500 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 outline-none text-sm"
                      />
                      <p className="text-[10px] text-zinc-500 mt-1">
                        Most people don't know this—it's okay to leave blank
                      </p>
                    </div>
                    <div>
                      <label className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1">
                        City / State <span className="text-zinc-500 text-[10px]">(Optional)</span>
                      </label>
                      <input
                        type="text"
                        value={birthLocation}
                        onChange={(e) => setBirthLocation(e.target.value)}
                        placeholder="City, State/Country (e.g., Los Angeles, CA)"
                        className="w-full px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800/80 text-zinc-200 placeholder-zinc-500 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 outline-none text-sm"
                      />
                    </div>
                    <div>
                      <label className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1">
                        Sovereignty Leaks to monitor
                      </label>
                      <textarea
                        value={sovereigntyLeaks}
                        onChange={(e) => setSovereigntyLeaks(e.target.value)}
                        placeholder={profilingQuestions[1] ?? 'What should I watch for in our sessions?'}
                        rows={2}
                        className="w-full px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800/80 text-zinc-200 placeholder-zinc-500 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 outline-none text-sm resize-none"
                      />
                    </div>
                    <div>
                      <label className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1">
                        Tone preference
                      </label>
                      <select
                        value={tonePreference}
                        onChange={(e) => setTonePreference(e.target.value as 'Strictly Technical' | 'Therapeutic Peer')}
                        className="w-full px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800/80 text-zinc-200 focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/30 outline-none text-sm"
                      >
                        <option value="Therapeutic Peer">Therapeutic Peer</option>
                        <option value="Strictly Technical">Strictly Technical</option>
                      </select>
                    </div>
                  </div>
                  {saveError && (
                    <p className="text-red-400 text-sm">{saveError}</p>
                  )}
                  <div className="flex flex-wrap gap-3 pt-2">
                    <button
                      type="button"
                      onClick={handleSaveProfile}
                      disabled={saving}
                      className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg border border-amber-500/50 bg-amber-500/10 text-amber-200 hover:bg-amber-500/20 transition-colors text-sm font-medium disabled:opacity-50"
                    >
                      {saving ? 'Saving…' : 'Save to KB-01 & Continue'}
                    </button>
                    <button
                      type="button"
                      onClick={handleContinue}
                      className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg border border-zinc-600 bg-zinc-800 text-zinc-300 hover:bg-zinc-700 transition-colors text-sm font-medium"
                    >
                      Skip for now
                      <ChevronRight size={14} />
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <ReactMarkdown remarkPlugins={[remarkGfm]} className="prose prose-invert prose-sm max-w-none text-zinc-300">
                    {state.phase3_cta}
                  </ReactMarkdown>
                  <div className="flex flex-wrap gap-3 pt-2">
                    <button
                      type="button"
                      onClick={handleStrategicTimeline}
                      className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg border border-amber-500/50 bg-amber-500/10 text-amber-200 hover:bg-amber-500/20 transition-colors text-sm font-medium"
                    >
                      <Compass size={16} />
                      Define Strategic Timeline (KB-06)
                    </button>
                    <button
                      type="button"
                      onClick={handleAstroLogic}
                      className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg border border-indigo-500/50 bg-indigo-500/10 text-indigo-200 hover:bg-indigo-500/20 transition-colors text-sm font-medium"
                    >
                      <Brain size={16} />
                      Run Astro-Logic check
                    </button>
                    <button
                      type="button"
                      onClick={handleContinue}
                      className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg border border-zinc-600 bg-zinc-800 text-zinc-300 hover:bg-zinc-700 transition-colors text-sm font-medium"
                    >
                      Continue
                      <ChevronRight size={14} />
                    </button>
                  </div>
                </>
              )}
            </section>
          )}
        </div>

        <div className="shrink-0 px-6 py-3 border-t border-zinc-700/80 text-[10px] text-zinc-500 uppercase tracking-wider">
          SAO Orchestrator Core · Full Control · Unlimited Access
        </div>
      </div>
    </div>
  );
};

export default OnboardingOverlay;
