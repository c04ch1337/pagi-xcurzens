
import React, { useState, useEffect } from 'react';

interface Settings {
  systemName: string;
  logoText: string;
  logoUrl: string;
  customCSS: string;
}

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  settings: Settings;
  onSave: (settings: Settings) => void;
}

const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose, settings, onSave }) => {
  const [localSettings, setLocalSettings] = useState<Settings>(settings);

  useEffect(() => {
    if (isOpen) setLocalSettings(settings);
  }, [isOpen, settings]);

  if (!isOpen) return null;

  const handleSave = () => {
    onSave(localSettings);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-navy/80 backdrop-blur-md" onClick={onClose} />
      
      <div className="bg-white w-full max-w-xl rounded-xl shadow-2xl overflow-hidden relative z-10 animate-in fade-in slide-in-from-bottom-4 duration-300">
        <div className="navy-bg px-6 py-4 flex items-center justify-between border-b border-white/10">
          <div className="flex items-center gap-3">
            <div className="bg-white/10 p-2 rounded">
              <svg className="w-5 h-5 text-orange-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
              </svg>
            </div>
            <h2 className="text-white font-bold text-lg uppercase tracking-widest">UI Override Module</h2>
          </div>
          <button onClick={onClose} className="text-white/50 hover:text-white">
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="p-6 space-y-6 max-h-[70vh] overflow-y-auto custom-scrollbar">
          {/* Branding Section */}
          <section className="space-y-4">
            <h3 className="text-[10px] font-black text-gray-400 uppercase tracking-[0.2em] border-b border-gray-100 pb-1">Branding Configuration</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-[10px] font-bold text-navy uppercase mb-1">System Name</label>
                <input 
                  type="text" 
                  value={localSettings.systemName}
                  onChange={(e) => setLocalSettings({...localSettings, systemName: e.target.value})}
                  className="w-full bg-gray-50 border border-gray-200 rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-orange-500/50"
                  placeholder="e.g. XCURZENS Command Center"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-[10px] font-bold text-navy uppercase mb-1">Logo Text</label>
                  <input 
                    type="text" 
                    value={localSettings.logoText}
                    onChange={(e) => setLocalSettings({...localSettings, logoText: e.target.value})}
                    className="w-full bg-gray-50 border border-gray-200 rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-orange-500/50"
                    maxLength={2}
                  />
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-navy uppercase mb-1">Logo Image URL</label>
                  <input 
                    type="text" 
                    placeholder="https://..."
                    value={localSettings.logoUrl}
                    onChange={(e) => setLocalSettings({...localSettings, logoUrl: e.target.value})}
                    className="w-full bg-gray-50 border border-gray-200 rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-orange-500/50"
                  />
                </div>
              </div>
            </div>
          </section>

          {/* CSS Section */}
          <section className="space-y-4">
            <div className="flex justify-between items-center">
              <h3 className="text-[10px] font-black text-gray-400 uppercase tracking-[0.2em]">Global CSS Injection</h3>
              <span className="text-[8px] font-mono text-orange-600 bg-orange-100 px-1 rounded">EXPERIMENTAL</span>
            </div>
            <div className="relative group">
              <textarea
                value={localSettings.customCSS}
                onChange={(e) => setLocalSettings({...localSettings, customCSS: e.target.value})}
                className="w-full h-48 bg-gray-900 text-emerald-400 font-mono text-xs p-4 rounded-lg focus:outline-none ring-1 ring-white/10 focus:ring-orange-500/50 resize-none shadow-inner"
                spellCheck={false}
              />
              <div className="absolute top-2 right-2 opacity-30 group-hover:opacity-100 transition-opacity">
                 <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                   <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
                 </svg>
              </div>
            </div>
            <p className="text-[9px] text-gray-400 italic">
              * Custom CSS is applied globally to the application DOM. Use with caution.
            </p>
          </section>
        </div>

        <div className="bg-gray-50 px-6 py-4 flex justify-between items-center border-t border-gray-100">
           <button 
             onClick={() => {
               const reset = { 
                 systemName: 'XCURZENS Command Center',
                 logoText: 'X', 
                 logoUrl: '', 
                 customCSS: '/* CSS Reset */' 
               };
               setLocalSettings(reset);
             }}
             className="text-[10px] font-bold text-gray-400 hover:text-red-500 uppercase tracking-widest transition-colors"
           >
             Factory Reset
           </button>
           <div className="flex gap-3">
             <button 
               onClick={onClose}
               className="px-4 py-2 text-xs font-bold text-navy hover:text-navy/70 uppercase tracking-widest"
             >
               Cancel
             </button>
             <button 
               onClick={handleSave}
               className="px-6 py-2 rounded-lg font-bold text-xs uppercase tracking-widest orange-bg text-navy hover:scale-[1.02] transition-all shadow-lg active:scale-95"
             >
               Update Interface
             </button>
           </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsModal;
