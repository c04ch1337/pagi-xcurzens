import React from 'react';
import { Shield, X } from 'lucide-react';

export interface BridgeConfirmModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  /** e.g. "Jamey, I've redacted your transcript. 4 terms were sanitized. Ready to bridge to Copilot?" */
  message: string;
  /** Optional short title. */
  title?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Disable confirm (e.g. while bridging). */
  confirmDisabled?: boolean;
}

/**
 * Sovereign Check modal: final confirmation before Copilot bridge automation.
 * Use when the user triggers the bridge so they can verify redaction summary before Win+C runs.
 */
const BridgeConfirmModal: React.FC<BridgeConfirmModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
  message,
  title = 'Sovereign check',
  confirmLabel = 'Bridge to Copilot',
  cancelLabel = 'Cancel',
  confirmDisabled = false,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
      <div className="bg-white dark:bg-zinc-900 rounded-xl shadow-xl border border-zinc-200 dark:border-zinc-700 max-w-md w-full overflow-hidden">
        <div className="flex items-center justify-between p-4 border-b border-zinc-200 dark:border-zinc-800">
          <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100 flex items-center gap-2">
            <Shield size={18} className="text-amber-500" />
            {title}
          </h3>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
            aria-label="Close"
          >
            <X size={18} />
          </button>
        </div>
        <div className="p-4">
          <p className="text-sm text-zinc-700 dark:text-zinc-300 whitespace-pre-wrap">{message}</p>
        </div>
        <div className="flex gap-2 p-4 border-t border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-950">
          <button
            type="button"
            onClick={onClose}
            className="flex-1 py-2 px-3 rounded-lg border border-zinc-300 dark:border-zinc-600 text-zinc-700 dark:text-zinc-300 text-sm font-medium hover:bg-zinc-100 dark:hover:bg-zinc-800"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={confirmDisabled}
            className="flex-1 py-2 px-3 rounded-lg bg-amber-500 hover:bg-amber-600 disabled:opacity-50 text-white text-sm font-medium"
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
};

export default BridgeConfirmModal;
