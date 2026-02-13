
import React, { useMemo, useState } from 'react';
import { Lead } from '../types';

interface LeadLedgerProps {
  leads: Lead[];
  onLeadClick?: (lead: Lead) => void;
  onRefresh?: () => void;
  isRefreshing?: boolean;
}

const LeadLedger: React.FC<LeadLedgerProps> = ({ leads, onLeadClick, onRefresh, isRefreshing }) => {
  const [searchTerm, setSearchTerm] = useState('');

  // 1. Filter leads based on search term
  const filteredLeads = useMemo(() => {
    if (!searchTerm.trim()) return leads;
    const term = searchTerm.toLowerCase();
    return leads.filter(lead => 
      lead.query_snippet.toLowerCase().includes(term) ||
      lead.city.toLowerCase().includes(term) ||
      lead.id.toLowerCase().includes(term)
    );
  }, [leads, searchTerm]);

  // 2. Calculate distribution metrics for the header visualization
  const stats = useMemo(() => {
    const total = filteredLeads.length;
    if (total === 0) return { 
      high: 0, medium: 0, low: 0, 
      counts: { high: 0, medium: 0, low: 0 } 
    };

    const highCount = filteredLeads.filter(l => l.intent.toLowerCase() === 'high').length;
    const mediumCount = filteredLeads.filter(l => l.intent.toLowerCase() === 'medium').length;
    const lowCount = filteredLeads.filter(l => l.intent.toLowerCase() === 'low').length;

    return {
      high: (highCount / total) * 100,
      medium: (mediumCount / total) * 100,
      low: (lowCount / total) * 100,
      counts: { high: highCount, medium: mediumCount, low: lowCount }
    };
  }, [filteredLeads]);

  return (
    <div className="flex flex-col flex-1 min-h-0 select-none">
      {/* Search & Action Header Area */}
      <div className="px-6 py-4 bg-gray-50/50 border-b border-gray-100 flex items-center justify-between gap-4">
        <div className="flex items-center gap-4 flex-1">
          <div className="relative flex-1 max-w-md group">
            <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <svg className="h-4 w-4 text-gray-400 group-focus-within:text-orange-500 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
            </div>
            <input
              type="text"
              placeholder="Search lead payloads, nodes, or IDs..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="block w-full pl-10 pr-3 py-2 border border-gray-200 rounded-lg leading-5 bg-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500 text-sm font-medium transition-all"
            />
            {searchTerm && (
              <button 
                onClick={() => setSearchTerm('')}
                className="absolute inset-y-0 right-0 pr-3 flex items-center text-gray-400 hover:text-navy transition-colors"
                aria-label="Clear search"
              >
                <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            )}
          </div>

          <button
            onClick={onRefresh}
            disabled={isRefreshing}
            className={`
              flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-black uppercase tracking-widest transition-all
              ${isRefreshing 
                ? 'bg-gray-200 text-gray-400 cursor-not-allowed' 
                : 'navy-bg text-white hover:orange-bg active:scale-95 shadow-md shadow-navy/20'}
            `}
          >
            <svg 
              className={`w-3.5 h-3.5 ${isRefreshing ? 'animate-spin' : 'group-hover:rotate-180 transition-transform duration-500'}`} 
              fill="none" 
              stroke="currentColor" 
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            {isRefreshing ? 'Syncing...' : 'Refresh Data'}
          </button>
        </div>
        
        <div className="text-[10px] font-black text-gray-400 uppercase tracking-widest hidden sm:block">
          {filteredLeads.length} signals matching filter
        </div>
      </div>

      <div className="overflow-auto flex-1 custom-scrollbar bg-white">
        <table className="w-full text-left border-collapse min-w-[800px]">
          <thead className="sticky top-0 z-10 bg-gray-50 text-navy uppercase text-[10px] font-black tracking-widest border-b border-gray-200">
            <tr>
              <th className="px-6 py-4">ID</th>
              <th className="px-6 py-4">Timestamp</th>
              <th className="px-6 py-4">Query Snippet</th>
              <th className="px-6 py-4">Location</th>
              <th className="px-6 py-4">Weather</th>
              <th className="px-6 py-4 min-w-[240px]">
                <div className="flex flex-col gap-2">
                  <div className="flex justify-between items-center group/title cursor-help">
                    <span className="group-hover/title:text-orange-500 transition-colors tracking-tight">Intent Flux</span>
                    <div className="flex gap-2 opacity-60 group-hover/title:opacity-100 transition-opacity text-[7px] font-bold uppercase whitespace-nowrap">
                      <span className="flex items-center gap-1"><div className="w-1.5 h-1.5 rounded-full navy-bg"></div> {Math.round(stats.high)}%</span>
                      <span className="flex items-center gap-1"><div className="w-1.5 h-1.5 rounded-full orange-bg"></div> {Math.round(stats.medium)}%</span>
                      <span className="flex items-center gap-1"><div className="w-1.5 h-1.5 rounded-full bg-gray-300"></div> {Math.round(stats.low)}%</span>
                    </div>
                  </div>
                  
                  {/* Distribution Bar Chart - High=Navy, Medium=Orange, Low=Gray */}
                  <div className="flex h-2 w-full rounded-full overflow-hidden bg-gray-200 shadow-inner relative group/chart border border-gray-100">
                    <div 
                      className="navy-bg h-full transition-all duration-700 ease-out border-r border-white/10" 
                      style={{ width: `${stats.high}%` }}
                      title={`High Intent: ${stats.counts.high} signals`}
                    />
                    <div 
                      className="orange-bg h-full transition-all duration-700 ease-out border-r border-white/10" 
                      style={{ width: `${stats.medium}%` }}
                      title={`Medium Intent: ${stats.counts.medium} signals`}
                    />
                    <div 
                      className="bg-gray-300 h-full transition-all duration-700 ease-out" 
                      style={{ width: `${stats.low}%` }}
                      title={`Low Intent: ${stats.counts.low} signals`}
                    />
                  </div>
                </div>
              </th>
              <th className="px-6 py-4">Partner ID</th>
            </tr>
          </thead>
          <tbody className="text-sm font-medium divide-y divide-gray-100">
            {filteredLeads.map((lead) => (
              <tr 
                key={lead.id} 
                onClick={() => onLeadClick?.(lead)}
                className={`
                  transition-all duration-150 cursor-pointer group
                  hover:bg-orange-500/5 active:bg-orange-500/10 active:scale-[0.998]
                  ${lead.highlight ? 'bg-orange-50/30 border-l-4 orange-border' : 'border-l-4 border-transparent'}
                `}
              >
                <td className={`px-6 py-4 font-mono text-xs ${lead.highlight ? 'orange-text font-bold' : 'text-gray-500'}`}>
                  {lead.id}
                </td>
                <td className="px-6 py-4 text-gray-400 text-xs font-medium">
                  {lead.timestamp.split(' ')[1]}
                  <span className="block text-[10px] opacity-50">{lead.timestamp.split(' ')[0]}</span>
                </td>
                <td className="px-6 py-4 relative group/snippet max-w-xs">
                  <p className="truncate text-gray-700 font-semibold group-hover:text-navy transition-colors">
                    {lead.query_snippet}
                  </p>
                  {/* Tooltip for Payload */}
                  <div className="absolute bottom-full mb-2 left-6 hidden group-hover/snippet:block z-30 w-72 p-4 navy-bg text-white text-xs rounded-xl shadow-2xl border border-white/10 pointer-events-none animate-in fade-in slide-in-from-bottom-2 duration-200">
                    <div className="flex items-center gap-2 mb-2 border-b border-white/10 pb-1.5">
                      <div className="w-1.5 h-1.5 rounded-full orange-bg"></div>
                      <span className="text-[9px] font-black uppercase tracking-widest text-orange-500">Payload Capture</span>
                    </div>
                    <p className="leading-relaxed italic opacity-90">
                      "{lead.query_snippet}"
                    </p>
                    <div className="absolute -bottom-1.5 left-4 w-3 h-3 navy-bg rotate-45 border-b border-r border-white/10"></div>
                  </div>
                </td>
                <td className="px-6 py-4 text-gray-600 font-medium">
                  {lead.city}
                </td>
                <td className="px-6 py-4 text-gray-400 italic text-[11px] font-semibold">
                  {lead.weather}
                </td>
                <td className="px-6 py-4">
                  <div className="flex items-center gap-2">
                    <span className={`
                      px-2.5 py-1 rounded text-[10px] font-black uppercase inline-block min-w-[75px] text-center shadow-sm text-white tracking-wider
                      ${lead.intent.toLowerCase() === 'high' ? 'navy-bg ring-1 ring-navy/50' : 
                        lead.intent.toLowerCase() === 'medium' ? 'orange-bg ring-1 ring-orange-500/50' : 'bg-gray-100 !text-gray-500 ring-1 ring-gray-200'}
                    `}>
                      {lead.intent}
                    </span>
                    {lead.intent.toLowerCase() === 'high' && (
                      <span className="w-1.5 h-1.5 rounded-full navy-bg animate-ping"></span>
                    )}
                  </div>
                </td>
                <td className="px-6 py-4 font-mono text-xs text-gray-400">
                  {lead.partner_id}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        
        {filteredLeads.length === 0 && (
          <div className="p-20 text-center text-gray-400">
            <div className="w-12 h-12 rounded-full bg-gray-50 flex items-center justify-center mx-auto mb-4 border border-gray-100">
              <svg className="w-6 h-6 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
            </div>
            <p className="text-lg font-bold italic tracking-tight text-navy">No signals matching "{searchTerm}" captured.</p>
            <p className="text-xs font-mono uppercase opacity-50 mt-1">Check search parameters or monitoring nodes.</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default LeadLedger;
