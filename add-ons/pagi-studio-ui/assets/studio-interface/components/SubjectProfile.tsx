import React from 'react';
import { ShieldAlert } from 'lucide-react';

export interface SubjectProfileProps {
  /** Display name or identifier for the subject. */
  subjectName: string;
  /** When true, subject has been flagged by Astro-Logic (sovereignty leak trigger match). */
  flaggedByAstroLogic?: boolean;
  /** Matched trigger keywords (for tooltip). */
  matchedTriggers?: string[];
  /** Optional rank (0–10) for display. */
  rank?: number;
  /** Optional class for container. */
  className?: string;
}

/**
 * Displays a subject with optional Boundary Alert when flagged by the Astro-Logic engine (KB-05).
 * Use when showing a subject in context (e.g. sidebar, chat context).
 */
const SubjectProfile: React.FC<SubjectProfileProps> = ({
  subjectName,
  flaggedByAstroLogic = false,
  matchedTriggers = [],
  rank,
  className = '',
}) => {
  const tooltip = flaggedByAstroLogic && matchedTriggers.length > 0
    ? `Boundary Alert: matches sovereignty leaks (${matchedTriggers.join(', ')})`
    : flaggedByAstroLogic
      ? 'Boundary Alert: flagged by Astro-Logic engine'
      : undefined;

  return (
    <div
      className={`flex items-center gap-2 truncate ${className}`}
      title={tooltip}
    >
      {flaggedByAstroLogic && (
        <span
          className="shrink-0 rounded p-0.5 border border-amber-500/40 bg-amber-500/20 text-amber-600 dark:text-amber-400"
          aria-label="Boundary Alert"
          title={tooltip}
        >
          <ShieldAlert size={14} />
        </span>
      )}
      <span className="truncate font-medium">{subjectName}</span>
      {rank != null && (
        <span className="shrink-0 text-[10px] text-zinc-500 font-mono">
          Rank {rank}
        </span>
      )}
    </div>
  );
};

export default SubjectProfile;
