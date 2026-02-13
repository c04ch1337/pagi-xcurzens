import React from 'react';
import { Lead } from '../types';
import { ArrowLeftIcon, ActivityIcon, ShieldCheckIcon, GlobeIcon, CodeIcon, ServerIcon } from './Icons';

interface LeadDetailsProps {
  lead: Lead;
  onBack: () => void;
}

export const LeadDetails: React.FC<LeadDetailsProps> = ({ lead, onBack }) => {
  // Mock JSON data for the payload view
  const mockPayload = {
    "signal_id": lead.id,
    "source_vector": lead.source.toLowerCase().replace(' ', '_'),
    "sector_classification": lead.sector,
    "timestamp_utc": new Date().toISOString(),
    "geo_coordinates": {
      "lat": 27.800583,
      "lng": -97.39638,
      "region": "Gulf Coast Zone 4"
    },
    "bandwidth_metrics": {
      "latency_ms": Math.floor(Math.random() * 50) + 10,
      "packet_loss": "0.01%",
      "encryption": "AES-256-GCM"
    },
    "infrastructure_node": {
      "verified": true,
      "node_hash": "x8821-alpha-secure"
    }
  };

  return (
    <div className="w-full max-w-6xl mx-auto space-y-6 animate-fade-in-up">
      
      {/* Navigation Header */}
      <div className="flex items-center gap-4 mb-2">
        <button 
          onClick={onBack}
          className="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 hover:text-white transition-colors border border-white/10"
        >
          <ArrowLeftIcon className="w-5 h-5" />
        </button>
        <div>
          <h2 className="text-xl font-bold text-white flex items-center gap-2">
            Signal Intercept: <span className="text-nexus-orange font-mono">{lead.id}</span>
          </h2>
          <p className="text-xs text-gray-400">Deep dive analysis of incoming infrastructure signal.</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        
        {/* Left Column: Intelligence Summary */}
        <div className="space-y-6">
          
          {/* Status Card */}
          <div className="bg-nexus-glass backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-6 shadow-xl relative overflow-hidden group h-full">
            <div className="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
              <ShieldCheckIcon className="w-24 h-24 text-nexus-orange" />
            </div>
            <h3 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4 flex items-center gap-2">
              <ActivityIcon className="w-4 h-4 text-nexus-orange" />
              Intelligence Summary
            </h3>
            
            <div className="space-y-4">
              <div className="flex justify-between items-center pb-2 border-b border-white/5">
                <span className="text-sm text-gray-400">Current Status</span>
                <span className={`px-2 py-1 rounded text-xs font-bold uppercase tracking-wide
                  ${lead.status === 'new' ? 'bg-nexus-orange/20 text-nexus-orange' : 
                    lead.status === 'processing' ? 'bg-blue-500/20 text-blue-400' : 
                    'bg-green-500/20 text-green-400'}`}>
                  {lead.status}
                </span>
              </div>
              <div className="flex justify-between items-center pb-2 border-b border-white/5">
                <span className="text-sm text-gray-400">Signal Confidence</span>
                <span className="text-sm text-white font-mono">98.4%</span>
              </div>
              <div className="flex justify-between items-center pb-2 border-b border-white/5">
                <span className="text-sm text-gray-400">Origin Source</span>
                <span className="text-sm text-white">{lead.source}</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-400">Target Sector</span>
                <span className="text-sm text-white">{lead.sector}</span>
              </div>
            </div>
          </div>

        </div>

        {/* Right Column: Technical & Audit */}
        <div className="space-y-6">
          
          {/* Geo/Tech Details */}
          <div className="bg-nexus-glass backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-6 shadow-xl">
             <h3 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4 flex items-center gap-2">
              <GlobeIcon className="w-4 h-4 text-blue-400" />
              Technical Origins
            </h3>
            <div className="grid grid-cols-2 gap-4">
              <div className="p-3 bg-nexus-navy/40 rounded-lg border border-white/5">
                <p className="text-[10px] text-gray-500 uppercase">IP Address</p>
                <p className="text-xs font-mono text-white mt-1">192.168.44.12</p>
              </div>
              <div className="p-3 bg-nexus-navy/40 rounded-lg border border-white/5">
                <p className="text-[10px] text-gray-500 uppercase">Latency</p>
                <p className="text-xs font-mono text-green-400 mt-1">24ms</p>
              </div>
              <div className="p-3 bg-nexus-navy/40 rounded-lg border border-white/5">
                <p className="text-[10px] text-gray-500 uppercase">Protocol</p>
                <p className="text-xs font-mono text-white mt-1">HTTPS / TLS 1.3</p>
              </div>
              <div className="p-3 bg-nexus-navy/40 rounded-lg border border-white/5">
                <p className="text-[10px] text-gray-500 uppercase">Region</p>
                <p className="text-xs font-mono text-white mt-1">US-SOUTH-1</p>
              </div>
            </div>
          </div>

          {/* Audit Timeline */}
          <div className="bg-nexus-glass backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-6 shadow-xl">
            <h3 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4 flex items-center gap-2">
                <ServerIcon className="w-4 h-4 text-purple-400" />
                Processing Audit
              </h3>
            <div className="space-y-4 relative pl-4 border-l border-white/10">
               <div className="relative">
                 <div className="absolute -left-[21px] top-1 w-3 h-3 bg-green-500 rounded-full border-2 border-nexus-navy"></div>
                 <p className="text-xs text-gray-400 mb-0.5">{lead.timestamp}</p>
                 <p className="text-sm text-white">Signal intercepted and verified.</p>
               </div>
               <div className="relative">
                 <div className="absolute -left-[21px] top-1 w-3 h-3 bg-blue-500 rounded-full border-2 border-nexus-navy"></div>
                 <p className="text-xs text-gray-400 mb-0.5">T - 0.05s</p>
                 <p className="text-sm text-white">Handshake initiated with partner node.</p>
               </div>
               <div className="relative">
                 <div className="absolute -left-[21px] top-1 w-3 h-3 bg-gray-600 rounded-full border-2 border-nexus-navy"></div>
                 <p className="text-xs text-gray-400 mb-0.5">T - 0.12s</p>
                 <p className="text-sm text-white">Inbound connection request received.</p>
               </div>
            </div>
          </div>

        </div>

      </div>

      {/* Full Width Payload Viewer */}
      <div className="bg-black/40 backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-6 shadow-xl flex flex-col h-[300px]">
        <div className="flex justify-between items-center mb-4">
            <h3 className="text-sm font-semibold text-gray-300 uppercase tracking-wider flex items-center gap-2">
            <CodeIcon className="w-4 h-4 text-yellow-400" />
            Raw Signal Payload
            </h3>
            <span className="text-[10px] font-mono text-gray-500">JSON</span>
        </div>
        <div className="flex-grow overflow-auto custom-scrollbar bg-nexus-navy/50 rounded-lg p-4 border border-white/5 font-mono text-xs text-blue-300">
            <pre>{JSON.stringify(mockPayload, null, 2)}</pre>
        </div>
      </div>
    </div>
  );
};