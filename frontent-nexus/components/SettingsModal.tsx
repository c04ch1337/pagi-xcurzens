import React, { useState, useEffect, useRef } from 'react';
import { SettingsIcon } from './Icons';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  branding: {
    appName: string;
    appSubtitle: string;
    logoUrl: string;
    customCss: string;
  };
  onSave: (branding: { appName: string; appSubtitle: string; logoUrl: string; customCss: string }) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose, branding, onSave }) => {
  const [localBranding, setLocalBranding] = useState(branding);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Sync local state when modal opens
  useEffect(() => {
    if (isOpen) {
      setLocalBranding(branding);
    }
  }, [isOpen, branding]);

  if (!isOpen) return null;

  const handleSave = () => {
    onSave(localBranding);
    onClose();
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onloadend = () => {
        if (reader.result) {
          setLocalBranding(prev => ({ ...prev, logoUrl: reader.result as string }));
        }
      };
      reader.readAsDataURL(file);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fade-in">
      <div className="bg-[#0A2569] border border-white/10 rounded-xl p-6 shadow-2xl max-w-lg w-full relative">
        
        <button 
          onClick={onClose}
          className="absolute top-4 right-4 text-gray-400 hover:text-white"
        >
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" /></svg>
        </button>

        <h3 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
          <SettingsIcon className="w-5 h-5 text-nexus-orange" />
          UI Customization
        </h3>

        <div className="space-y-6">
          
          {/* App Name Settings */}
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <label htmlFor="appName" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
                App Name
              </label>
              <input
                type="text"
                id="appName"
                value={localBranding.appName}
                onChange={(e) => setLocalBranding(prev => ({ ...prev, appName: e.target.value }))}
                placeholder="XCURZENS"
                className="w-full bg-nexus-navy/50 border border-white/20 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-nexus-orange focus:ring-1 focus:ring-nexus-orange transition-all text-sm"
              />
            </div>
            <div className="space-y-2">
              <label htmlFor="appSubtitle" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
                Subtitle
              </label>
              <input
                type="text"
                id="appSubtitle"
                value={localBranding.appSubtitle}
                onChange={(e) => setLocalBranding(prev => ({ ...prev, appSubtitle: e.target.value }))}
                placeholder="Nexus Terminal"
                className="w-full bg-nexus-navy/50 border border-white/20 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-nexus-orange focus:ring-1 focus:ring-nexus-orange transition-all text-sm"
              />
            </div>
          </div>

          {/* Logo URL Input */}
          <div className="space-y-2">
            <label htmlFor="logoUrl" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
              Custom Logo
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                id="logoUrl"
                value={localBranding.logoUrl}
                onChange={(e) => setLocalBranding(prev => ({ ...prev, logoUrl: e.target.value }))}
                placeholder="https://example.com/logo.png"
                className="flex-1 bg-nexus-navy/50 border border-white/20 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-nexus-orange focus:ring-1 focus:ring-nexus-orange transition-all text-sm"
              />
              <input 
                type="file" 
                ref={fileInputRef}
                onChange={handleFileUpload}
                accept="image/*"
                className="hidden"
              />
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                className="px-4 py-2 bg-white/5 hover:bg-white/10 border border-white/20 rounded-lg text-white text-xs font-medium transition-colors whitespace-nowrap"
              >
                Upload
              </button>
            </div>
            
            <p className="text-[10px] text-gray-500 flex justify-between items-center">
              <span>Replaces the default Anchor icon in the header.</span>
              {localBranding.logoUrl && <span className="text-nexus-orange">Preview Active</span>}
            </p>

            {localBranding.logoUrl && (
              <div className="mt-3 p-4 bg-white/5 rounded-lg border border-white/10 flex justify-center items-center">
                 <img src={localBranding.logoUrl} alt="Logo Preview" className="h-12 object-contain" />
              </div>
            )}
          </div>

          {/* Custom CSS Textarea */}
          <div className="space-y-2">
            <label htmlFor="customCss" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
              Custom CSS
            </label>
            <textarea
              id="customCss"
              value={localBranding.customCss}
              onChange={(e) => setLocalBranding(prev => ({ ...prev, customCss: e.target.value }))}
              placeholder=".bg-nexus-navy { background-color: #000; }"
              rows={6}
              className="w-full bg-nexus-navy/50 border border-white/20 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-nexus-orange focus:ring-1 focus:ring-nexus-orange transition-all font-mono text-xs"
            />
             <p className="text-[10px] text-gray-500">
              Injects raw CSS into the page. Use carefully.
            </p>
          </div>

          {/* Actions */}
          <div className="flex gap-3 justify-end pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-gray-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleSave}
              className="px-5 py-2 text-sm font-medium bg-nexus-orange text-white rounded-lg hover:bg-orange-500 transition-colors shadow-lg shadow-nexus-orange/20"
            >
              Save Changes
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};