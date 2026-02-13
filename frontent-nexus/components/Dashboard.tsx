import React, { useState, useEffect } from 'react';
import { 
  ActivityIcon, 
  ServerIcon, 
  UsersIcon, 
  RefreshIcon, 
} from './Icons';
import { 
  NetworkTopologyImg, 
  SecureStackImg, 
  CoastalMapImg, 
  PartnerLinkImg, 
  DataStreamImg 
} from './DashboardAssets';
import { LeadDetails } from './LeadDetails';
import { SignalFeed } from './SignalFeed';
import { Lead, SystemLog } from '../types';

export const Dashboard: React.FC = () => {
  const [leads, setLeads] = useState<Lead[]>([
    { id: 'LD-8821', source: 'Direct API', sector: 'Charter', status: 'synced', timestamp: '10:42:01' },
    { id: 'LD-8820', source: 'Web Form', sector: 'Rental', status: 'synced', timestamp: '10:41:45' },
    { id: 'LD-8819', source: 'Partner Net', sector: 'Logistics', status: 'processing', timestamp: '10:40:12' },
  ]);

  const [logs, setLogs] = useState<SystemLog[]>([
    { id: 1, message: 'System initialized. Node active.', type: 'info', timestamp: '10:40:00' },
    { id: 2, message: 'Secure handshake established with Nexus Core.', type: 'success', timestamp: '10:40:02' },
    { id: 3, message: 'Listening for incoming lead signals...', type: 'info', timestamp: '10:40:05' },
  ]);

  const [isRefreshing, setIsRefreshing] = useState(false);
  const [selectedLead, setSelectedLead] = useState<Lead | null>(null);
  const [viewMode, setViewMode] = useState<'overview' | 'feed'>('overview');

  // Helper for consistent time formatting (24h format)
  const getTimestamp = () => new Date().toLocaleTimeString('en-US', { hour12: false });

  // Simulate Polling for Leads
  useEffect(() => {
    const interval = setInterval(() => {
      const sectors = ['Charter', 'Rental', 'Beach Box', 'Logistics'];
      const sources = ['Direct API', 'Web Form', 'Partner Net', 'Walk-in'];
      
      const newLead: Lead = {
        id: `LD-${Math.floor(Math.random() * 9000) + 1000}`,
        source: sources[Math.floor(Math.random() * sources.length)],
        sector: sectors[Math.floor(Math.random() * sectors.length)],
        status: 'new',
        timestamp: getTimestamp(),
      };

      setLeads(prev => [newLead, ...prev].slice(0, 50)); // Keep last 50 for the feed
      
      // Add log
      const logMsg = `Incoming signal: ${newLead.id} via ${newLead.source}`;
      setLogs(prev => [...prev, { 
        id: Date.now(), 
        message: logMsg, 
        type: 'info', 
        timestamp: getTimestamp()
      }].slice(-8)); // Keep last 8 logs

    }, 5000); // 5 seconds polling

    return () => clearInterval(interval);
  }, []);

  const handleRefresh = () => {
    if (isRefreshing) return;
    setIsRefreshing(true);

    // Immediate feedback log
    setLogs(prev => [...prev, { 
      id: Date.now(), 
      message: 'Manual refresh sequence initiated...', 
      type: 'info', 
      timestamp: getTimestamp()
    }].slice(-8));

    // Simulate network latency
    setTimeout(() => {
      const sectors = ['Charter', 'Rental', 'Beach Box', 'Logistics'];
      const sources = ['Manual Sync', 'Admin Audit', 'Partner Net'];
      
      const newLead: Lead = {
        id: `LD-${Math.floor(Math.random() * 9000) + 1000}`,
        source: sources[Math.floor(Math.random() * sources.length)],
        sector: sectors[Math.floor(Math.random() * sectors.length)],
        status: 'synced',
        timestamp: getTimestamp(),
      };

      setLeads(prev => [newLead, ...prev].slice(0, 50));

      setLogs(prev => [...prev, { 
        id: Date.now() + 100, 
        message: 'Index updated. Node data synchronized.', 
        type: 'success', 
        timestamp: getTimestamp()
      }].slice(-8));
      
      setIsRefreshing(false);
    }, 800);
  };

  // Navigation Handlers
  if (selectedLead) {
    return (
      <LeadDetails 
        lead={selectedLead} 
        onBack={() => setSelectedLead(null)} 
      />
    );
  }

  if (viewMode === 'feed') {
    return (
      <SignalFeed 
        leads={leads}
        onSelectLead={setSelectedLead}
        onBack={() => setViewMode('overview')}
      />
    );
  }

  return (
    <div className="w-full max-w-6xl mx-auto space-y-6 animate-fade-in pb-12">
      
      {/* Header Stats */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Card 1 */}
        <div className="bg-nexus-glass backdrop-blur-md border border-nexus-glassBorder rounded-xl p-5 flex items-center gap-4 shadow-lg hover:border-nexus-orange/30 transition-colors group">
          <div className="p-3 bg-nexus-orange/10 rounded-lg text-nexus-orange group-hover:scale-110 transition-transform">
            <ActivityIcon className="w-6 h-6" />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase tracking-wider">Network Status</p>
            <p className="text-lg font-bold text-white flex items-center gap-2">
              Online 
              <span className="flex h-2 w-2 relative">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
              </span>
            </p>
          </div>
        </div>

        {/* Card 2 */}
        <div className="bg-nexus-glass backdrop-blur-md border border-nexus-glassBorder rounded-xl p-5 flex items-center gap-4 shadow-lg hover:border-nexus-orange/30 transition-colors group">
          <div className="p-3 bg-blue-500/10 rounded-lg text-blue-400 group-hover:scale-110 transition-transform">
            <ServerIcon className="w-6 h-6" />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase tracking-wider">Active Nodes</p>
            <p className="text-lg font-bold text-white">142</p>
          </div>
        </div>

        {/* Card 3 */}
        <div className="bg-nexus-glass backdrop-blur-md border border-nexus-glassBorder rounded-xl p-5 flex items-center gap-4 shadow-lg hover:border-nexus-orange/30 transition-colors group">
          <div className="p-3 bg-purple-500/10 rounded-lg text-purple-400 group-hover:scale-110 transition-transform">
            <UsersIcon className="w-6 h-6" />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase tracking-wider">Total Leads</p>
            <p className="text-lg font-bold text-white">{1248 + leads.length}</p>
          </div>
        </div>

        {/* Card 4 */}
        <div className="bg-nexus-glass backdrop-blur-md border border-nexus-glassBorder rounded-xl p-5 flex items-center gap-4 shadow-lg hover:border-nexus-orange/30 transition-colors group">
          <div className="p-3 bg-teal-500/10 rounded-lg text-teal-400 group-hover:scale-110 transition-transform">
            <RefreshIcon className="w-6 h-6" />
          </div>
          <div>
            <p className="text-xs text-gray-400 uppercase tracking-wider">Uptime</p>
            <p className="text-lg font-bold text-white">99.9%</p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        
        {/* Main Panel: Leads Feed Preview */}
        <div className="lg:col-span-2 bg-nexus-navy/40 backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-6 shadow-2xl relative overflow-hidden">
           <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-blue-500 to-transparent opacity-30"></div>
           
           <div className="flex justify-between items-center mb-6">
             <div>
               <h3 className="text-xl font-semibold text-white">Live Signal Feed</h3>
               <p className="text-xs text-gray-400">Real-time incoming infrastructure requests</p>
             </div>
             <div className="flex items-center gap-3">
               <div className="flex items-center gap-2 text-xs text-nexus-orange border border-nexus-orange/20 px-2 py-1 rounded bg-nexus-orange/5">
                  <span className="animate-pulse">●</span> LIVE
               </div>
               <button 
                 onClick={() => setViewMode('feed')}
                 className="px-3 py-1.5 rounded-lg bg-blue-500/10 hover:bg-blue-500/20 text-blue-300 hover:text-white text-xs font-medium transition-all border border-blue-500/20 hover:border-blue-500/40"
               >
                 View Source Feed &rarr;
               </button>
             </div>
           </div>

           <div className="overflow-x-auto">
             <table className="w-full text-sm text-left">
               <thead className="text-xs text-gray-400 uppercase bg-white/5">
                 <tr>
                   <th className="px-4 py-3 rounded-l-lg">ID</th>
                   <th className="px-4 py-3">Sector</th>
                   <th className="px-4 py-3">Source</th>
                   <th className="px-4 py-3">Timestamp</th>
                   <th className="px-4 py-3 rounded-r-lg text-right">Status</th>
                 </tr>
               </thead>
               <tbody className="divide-y divide-white/5">
                 {leads.slice(0, 8).map((lead) => (
                   <tr 
                     key={lead.id} 
                     onClick={() => setSelectedLead(lead)}
                     className="hover:bg-white/5 transition-colors group cursor-pointer"
                   >
                     <td className="px-4 py-3 font-mono text-blue-300 group-hover:text-nexus-orange transition-colors underline decoration-blue-300/30 underline-offset-4 group-hover:decoration-nexus-orange/50">
                        {lead.id}
                     </td>
                     <td className="px-4 py-3 text-white">{lead.sector}</td>
                     <td className="px-4 py-3 text-gray-300">{lead.source}</td>
                     <td className="px-4 py-3 text-gray-400 font-mono text-xs">{lead.timestamp}</td>
                     <td className="px-4 py-3 text-right">
                       <span className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium border
                         ${lead.status === 'new' ? 'bg-nexus-orange/10 text-nexus-orange border-nexus-orange/20 animate-pulse' : 
                           lead.status === 'processing' ? 'bg-blue-500/10 text-blue-400 border-blue-500/20' : 
                           'bg-green-500/10 text-green-400 border-green-500/20'}`}>
                         {lead.status === 'new' && '★'} 
                         {lead.status.toUpperCase()}
                       </span>
                     </td>
                   </tr>
                 ))}
               </tbody>
             </table>
           </div>
        </div>

        {/* Side Panel: System Logs */}
        <div className="bg-black/30 backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-6 shadow-2xl flex flex-col h-[500px]">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-semibold text-white flex items-center gap-2">
                <div className="w-2 h-2 bg-gray-400 rounded-full"></div>
                System Console
            </h3>
            <button 
                onClick={handleRefresh}
                disabled={isRefreshing}
                className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium text-nexus-orange bg-nexus-orange/10 hover:bg-nexus-orange/20 border border-nexus-orange/20 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
                <RefreshIcon className={`w-3 h-3 ${isRefreshing ? 'animate-spin' : ''}`} />
                {isRefreshing ? 'Syncing...' : 'Refresh Logs'}
            </button>
          </div>
          
          <div className="flex-grow overflow-y-auto font-mono text-xs space-y-1 pr-2 custom-scrollbar p-2">
            {logs.map((log) => (
              <div 
                key={log.id} 
                className="flex gap-3 px-2 py-1.5 rounded hover:bg-white/5 transition-all duration-200 animate-fade-in-left group border-l-2 border-transparent hover:border-nexus-orange/50 items-baseline"
              >
                <span className="text-gray-500 text-[10px] w-[64px] shrink-0 opacity-60 group-hover:opacity-100 transition-opacity tabular-nums">
                    [{log.timestamp}]
                </span>
                <span className={`${
                  log.type === 'success' ? 'text-green-400' : 
                  log.type === 'warning' ? 'text-yellow-400' : 
                  'text-blue-300'
                } leading-relaxed break-words`}>
                  <span className="opacity-50 mr-2 font-bold select-none">
                    {log.type === 'success' && '✓'}
                    {log.type === 'warning' && '⚠'}
                    {log.type === 'info' && '>'}
                  </span>
                  {log.message}
                </span>
              </div>
            ))}
             <div className="h-4"></div> {/* Spacer */}
          </div>
          
          <div className="mt-4 pt-4 border-t border-white/10">
            <div className="flex items-center gap-2 text-xs text-gray-500">
              <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
              Daemon Active
            </div>
          </div>
        </div>
      </div>

      {/* Infrastructure Modules Preview */}
      <div className="mt-8">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
           <span className="w-1 h-4 bg-nexus-orange rounded-sm"></span>
           Infrastructure Modules (Coming Soon)
        </h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4">
           {/* Module 1 */}
           <div className="bg-nexus-glass border border-nexus-glassBorder rounded-xl overflow-hidden hover:border-nexus-orange/50 transition-colors group">
              <div className="h-32 w-full bg-nexus-navy relative overflow-hidden">
                <NetworkTopologyImg className="w-full h-full opacity-80 group-hover:opacity-100 transition-opacity transform group-hover:scale-105 duration-500" />
              </div>
              <div className="p-3">
                 <h4 className="text-sm font-medium text-white">Mesh Topology</h4>
                 <p className="text-[10px] text-gray-400 mt-1">Visualize active node connections.</p>
              </div>
           </div>
           
           {/* Module 2 */}
           <div className="bg-nexus-glass border border-nexus-glassBorder rounded-xl overflow-hidden hover:border-nexus-orange/50 transition-colors group">
              <div className="h-32 w-full bg-nexus-navy relative overflow-hidden">
                <SecureStackImg className="w-full h-full opacity-80 group-hover:opacity-100 transition-opacity transform group-hover:scale-105 duration-500" />
              </div>
              <div className="p-3">
                 <h4 className="text-sm font-medium text-white">Stack Security</h4>
                 <p className="text-[10px] text-gray-400 mt-1">Real-time threat monitoring.</p>
              </div>
           </div>

           {/* Module 3 */}
           <div className="bg-nexus-glass border border-nexus-glassBorder rounded-xl overflow-hidden hover:border-nexus-orange/50 transition-colors group">
              <div className="h-32 w-full bg-nexus-navy relative overflow-hidden">
                <CoastalMapImg className="w-full h-full opacity-80 group-hover:opacity-100 transition-opacity transform group-hover:scale-105 duration-500" />
              </div>
              <div className="p-3">
                 <h4 className="text-sm font-medium text-white">Geo-Logistics</h4>
                 <p className="text-[10px] text-gray-400 mt-1">Coastal asset tracking.</p>
              </div>
           </div>

           {/* Module 4 */}
           <div className="bg-nexus-glass border border-nexus-glassBorder rounded-xl overflow-hidden hover:border-nexus-orange/50 transition-colors group">
              <div className="h-32 w-full bg-nexus-navy relative overflow-hidden">
                <PartnerLinkImg className="w-full h-full opacity-80 group-hover:opacity-100 transition-opacity transform group-hover:scale-105 duration-500" />
              </div>
              <div className="p-3">
                 <h4 className="text-sm font-medium text-white">B2B Nexus</h4>
                 <p className="text-[10px] text-gray-400 mt-1">Partner relationship management.</p>
              </div>
           </div>

           {/* Module 5 */}
           <div className="bg-nexus-glass border border-nexus-glassBorder rounded-xl overflow-hidden hover:border-nexus-orange/50 transition-colors group">
              <div className="h-32 w-full bg-nexus-navy relative overflow-hidden">
                <DataStreamImg className="w-full h-full opacity-80 group-hover:opacity-100 transition-opacity transform group-hover:scale-105 duration-500" />
              </div>
              <div className="p-3">
                 <h4 className="text-sm font-medium text-white">Deep Analytics</h4>
                 <p className="text-[10px] text-gray-400 mt-1">Predictive lead flow models.</p>
              </div>
           </div>
        </div>
      </div>

      {/* Visual Telemetry Carousel */}
      <div className="mt-8">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
           <span className="w-1 h-4 bg-nexus-orange rounded-sm"></span>
           Visual Telemetry
        </h3>
        <div className="flex gap-4 overflow-x-auto pb-4 custom-scrollbar snap-x">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="min-w-[300px] h-[200px] bg-nexus-glass border border-nexus-glassBorder rounded-xl overflow-hidden relative group snap-center shrink-0">
                <img 
                  src={`data:image/svg+xml;charset=UTF-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='300' height='200' viewBox='0 0 300 200'%3E%3Crect width='300' height='200' fill='%23051C55'/%3E%3Crect width='100%25' height='100%25' fill='url(%23grid)'/%3E%3Cdefs%3E%3Cpattern id='grid' width='20' height='20' patternUnits='userSpaceOnUse'%3E%3Cpath d='M 20 0 L 0 0 0 20' fill='none' stroke='%23FA921C' stroke-width='0.5' opacity='0.2'/%3E%3C/pattern%3E%3C/defs%3E%3Ctext x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' font-family='sans-serif' font-size='14' fill='%23FA921C' font-weight='bold' letter-spacing='2'%3EFEED SIGNAL 0${i}%3C/text%3E%3C/svg%3E`}
                  alt={`Live infrastructure feed camera ${i} - Active Monitoring`}
                  className="w-full h-full object-cover opacity-80 group-hover:opacity-100 transition-opacity"
                />
                <div className="absolute top-2 left-2 flex items-center gap-2 z-10">
                  <span className="w-2 h-2 bg-red-500 rounded-full animate-pulse shadow-[0_0_8px_rgba(239,68,68,0.8)]"></span>
                  <span className="text-[10px] font-mono text-red-400 bg-black/50 px-1.5 py-0.5 rounded backdrop-blur-sm border border-red-500/20">REC</span>
                </div>
                <div className="absolute bottom-0 left-0 w-full p-3 bg-gradient-to-t from-nexus-navy to-transparent">
                   <p className="text-xs font-mono text-nexus-orange flex justify-between items-center">
                      <span>CAM-0{i} // SECTOR-{String.fromCharCode(64+i)}</span>
                      <span className="opacity-70">1080p</span>
                   </p>
                </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};