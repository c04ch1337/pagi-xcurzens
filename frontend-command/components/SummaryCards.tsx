
import React from 'react';
import { SystemSummary } from '../types';

interface SummaryCardsProps {
  summary: SystemSummary;
}

const SummaryCards: React.FC<SummaryCardsProps> = ({ summary }) => {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {/* Total Leads Card */}
      <div className="bg-white p-5 rounded-lg border border-gray-200 shadow-sm transition-transform hover:scale-[1.01]">
        <p className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1">Total Leads</p>
        <div className="flex items-baseline gap-2">
          <span className="text-3xl font-black navy-text">{summary.total_leads.toLocaleString()}</span>
          <span className="text-xs text-green-600 font-bold">+12.5%</span>
        </div>
      </div>

      {/* High-Intent Leads Card - Orange Highlighted */}
      <div className="bg-white p-5 rounded-lg border-2 orange-border shadow-md transition-transform hover:scale-[1.01] relative overflow-hidden group">
        <div className="absolute top-0 right-0 p-2 opacity-10 group-hover:opacity-20 transition-opacity">
           <svg className="w-12 h-12" fill="currentColor" viewBox="0 0 20 20">
             <path fillRule="evenodd" d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z" clipRule="evenodd" />
           </svg>
        </div>
        <p className="text-xs font-bold orange-text uppercase tracking-wider mb-1">High-Intent Leads</p>
        <div className="flex items-baseline gap-2">
          <span className="text-3xl font-black orange-text">{summary.high_intent_leads.toLocaleString()}</span>
          <span className="text-xs orange-text opacity-70 font-medium">Critical focus</span>
        </div>
      </div>

      {/* Active Partners Card */}
      <div className="bg-white p-5 rounded-lg border border-gray-200 shadow-sm transition-transform hover:scale-[1.01]">
        <p className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1">Active Partners</p>
        <div className="flex items-baseline gap-2">
          <span className="text-3xl font-black navy-text">{summary.active_partners}</span>
          <span className="text-xs text-blue-500 font-medium uppercase">Network Nodes</span>
        </div>
      </div>

      {/* System Uptime Card */}
      <div className="bg-white p-5 rounded-lg border border-gray-200 shadow-sm transition-transform hover:scale-[1.01]">
        <p className="text-xs font-bold text-gray-500 uppercase tracking-wider mb-1">Infrastructure Health</p>
        <div className="flex items-baseline gap-2">
          <span className="text-3xl font-black text-emerald-600">{summary.system_uptime}</span>
          <span className="text-xs text-emerald-600/70 font-medium">Bare Metal Stable</span>
        </div>
      </div>
    </div>
  );
};

export default SummaryCards;
