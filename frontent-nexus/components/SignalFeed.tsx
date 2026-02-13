import React, { useState, useMemo } from 'react';
import { Lead } from '../types';
import { ArrowLeftIcon, SearchIcon, FilterIcon, SignalIcon, RefreshIcon } from './Icons';

interface SignalFeedProps {
  leads: Lead[];
  onSelectLead: (lead: Lead) => void;
  onBack: () => void;
}

export const SignalFeed: React.FC<SignalFeedProps> = ({ leads, onSelectLead, onBack }) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'new' | 'processing' | 'synced'>('all');
  const [sectorFilter, setSectorFilter] = useState<string>('all');
  const [sourceFilter, setSourceFilter] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'id' | 'timestamp' | 'status'>('timestamp');

  // Extract unique options for dropdowns
  const sectors = useMemo(() => Array.from(new Set(leads.map(l => l.sector))), [leads]);
  const sources = useMemo(() => Array.from(new Set(leads.map(l => l.source))), [leads]);

  // Helper to generate consistent mock IDs for sources based on their name
  const getSourceId = (sourceName: string) => {
    const hash = sourceName.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
    return `SRC-${(hash % 1000).toString().padStart(3, '0')}`;
  };

  const filteredLeads = useMemo(() => {
    return leads
      .filter(lead => {
        const matchesSearch = 
          lead.id.toLowerCase().includes(searchTerm.toLowerCase()) ||
          lead.source.toLowerCase().includes(searchTerm.toLowerCase()) ||
          getSourceId(lead.source).toLowerCase().includes(searchTerm.toLowerCase()) ||
          lead.sector.toLowerCase().includes(searchTerm.toLowerCase());
        
        const matchesStatus = statusFilter === 'all' || lead.status === statusFilter;
        const matchesSector = sectorFilter === 'all' || lead.sector === sectorFilter;
        const matchesSource = sourceFilter === 'all' || lead.source === sourceFilter;

        return matchesSearch && matchesStatus && matchesSector && matchesSource;
      })
      .sort((a, b) => {
        if (sortBy === 'id') {
           return a.id.localeCompare(b.id);
        } else if (sortBy === 'status') {
           return a.status.localeCompare(b.status);
        } else {
           // Default to timestamp descending (newest first)
           return b.timestamp.localeCompare(a.timestamp); 
        }
      });
  }, [leads, searchTerm, statusFilter, sectorFilter, sourceFilter, sortBy]);

  const resetFilters = () => {
    setSearchTerm('');
    setStatusFilter('all');
    setSectorFilter('all');
    setSourceFilter('all');
    setSortBy('timestamp');
  };

  const activeFiltersCount = [
    statusFilter !== 'all',
    sectorFilter !== 'all',
    sourceFilter !== 'all',
    searchTerm !== ''
  ].filter(Boolean).length;

  return (
    <div className="w-full max-w-6xl mx-auto space-y-6 animate-fade-in-up">
      
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-4">
        <div className="flex items-center gap-4">
          <button 
            onClick={onBack}
            className="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 hover:text-white transition-colors border border-white/10"
          >
            <ArrowLeftIcon className="w-5 h-5" />
          </button>
          <div>
            <h2 className="text-xl font-bold text-white flex items-center gap-2">
              Live Signal Feed Sources
            </h2>
            <p className="text-xs text-gray-400">Registry of incoming infrastructure signals and source IDs.</p>
          </div>
        </div>
        <div className="flex items-center gap-6">
            <div className="text-right hidden sm:block">
                <p className="text-[10px] text-gray-500 font-mono uppercase tracking-wider">Active Sources</p>
                <p className="text-lg font-bold text-white leading-none">{sources.length}</p>
            </div>
            <div className="text-right hidden sm:block">
                <p className="text-[10px] text-gray-500 font-mono uppercase tracking-wider">Total Records</p>
                <p className="text-lg font-bold text-nexus-orange leading-none">{filteredLeads.length}</p>
            </div>
        </div>
      </div>

      {/* Controls */}
      <div className="bg-nexus-glass backdrop-blur-xl border border-nexus-glassBorder rounded-xl p-4 flex flex-col gap-4 shadow-lg">
         
         {/* Top Row: Search & Reset */}
         <div className="flex gap-4">
           <div className="relative w-full">
              <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                 <SearchIcon className="w-4 h-4" />
              </div>
              <input 
                type="text" 
                placeholder="Search Signal ID, Source ID, or Type..." 
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full bg-nexus-navy/60 border border-white/10 rounded-lg pl-10 pr-4 py-2 text-sm text-white focus:outline-none focus:border-nexus-orange focus:ring-1 focus:ring-nexus-orange placeholder-gray-500 transition-all"
              />
           </div>
           {activeFiltersCount > 0 && (
             <button 
               onClick={resetFilters}
               className="px-4 py-2 text-xs font-medium text-gray-400 hover:text-white bg-white/5 hover:bg-white/10 border border-white/10 rounded-lg transition-colors whitespace-nowrap flex items-center gap-2"
             >
               <RefreshIcon className="w-3 h-3" />
               Reset
             </button>
           )}
         </div>

         {/* Bottom Row: Filters */}
         <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            
            {/* Status Filter */}
            <div className="relative group">
              <select 
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value as any)}
                className="w-full bg-nexus-navy/60 border border-white/10 group-hover:border-white/20 rounded-lg px-3 py-2 text-xs text-white appearance-none focus:outline-none focus:border-nexus-orange cursor-pointer transition-colors"
              >
                <option value="all">Status: All</option>
                <option value="new">Status: New</option>
                <option value="processing">Status: Processing</option>
                <option value="synced">Status: Synced</option>
              </select>
               <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-500">▼</div>
            </div>

            {/* Sector Filter */}
            <div className="relative group">
              <select 
                value={sectorFilter}
                onChange={(e) => setSectorFilter(e.target.value)}
                className="w-full bg-nexus-navy/60 border border-white/10 group-hover:border-white/20 rounded-lg px-3 py-2 text-xs text-white appearance-none focus:outline-none focus:border-nexus-orange cursor-pointer transition-colors"
              >
                <option value="all">Sector: All</option>
                {sectors.map(s => <option key={s} value={s}>{s}</option>)}
              </select>
               <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-500">▼</div>
            </div>

            {/* Source Filter */}
            <div className="relative group">
              <select 
                value={sourceFilter}
                onChange={(e) => setSourceFilter(e.target.value)}
                className="w-full bg-nexus-navy/60 border border-white/10 group-hover:border-white/20 rounded-lg px-3 py-2 text-xs text-white appearance-none focus:outline-none focus:border-nexus-orange cursor-pointer transition-colors"
              >
                <option value="all">Source: All</option>
                {sources.map(s => <option key={s} value={s}>{s}</option>)}
              </select>
               <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-gray-500">▼</div>
            </div>

            {/* Sort Toggle */}
            <div className="flex bg-nexus-navy/60 border border-white/10 rounded-lg p-1">
               <button 
                 onClick={() => setSortBy('timestamp')}
                 className={`flex-1 px-2 py-1 text-[10px] font-medium rounded transition-colors ${sortBy === 'timestamp' ? 'bg-white/10 text-white shadow' : 'text-gray-400 hover:text-white'}`}
               >
                 Time
               </button>
               <button 
                 onClick={() => setSortBy('id')}
                 className={`flex-1 px-2 py-1 text-[10px] font-medium rounded transition-colors ${sortBy === 'id' ? 'bg-white/10 text-white shadow' : 'text-gray-400 hover:text-white'}`}
               >
                 ID
               </button>
               <button 
                 onClick={() => setSortBy('status')}
                 className={`flex-1 px-2 py-1 text-[10px] font-medium rounded transition-colors ${sortBy === 'status' ? 'bg-white/10 text-white shadow' : 'text-gray-400 hover:text-white'}`}
               >
                 Status
               </button>
            </div>
         </div>
      </div>

      {/* Table */}
      <div className="bg-nexus-navy/40 backdrop-blur-xl border border-nexus-glassBorder rounded-2xl shadow-2xl relative overflow-hidden min-h-[500px]">
        <div className="overflow-x-auto">
             <table className="w-full text-sm text-left">
               <thead className="text-xs text-gray-400 uppercase bg-black/20 sticky top-0 backdrop-blur-md z-10">
                 <tr>
                   <th className="px-6 py-4 font-medium tracking-wider">Signal ID</th>
                   <th className="px-6 py-4 font-medium tracking-wider">Source Origin</th>
                   <th className="px-6 py-4 font-medium tracking-wider">Sector</th>
                   <th className="px-6 py-4 font-medium tracking-wider">Timestamp</th>
                   <th className="px-6 py-4 text-right font-medium tracking-wider">Status</th>
                 </tr>
               </thead>
               <tbody className="divide-y divide-white/5">
                 {filteredLeads.length > 0 ? (
                   filteredLeads.map((lead) => (
                     <tr 
                       key={lead.id} 
                       onClick={() => onSelectLead(lead)}
                       className="hover:bg-white/5 transition-all duration-200 group cursor-pointer border-l-2 border-transparent hover:border-nexus-orange"
                     >
                       <td className="px-6 py-4">
                          <span className="font-mono text-blue-300 group-hover:text-nexus-orange transition-colors flex items-center gap-2 font-medium">
                             <SignalIcon className="w-3 h-3 opacity-50" />
                             {lead.id}
                          </span>
                       </td>
                       <td className="px-6 py-4">
                          <div className="flex flex-col">
                            <span className="text-white font-medium text-sm">{lead.source}</span>
                            <span className="text-[10px] text-gray-500 font-mono mt-0.5 group-hover:text-gray-400 transition-colors">
                                {getSourceId(lead.source)}
                            </span>
                          </div>
                       </td>
                       <td className="px-6 py-4 text-gray-300">
                          <span className="bg-white/5 px-2 py-1 rounded text-xs border border-white/5">{lead.sector}</span>
                       </td>
                       <td className="px-6 py-4 text-gray-400 font-mono text-xs tabular-nums">{lead.timestamp}</td>
                       <td className="px-6 py-4 text-right">
                         <span className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium border shadow-sm
                           ${lead.status === 'new' ? 'bg-nexus-orange/10 text-nexus-orange border-nexus-orange/20' : 
                             lead.status === 'processing' ? 'bg-blue-500/10 text-blue-400 border-blue-500/20' : 
                             'bg-green-500/10 text-green-400 border-green-500/20'}`}>
                           {lead.status === 'new' && <span className="w-1.5 h-1.5 rounded-full bg-nexus-orange animate-pulse"></span>}
                           {lead.status.toUpperCase()}
                         </span>
                       </td>
                     </tr>
                   ))
                 ) : (
                   <tr>
                     <td colSpan={5} className="px-6 py-16 text-center text-gray-500">
                        <div className="flex flex-col items-center gap-2">
                           <FilterIcon className="w-8 h-8 opacity-20" />
                           <p>No signal sources match your active filters.</p>
                           <button 
                             onClick={resetFilters} 
                             className="text-nexus-orange hover:text-white text-xs underline underline-offset-4 mt-2 transition-colors"
                           >
                             Clear all filters
                           </button>
                        </div>
                     </td>
                   </tr>
                 )}
               </tbody>
             </table>
           </div>
      </div>
    </div>
  );
};