import React, { useState } from 'react';
import { NexusForm } from './components/NexusForm';
import { Dashboard } from './components/Dashboard';
import { AnchorIcon, SignalIcon, SettingsIcon, LayoutIcon } from './components/Icons';
import { SettingsModal } from './components/SettingsModal';

type ViewState = 'onboarding' | 'dashboard';

function App() {
  const [currentView, setCurrentView] = useState<ViewState>('onboarding');
  const [showSettings, setShowSettings] = useState(false);
  const [branding, setBranding] = useState({
    appName: 'XCURZENS',
    appSubtitle: 'Nexus Terminal',
    logoUrl: '',
    customCss: ''
  });

  return (
    <div className="min-h-screen w-full bg-nexus-navy bg-mesh-gradient text-white flex flex-col relative overflow-x-hidden">
      
      {/* Inject Custom CSS */}
      {branding.customCss && (
        <style dangerouslySetInnerHTML={{ __html: branding.customCss }} />
      )}

      {/* Background Decor */}
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden pointer-events-none z-0">
         {/* Subtle grid mesh overlay */}
         <div className="absolute inset-0 bg-[url('https://grainy-gradients.vercel.app/noise.svg')] opacity-5 mix-blend-overlay"></div>
         <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-nexus-orange/5 blur-[100px] rounded-full"></div>
         <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-blue-600/10 blur-[100px] rounded-full"></div>
      </div>

      {/* Header */}
      <header className="relative z-10 w-full px-4 sm:px-8 py-6 flex justify-between items-center border-b border-white/5 backdrop-blur-sm">
        <div className="flex items-center gap-3">
          {branding.logoUrl ? (
             <img src={branding.logoUrl} alt="Logo" className="h-10 w-auto rounded-lg object-contain" />
          ) : (
            <div className="w-10 h-10 bg-white/10 rounded-lg flex items-center justify-center border border-white/10">
               <AnchorIcon className="text-nexus-orange w-6 h-6" />
            </div>
          )}
          <div className="flex flex-col">
            <h1 className="text-lg font-bold tracking-wider leading-none">{branding.appName}</h1>
            <span className="text-[10px] text-gray-400 tracking-[0.2em] uppercase mt-1">{branding.appSubtitle}</span>
          </div>
        </div>

        {/* Nav Tabs (Desktop) */}
        <div className="hidden md:flex bg-white/5 p-1 rounded-lg border border-white/5">
          <button 
            onClick={() => setCurrentView('onboarding')}
            className={`px-4 py-1.5 rounded-md text-xs font-medium transition-all ${currentView === 'onboarding' ? 'bg-nexus-orange text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}
          >
            Registration
          </button>
          <button 
             onClick={() => setCurrentView('dashboard')}
             className={`px-4 py-1.5 rounded-md text-xs font-medium transition-all ${currentView === 'dashboard' ? 'bg-nexus-orange text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}
          >
            Dashboard
          </button>
        </div>
        
        <div className="flex items-center gap-4 text-right">
          <div className="hidden sm:block">
            <p className="text-xs text-gray-400 font-light">Root Sovereign</p>
            <p className="text-sm font-medium text-nexus-orange">The Creator</p>
          </div>
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse shadow-[0_0_10px_rgba(34,197,94,0.5)]"></div>
          
          <button 
            onClick={() => setShowSettings(true)}
            className="ml-2 p-2 rounded-full hover:bg-white/5 text-gray-400 hover:text-white transition-colors"
            title="UI Settings"
          >
            <SettingsIcon className="w-5 h-5" />
          </button>
        </div>
      </header>
      
      {/* Mobile Nav */}
      <div className="md:hidden flex justify-center py-2 border-b border-white/5 relative z-10 bg-nexus-navy/30 backdrop-blur-md">
        <div className="flex bg-white/5 p-1 rounded-lg border border-white/5">
            <button 
              onClick={() => setCurrentView('onboarding')}
              className={`px-4 py-1.5 rounded-md text-xs font-medium transition-all ${currentView === 'onboarding' ? 'bg-nexus-orange text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}
            >
              Registration
            </button>
            <button 
              onClick={() => setCurrentView('dashboard')}
              className={`px-4 py-1.5 rounded-md text-xs font-medium transition-all ${currentView === 'dashboard' ? 'bg-nexus-orange text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}
            >
              Dashboard
            </button>
          </div>
      </div>

      {/* Main Content */}
      <main className="relative z-10 flex-grow flex flex-col justify-center items-center px-4 py-8 sm:py-12 w-full">
        {currentView === 'onboarding' ? (
          <>
            <NexusForm onGoToDashboard={() => setCurrentView('dashboard')} />
            {/* Technical Mock Note for User Clarity */}
            <div className="mt-8 text-center opacity-40 text-xs font-mono max-w-lg">
              <p>Local Simulation: Submit with "error" in URL to test failure state.</p>
            </div>
          </>
        ) : (
          <Dashboard />
        )}
      </main>

      {/* Footer */}
      <footer className="relative z-10 w-full py-6 text-center border-t border-white/5 backdrop-blur-sm bg-nexus-navy/30">
        <div className="flex flex-col sm:flex-row justify-center items-center gap-2 text-xs text-gray-500 tracking-wide uppercase">
          <SignalIcon className="w-4 h-4 text-nexus-orange/70" />
          <span>Secure Bare Metal Infrastructure</span>
          <span className="hidden sm:inline mx-2 text-gray-700">|</span>
          <span>No Containers</span>
        </div>
      </footer>

      {/* Settings Modal */}
      <SettingsModal 
        isOpen={showSettings}
        onClose={() => setShowSettings(false)}
        branding={branding}
        onSave={setBranding}
      />

    </div>
  );
}

export default App;