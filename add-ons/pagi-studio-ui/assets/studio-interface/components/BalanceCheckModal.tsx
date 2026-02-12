import React, { useState } from 'react';
import { X, Heart, Brain, Sparkles } from 'lucide-react';
import { API_BASE_URL } from '../src/api/config';

export interface BalanceCheckModalProps {
  isOpen: boolean;
  onClose: () => void;
  message?: string;
  onSaved?: () => void;
}

const BalanceCheckModal: React.FC<BalanceCheckModalProps> = ({
  isOpen,
  onClose,
  message = 'Checking in. Spirit/Mind/Body balance check—where are we at?',
  onSaved,
}) => {
  const [spirit, setSpirit] = useState(5);
  const [mind, setMind] = useState(5);
  const [body, setBody] = useState(5);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<'idle' | 'success' | 'error'>('idle');

  const handleSubmit = async () => {
    setSaving(true);
    setToast('idle');
    try {
      const res = await fetch(`${API_BASE_URL}/soma/balance`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ spirit, mind, body }),
      });
      const data = await res.json().catch(() => ({}));
      if (res.ok && data.status === 'ok') {
        setToast('success');
        onSaved?.();
        setTimeout(() => {
          onClose();
        }, 800);
      } else {
        setToast('error');
      }
    } catch {
      setToast('error');
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-100 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
      <div
        className="bg-white dark:bg-zinc-900 rounded-xl shadow-2xl border border-zinc-200 dark:border-zinc-800 w-full max-w-md overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between p-4 border-b border-zinc-200 dark:border-zinc-800">
          <h3 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100 flex items-center gap-2">
            <Sparkles size={20} className="text-amber-500" />
            Warden Check-in
          </h3>
          <button
            onClick={onClose}
            className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors p-1"
          >
            <X size={20} />
          </button>
        </div>

        <div className="p-4 space-y-4">
          <p className="text-sm text-zinc-600 dark:text-zinc-400">{message}</p>

          <div className="space-y-4">
            <div>
              <label className="flex items-center gap-2 text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-2">
                <Sparkles size={14} className="text-violet-400" />
                Spirit (1–10)
              </label>
              <div className="flex items-center gap-2">
                <input
                  type="range"
                  min={1}
                  max={10}
                  value={spirit}
                  onChange={(e) => setSpirit(Number(e.target.value))}
                  className="flex-1 h-2 bg-zinc-200 dark:bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-violet-500"
                />
                <span className="w-6 text-sm font-mono text-zinc-700 dark:text-zinc-300">{spirit}</span>
              </div>
            </div>

            <div>
              <label className="flex items-center gap-2 text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-2">
                <Brain size={14} className="text-blue-400" />
                Mind (1–10)
              </label>
              <div className="flex items-center gap-2">
                <input
                  type="range"
                  min={1}
                  max={10}
                  value={mind}
                  onChange={(e) => setMind(Number(e.target.value))}
                  className="flex-1 h-2 bg-zinc-200 dark:bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-blue-500"
                />
                <span className="w-6 text-sm font-mono text-zinc-700 dark:text-zinc-300">{mind}</span>
              </div>
            </div>

            <div>
              <label className="flex items-center gap-2 text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-2">
                <Heart size={14} className="text-emerald-400" />
                Body (1–10)
              </label>
              <div className="flex items-center gap-2">
                <input
                  type="range"
                  min={1}
                  max={10}
                  value={body}
                  onChange={(e) => setBody(Number(e.target.value))}
                  className="flex-1 h-2 bg-zinc-200 dark:bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-emerald-500"
                />
                <span className="w-6 text-sm font-mono text-zinc-700 dark:text-zinc-300">{body}</span>
              </div>
            </div>
          </div>

          {toast === 'success' && (
            <p className="text-sm text-emerald-600 dark:text-emerald-400">Saved to KB-8 (Soma).</p>
          )}
          {toast === 'error' && (
            <p className="text-sm text-red-500 dark:text-red-400">Failed to save. Check gateway connection.</p>
          )}

          <div className="flex gap-2 pt-2">
            <button
              onClick={handleSubmit}
              disabled={saving}
              className="flex-1 px-4 py-2 rounded-lg bg-amber-500 hover:bg-amber-600 dark:bg-amber-600 dark:hover:bg-amber-500 text-white text-sm font-medium transition-colors disabled:opacity-50"
            >
              {saving ? 'Saving…' : 'Save to Soma (KB-8)'}
            </button>
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-lg border border-zinc-300 dark:border-zinc-700 text-zinc-700 dark:text-zinc-300 text-sm font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors"
            >
              Later
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default BalanceCheckModal;
