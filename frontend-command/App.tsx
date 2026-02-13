
import React, { useState, useEffect, useCallback } from 'react';
import Header from './components/Header';
import SummaryCards from './components/SummaryCards';
import LeadLedger from './components/LeadLedger';
import Footer from './components/Footer';
import AIInsightBar from './components/AIInsightBar';
import LeadDetailModal from './components/LeadDetailModal';
import SettingsModal from './components/SettingsModal';
import { fetchInfrastructureLeads } from './services/leadService';
import { getAIInsight } from './services/geminiService';
import { InfrastructureData, Lead } from './types';

const App: React.FC = () => {
  const [data, setData] = useState<InfrastructureData | null>(null);
  const [insight, setInsight] = useState<string>("Analyzing infrastructure metrics...");
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  
  // UI Settings State
  const [settings, setSettings] = useState({
    systemName: 'XCURZENS Command Center',
    logoText: 'X',
    logoUrl: '',
    customCSS: '/* Add your sovereign CSS overrides here */\n.navy-bg {\n  /* Example: change background to a gradient */\n  /* background: linear-gradient(135deg, #051C55 0%, #0a2e8a 100%); */\n}',
  });

  // Modal States
  const [selectedLead, setSelectedLead] = useState<Lead | null>(null);
  const [isDetailModalOpen, setIsDetailModalOpen] = useState(false);
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);

  const refreshData = useCallback(async () => {
    setIsRefreshing(true);
    try {
      const newData = await fetchInfrastructureLeads();
      setData(newData);
      setLastUpdate(new Date());
      setIsLoading(false);
    } catch (error) {
      console.error("Polling error:", error);
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  const refreshInsight = useCallback(async (currentData: InfrastructureData) => {
    const newInsight = await getAIInsight(currentData);
    setInsight(newInsight);
  }, []);

  useEffect(() => {
    refreshData();
    const interval = setInterval(refreshData, 5000);
    return () => clearInterval(interval);
  }, [refreshData]);

  useEffect(() => {
    if (data && insight === "Analyzing infrastructure metrics...") {
      refreshInsight(data);
    }
  }, [data, insight, refreshInsight]);

  const handleLeadClick = (lead: Lead) => {
    setSelectedLead(lead);
    setIsDetailModalOpen(true);
  };

  const handleCloseDetailModal = () => {
    setIsDetailModalOpen(false);
    setTimeout(() => setSelectedLead(null), 300);
  };

  const saveSettings = (newSettings: typeof settings) => {
    setSettings(newSettings);
    // Optional: Persist to localStorage
    localStorage.setItem('xcurzens_settings', JSON.stringify(newSettings));
  };

  // Load settings on mount
  useEffect(() => {
    const saved = localStorage.getItem('xcurzens_settings');
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        // Migration: Ensure new fields exist
        if (!parsed.systemName) parsed.systemName = 'XCURZENS Command Center';
        setSettings(parsed);
      } catch (e) {
        console.error("Failed to load settings", e);
      }
    }
  }, []);

  if (!data && isLoading) {
    return (
      <div className="h-screen w-screen navy-bg flex items-center justify-center text-white">
        <div className="text-center">
          <div className="w-16 h-16 border-4 border-orange-500 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
          <p className="text-xl font-bold tracking-widest">INITIALIZING {settings.systemName.toUpperCase()}</p>
          <p className="text-sm opacity-50 mt-2 text-gray-400">Handshaking with Sovereign Root IP...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-gray-100">
      {/* Inject Custom CSS */}
      <style>{settings.customCSS}</style>

      <Header 
        lastUpdate={lastUpdate} 
        logoText={settings.logoText} 
        logoUrl={settings.logoUrl}
        systemName={settings.systemName}
        onOpenSettings={() => setIsSettingsModalOpen(true)}
        hasHighIntentLeads={!!data && data.system_summary.high_intent_leads > 0}
      />
      
      <main className="flex-1 overflow-hidden p-4 md:p-6 space-y-6 flex flex-col pt-20 pb-16">
        <AIInsightBar insight={insight} />
        
        {data && (
          <>
            <section>
              <SummaryCards summary={data.system_summary} />
            </section>
            
            <section className="flex-1 min-h-0 flex flex-col">
              <div className="bg-white rounded-lg shadow-xl overflow-hidden flex flex-col flex-1 border border-gray-200">
                <div className="navy-bg p-4 flex justify-between items-center">
                  <h2 className="text-white font-bold text-lg flex items-center gap-2">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                    LEAD LEDGER
                  </h2>
                  <div className="text-xs text-gray-300 uppercase tracking-tighter">
                    Active Feed | {data.leads.length} Records Shown
                  </div>
                </div>
                <LeadLedger 
                  leads={data.leads} 
                  onLeadClick={handleLeadClick} 
                  onRefresh={refreshData}
                  isRefreshing={isRefreshing}
                />
              </div>
            </section>
          </>
        )}
      </main>

      <Footer />

      {/* Detail Modal */}
      {selectedLead && (
        <LeadDetailModal 
          lead={selectedLead} 
          isOpen={isDetailModalOpen} 
          onClose={handleCloseDetailModal} 
        />
      )}

      {/* Settings Modal */}
      <SettingsModal
        isOpen={isSettingsModalOpen}
        onClose={() => setIsSettingsModalOpen(false)}
        settings={settings}
        onSave={saveSettings}
      />
    </div>
  );
};

export default App;
