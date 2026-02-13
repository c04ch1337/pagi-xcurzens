import React from 'react';

interface HeaderProps {
  lastUpdate: Date;
  logoText: string;
  logoUrl: string;
  systemName: string;
  onOpenSettings: () => void;
  hasHighIntentLeads?: boolean;
}

const Header: React.FC<HeaderProps> = ({ 
  lastUpdate, 
  logoText, 
  logoUrl, 
  systemName, 
  onOpenSettings,
  hasHighIntentLeads = false
}) => {
  // Split the system name to keep the stylized look (First word bold, rest light)
  const nameParts = systemName.split(' ');
  const firstWord = nameParts[0];
  const rest = nameParts.slice(1).join(' ');

  return (
    <header className="fixed top-0 left-0 right-0 navy-bg text-white h-16 flex items-center justify-between px-6 z-50 shadow-2xl border-b border-white/5">
      <div className="flex items-center gap-4">
        <div className="orange-bg w-10 h-10 rounded flex items-center justify-center font-black text-navy text-2xl shadow-inner shadow-black/20 overflow-hidden">
          {logoUrl ? (
            <img src={logoUrl} alt="Logo" className="w-full h-full object-cover" />
          ) : (
            logoText
          )}
        </div>
        <h1 className="text-xl md:text-2xl font-bold tracking-tight uppercase flex gap-2 overflow-hidden">
          <span className="whitespace-nowrap">{firstWord}</span>
          {rest && <span className="font-light text-gray-400 whitespace-nowrap hidden sm:inline">{rest}</span>}
        </h1>
      </div>

      <div className="hidden md:flex items-center gap-6">
        {/* Root Sovereign Identity Section with Dynamic High-Intent Alert */}
        <div className={`text-right transition-all duration-700 px-4 py-1.5 rounded-lg border ${
          hasHighIntentLeads 
            ? 'border-orange-500/50 shadow-[0_0_20px_rgba(250,146,28,0.3)] bg-orange-500/5 animate-pulse' 
            : 'border-transparent'
        }`}>
          <div className="flex items-center justify-end gap-2 mb-0.5">
            <div className={`flex items-center gap-1.5 px-2 py-0.5 border rounded text-[7px] font-black tracking-widest uppercase shadow-sm transition-colors duration-500 ${
              hasHighIntentLeads 
                ? 'bg-orange-500/20 border-orange-500/40 text-orange-400' 
                : 'bg-green-500/10 border-green-500/20 text-green-500'
            }`}>
              <span className="relative flex h-2 w-2">
                <span className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${hasHighIntentLeads ? 'bg-orange-400' : 'bg-green-400'}`}></span>
                <span className={`relative inline-flex rounded-full h-2 w-2 ${hasHighIntentLeads ? 'bg-orange-500' : 'bg-green-500'}`}></span>
              </span>
              {hasHighIntentLeads ? 'PRIORITY' : 'SECURE'}
            </div>
            <p className="text-[10px] text-gray-400 uppercase tracking-[0.2em] font-black">Root Sovereign</p>
          </div>
          <p className={`text-sm font-black tracking-tighter uppercase transition-all duration-500 ${
            hasHighIntentLeads 
              ? 'text-orange-400 drop-shadow-[0_0_12px_rgba(250,146,28,0.8)]' 
              : 'text-white'
          }`}>THE CREATOR</p>
        </div>
        
        <div className="h-8 w-px bg-white/10 mx-2"></div>
        
        <div className="text-right">
          <p className="text-[10px] text-gray-400 uppercase tracking-widest font-black mb-0.5">Live Monitoring</p>
          <p className="text-sm font-mono text-orange-400 font-bold">
            {lastUpdate.toLocaleTimeString([], { hour12: false })}
          </p>
        </div>
        
        {/* Settings Button */}
        <button 
          onClick={onOpenSettings}
          className="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-white/60 hover:text-white transition-all border border-white/10 group"
          title="UI Settings"
        >
          <svg className="w-5 h-5 group-hover:rotate-90 transition-transform duration-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>
      
      {/* Small Screen Root indicator */}
      <div className="md:hidden flex items-center gap-3">
        <button onClick={onOpenSettings} className="p-1.5 text-white/50 border border-white/10 rounded">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
        <div className={`px-3 py-1 rounded text-[10px] font-black border uppercase tracking-tighter flex items-center gap-1.5 transition-all duration-500 ${
          hasHighIntentLeads 
            ? 'bg-orange-500/20 border-orange-500 text-orange-400 shadow-[0_0_20px_rgba(250,146,28,0.3)]' 
            : 'bg-white/10 border-white/20 text-white'
        }`}>
          <span className={`w-1.5 h-1.5 rounded-full animate-pulse ${hasHighIntentLeads ? 'bg-orange-500 shadow-[0_0_5px_#FA921C]' : 'bg-green-500'}`}></span>
          CREATOR
        </div>
      </div>
    </header>
  );
};

export default Header;