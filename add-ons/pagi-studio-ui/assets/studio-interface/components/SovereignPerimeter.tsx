import React, { useState, useEffect } from 'react';
import { Shield, Plus, Trash2, RefreshCw, Copy, CheckCircle2, XCircle } from 'lucide-react';
import {
  getVaultProtectedTerms,
  postVaultProtectedTerms,
  postVaultRedactTest,
  getMimirStatus,
  getProjectAssociations,
  type ProjectAssociationRecord,
} from '../services/apiService';
import BridgeConfirmModal from './BridgeConfirmModal';

const BRIDGE_AUTO_REDACT_KEY = 'pagi_bridge_auto_redact';
const PROTECTED_PLACEHOLDER = '[PROTECTED_TERM]';

export interface SovereignPerimeterProps {
  /** Optional display name for confirmation message (e.g. "The Creator"). */
  userName?: string;
  /** Active project id (from current chat thread) for Scope default and sandbox merge. */
  activeProjectId?: string | null;
  /** Optional project names by id (e.g. from Chronos) for Scope dropdown labels. */
  projectNames?: Record<string, string>;
}

const SovereignPerimeter: React.FC<SovereignPerimeterProps> = ({
  userName = 'Operator',
  activeProjectId = null,
  projectNames = {},
}) => {
  const [scope, setScope] = useState<'global' | string>('global');
  const [associations, setAssociations] = useState<ProjectAssociationRecord[]>([]);
  const [protectedTerms, setProtectedTerms] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [newTerm, setNewTerm] = useState('');
  const [bulkImportText, setBulkImportText] = useState('');
  const [bridgeStatus, setBridgeStatus] = useState<'active' | 'idle' | 'loading'>('idle');
  const [autoRedact, setAutoRedact] = useState(true);
  const [sandboxInput, setSandboxInput] = useState('');
  const [sandboxOutput, setSandboxOutput] = useState('');
  const [sandboxTermsRedacted, setSandboxTermsRedacted] = useState<number | null>(null);
  const [sandboxLoading, setSandboxLoading] = useState(false);
  const [confirmModalOpen, setConfirmModalOpen] = useState(false);
  const [confirmMessage, setConfirmMessage] = useState('');

  const fetchAssociations = async () => {
    try {
      const list = await getProjectAssociations();
      setAssociations(list);
    } catch {
      setAssociations([]);
    }
  };

  const fetchTerms = async () => {
    setLoading(true);
    try {
      const projectId = scope === 'global' ? undefined : scope;
      const data = await getVaultProtectedTerms(projectId);
      if (data.scope === 'project' && data.local != null) {
        setProtectedTerms(data.local);
      } else {
        setProtectedTerms(data.terms ?? []);
      }
    } catch {
      setProtectedTerms([]);
    } finally {
      setLoading(false);
    }
  };

  const fetchBridgeStatus = async () => {
    setBridgeStatus('loading');
    try {
      const data = await getMimirStatus();
      setBridgeStatus(data.recording ? 'active' : 'idle');
    } catch {
      setBridgeStatus('idle');
    }
  };

  useEffect(() => {
    fetchAssociations();
    const stored = localStorage.getItem(BRIDGE_AUTO_REDACT_KEY);
    setAutoRedact(stored !== 'false');
  }, []);

  useEffect(() => {
    if (activeProjectId && associations.some((a) => a.project_id === activeProjectId) && scope === 'global') {
      setScope(activeProjectId);
    }
  }, [activeProjectId, associations]);

  useEffect(() => {
    fetchTerms();
  }, [scope]);

  useEffect(() => {
    fetchBridgeStatus();
    const t = setInterval(fetchBridgeStatus, 5000);
    return () => clearInterval(t);
  }, []);

  const projectIdForSave = scope === 'global' ? undefined : scope;

  const handleAddTerm = async () => {
    const t = newTerm.trim().toUpperCase();
    if (!t) return;
    const next = [...protectedTerms, t];
    setSaving(true);
    try {
      await postVaultProtectedTerms(next, projectIdForSave);
      setProtectedTerms(next);
      setNewTerm('');
    } catch (e) {
      console.error('Add term failed:', e);
    } finally {
      setSaving(false);
    }
  };

  const handleRemoveTerm = async (term: string) => {
    const next = protectedTerms.filter((x) => x !== term);
    setSaving(true);
    try {
      await postVaultProtectedTerms(next, projectIdForSave);
      setProtectedTerms(next);
    } catch (e) {
      console.error('Remove term failed:', e);
    } finally {
      setSaving(false);
    }
  };

  const handleBulkImport = async () => {
    const lines = bulkImportText
      .split(/[\n,;]+/)
      .map((s) => s.trim().toUpperCase())
      .filter((s) => s.length > 0);
    if (lines.length === 0) return;
    const merged = [...new Set([...protectedTerms, ...lines])];
    setSaving(true);
    try {
      await postVaultProtectedTerms(merged, projectIdForSave);
      setProtectedTerms(merged);
      setBulkImportText('');
    } catch (e) {
      console.error('Bulk import failed:', e);
    } finally {
      setSaving(false);
    }
  };

  const handleAutoRedactToggle = () => {
    const next = !autoRedact;
    setAutoRedact(next);
    localStorage.setItem(BRIDGE_AUTO_REDACT_KEY, String(next));
  };

  const handleSimulateRedaction = async () => {
    if (!sandboxInput.trim()) return;
    setSandboxLoading(true);
    setSandboxOutput('');
    setSandboxTermsRedacted(null);
    try {
      const projectId = scope === 'global' ? undefined : scope;
      const data = await postVaultRedactTest(sandboxInput, projectId);
      setSandboxOutput(data.sanitized);
      const count = (data.sanitized.match(new RegExp(PROTECTED_PLACEHOLDER.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g')) || []).length;
      setSandboxTermsRedacted(count);
      setConfirmMessage(
        count > 0
          ? `${userName}, ${count} term(s) would be sanitized. This is how it would look before reaching Copilot.`
          : `${userName}, no protected terms in this text. It would be sent as-is (after you confirm).`
      );
      setConfirmModalOpen(true);
    } catch (e) {
      setSandboxOutput(`Error: ${e instanceof Error ? e.message : 'Failed'}`);
    } finally {
      setSandboxLoading(false);
    }
  };

  return (
    <div className="p-4 space-y-6">
      <div className="flex items-center gap-2 text-zinc-700 dark:text-zinc-300">
        <Shield size={18} className="text-amber-500" />
        <h2 className="text-sm font-semibold">Sovereign Perimeter</h2>
      </div>
      <p className="text-xs text-zinc-500 dark:text-zinc-400">
        Manage SAO protected terms (redacted in meeting transcripts and Copilot bridge). Bridge status reflects Mimir capture.
      </p>

      {/* Scope: Global vs Project */}
      <div className="space-y-2">
        <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider font-semibold">Scope</label>
        <select
          value={scope}
          onChange={(e) => setScope(e.target.value)}
          className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-amber-500/50"
        >
          <option value="global">Global (all contexts)</option>
          {associations.map((a) => (
            <option key={a.project_id} value={a.project_id}>
              {projectNames[a.project_id] ?? a.project_id}
            </option>
          ))}
        </select>
        <p className="text-[10px] text-zinc-500 dark:text-zinc-400">
          {scope === 'global'
            ? 'Terms apply everywhere. Use project scope to add terms only for a specific project.'
            : 'Editing this project\'s .sao_policy. Redaction uses global + these terms when this project is active.'}
        </p>
      </div>

      {/* Protected Terms */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-wider">
          {scope === 'global' ? 'Protected terms' : 'Project terms'}
        </h3>
        {loading ? (
          <p className="text-xs text-zinc-500">Loading…</p>
        ) : (
          <>
            <ul className="space-y-1 max-h-36 overflow-y-auto rounded border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-950 p-2">
              {protectedTerms.length === 0 ? (
                <li className="text-xs text-zinc-500 italic">No terms. Add or bulk import below.</li>
              ) : (
                protectedTerms.map((term) => (
                  <li key={term} className="flex items-center justify-between gap-2 text-xs">
                    <span className="font-mono text-zinc-800 dark:text-zinc-200">{term}</span>
                    <button
                      type="button"
                      onClick={() => handleRemoveTerm(term)}
                      disabled={saving}
                      className="p-1 rounded text-zinc-400 hover:text-red-500 hover:bg-red-500/10 disabled:opacity-50"
                      title="Remove term"
                    >
                      <Trash2 size={12} />
                    </button>
                  </li>
                ))
              )}
            </ul>
            <div className="flex gap-2">
              <input
                type="text"
                value={newTerm}
                onChange={(e) => setNewTerm(e.target.value.toUpperCase())}
                onKeyDown={(e) => e.key === 'Enter' && handleAddTerm()}
                placeholder="New term (e.g. PROJECT_ALPHA)"
                className="flex-1 bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-amber-500/50"
              />
              <button
                type="button"
                onClick={handleAddTerm}
                disabled={saving || !newTerm.trim()}
                className="flex items-center gap-1 px-2 py-1.5 rounded border border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300 text-xs font-medium hover:bg-amber-500/20 disabled:opacity-50"
              >
                <Plus size={12} /> Add
              </button>
            </div>
            <div className="space-y-1">
              <label className="text-[10px] text-zinc-400 dark:text-zinc-500 uppercase tracking-wider">Bulk import (one per line or comma-separated)</label>
              <textarea
                value={bulkImportText}
                onChange={(e) => setBulkImportText(e.target.value)}
                placeholder="PROJECT_ALPHA&#10;OMEGA&#10;VANGUARD"
                className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-amber-500/50 resize-y min-h-[60px]"
                rows={2}
              />
              <button
                type="button"
                onClick={handleBulkImport}
                disabled={saving || !bulkImportText.trim()}
                className="flex items-center gap-1 px-2 py-1.5 rounded border border-zinc-300 dark:border-zinc-700 text-zinc-600 dark:text-zinc-400 text-xs font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800 disabled:opacity-50"
              >
                <Copy size={12} /> Bulk import
              </button>
            </div>
          </>
        )}
      </div>

      {/* Bridge configuration */}
      <div className="space-y-3 pt-2 border-t border-zinc-200 dark:border-zinc-800">
        <h3 className="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-wider">Bridge configuration</h3>
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs text-zinc-700 dark:text-zinc-300">Auto-redact Copilot submissions</span>
          <button
            type="button"
            role="switch"
            aria-checked={autoRedact}
            onClick={handleAutoRedactToggle}
            className={`relative w-10 h-5 rounded-full transition-colors ${autoRedact ? 'bg-amber-500' : 'bg-zinc-300 dark:bg-zinc-600'}`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform ${autoRedact ? 'translate-x-5' : 'translate-x-0'}`}
            />
          </button>
        </div>
        <p className="text-[10px] text-zinc-500 dark:text-zinc-400">When ON, all text sent to Copilot is sanitized with the terms above. Default: ON.</p>
        <div className="flex items-center gap-2">
          <span className="text-xs text-zinc-500 dark:text-zinc-400">Bridge status:</span>
          {bridgeStatus === 'loading' ? (
            <RefreshCw size={12} className="animate-spin text-zinc-400" />
          ) : bridgeStatus === 'active' ? (
            <span className="flex items-center gap-1 text-xs text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 size={12} /> Active
            </span>
          ) : (
            <span className="flex items-center gap-1 text-xs text-zinc-500 dark:text-zinc-400">
              <XCircle size={12} /> Idle
            </span>
          )}
        </div>
        <p className="text-[10px] text-zinc-500 dark:text-zinc-400">Uses Mimir status (Active = recording). Bridge automation runs only when you trigger it.</p>
      </div>

      {/* Bridge sandbox */}
      <div className="space-y-3 pt-2 border-t border-zinc-200 dark:border-zinc-800">
        <h3 className="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-wider">Bridge sandbox</h3>
        <p className="text-[10px] text-zinc-500 dark:text-zinc-400">Preview how text will look after redaction before it ever reaches Copilot.</p>
        <textarea
          value={sandboxInput}
          onChange={(e) => setSandboxInput(e.target.value)}
          placeholder="e.g. Meeting about PROJECT_ALPHA and OMEGA"
          className="w-full bg-zinc-50 dark:bg-zinc-950 border border-zinc-300 dark:border-zinc-800 rounded px-2 py-1.5 text-zinc-900 dark:text-zinc-300 text-xs focus:outline-none focus:ring-1 focus:ring-amber-500/50 resize-y min-h-[70px]"
          rows={3}
        />
        <button
          type="button"
          onClick={handleSimulateRedaction}
          disabled={sandboxLoading || !sandboxInput.trim()}
          className="w-full flex items-center justify-center gap-2 py-2 rounded border border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300 text-xs font-medium hover:bg-amber-500/20 disabled:opacity-50"
        >
          {sandboxLoading ? '…' : 'Simulate redaction'}
        </button>
        {sandboxOutput !== '' && (
          <div className="rounded border border-zinc-200 dark:border-zinc-800 bg-zinc-100 dark:bg-zinc-900 p-2 space-y-1">
            <p className="text-[10px] text-zinc-500 dark:text-zinc-400">
              Sanitized{sandboxTermsRedacted != null ? ` (${sandboxTermsRedacted} term(s) replaced)` : ''}:
            </p>
            <p className="text-xs font-mono text-zinc-800 dark:text-zinc-200 break-words whitespace-pre-wrap">{sandboxOutput}</p>
          </div>
        )}
      </div>

      <BridgeConfirmModal
        isOpen={confirmModalOpen}
        onClose={() => setConfirmModalOpen(false)}
        onConfirm={() => setConfirmModalOpen(false)}
        message={confirmMessage}
        title="Redaction preview"
        confirmLabel="OK"
        cancelLabel="Close"
      />
    </div>
  );
};

export default SovereignPerimeter;
