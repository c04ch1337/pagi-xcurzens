
import React, { useEffect } from 'react';
import { Lead } from '../types';

interface LeadDetailModalProps {
  lead: Lead;
  isOpen: boolean;
  onClose: () => void;
}

const LeadDetailModal: React.FC<LeadDetailModalProps> = ({ lead, isOpen, onClose }) => {
  // Handle escape key
  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleEsc);
    return () => window.removeEventListener('keydown', handleEsc);
  }, [onClose]);

  if (!isOpen) return null;

  const intentLower = lead.intent.toLowerCase();

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      {/* Backdrop */}
      <div 
        className="absolute inset-0 bg-navy/80 backdrop-blur-sm"
        onClick={onClose}
      />
      
      {/* Modal Container */}
      <div className="bg-white w-full max-w-2xl rounded-xl shadow-2xl overflow-hidden relative z-10 animate-in fade-in zoom-in duration-200">
        {/* Header */}
        <div className={`navy-bg px-6 py-4 flex items-center justify-between border-b ${intentLower === 'high' ? 'border-orange-500 border-b-2' : 'border-white/10'}`}>
          <div className="flex items-center gap-3">
            <div className={`${intentLower === 'high' ? 'navy-bg border border-white/20' : 'bg-white/20'} p-2 rounded`}>
               <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                 <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
               </svg>
            </div>
            <div>
              <h2 className="text-white font-bold text-lg leading-none uppercase tracking-tight">Lead Transmission Details</h2>
              <p className="text-xs text-gray-400 mt-1 font-mono">{lead.id}</p>
            </div>
          </div>
          <button 
            onClick={onClose}
            className="text-white/50 hover:text-white transition-colors"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content Body */}
        <div className="p-6">
          {/* Query Snippet - Hero Section */}
          <div className="mb-8">
            <h3 className="text-[10px] font-black text-gray-400 uppercase tracking-widest mb-2">Primary Query Transmission</h3>
            <div className="bg-gray-50 border border-gray-100 p-4 rounded-lg">
              <p className="text-lg font-semibold text-navy leading-snug">
                "{lead.query_snippet}"
              </p>
            </div>
          </div>

          {/* Details Grid */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-y-6 gap-x-12">
            <DetailItem label="Capture Timestamp" value={lead.timestamp} />
            <DetailItem label="Geographic Node" value={lead.city} />
            <DetailItem label="Environmental Data" value={lead.weather} />
            <DetailItem label="Associated Partner" value={lead.partner_id} />
            
            <div className="md:col-span-2">
               <h3 className="text-[10px] font-black text-gray-400 uppercase tracking-widest mb-2">Intent Classification</h3>
               <div className="flex items-center gap-3">
                 <span className={`
                    px-4 py-1.5 rounded-full text-xs font-black uppercase shadow-sm text-white
                    ${intentLower === 'high' ? 'navy-bg ring-4 ring-navy/10' : 
                      intentLower === 'medium' ? 'orange-bg' : 'bg-gray-100 !text-gray-600'}
                  `}>
                    {lead.intent} INTENT
                 </span>
                 {intentLower === 'high' && (
                   <span className="text-[10px] font-bold text-navy animate-pulse">
                     [SOVEREIGN PRIORITY SIGNAL]
                   </span>
                 )}
               </div>
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="bg-gray-50 px-6 py-4 flex justify-end items-center gap-3 border-t border-gray-100">
           <p className="text-[9px] text-gray-400 font-bold uppercase tracking-tighter mr-auto">
             Encrypted Payload • Access Level 0 (Root)
           </p>
           <button 
             onClick={onClose}
             className="px-6 py-2 rounded-lg font-bold text-xs uppercase tracking-widest navy-bg text-white hover:bg-navy/90 transition-all shadow-lg"
           >
             Acknowledge
           </button>
        </div>
      </div>
    </div>
  );
};

interface DetailItemProps {
  label: string;
  value: string;
}

const DetailItem: React.FC<DetailItemProps> = ({ label, value }) => (
  <div>
    <h3 className="text-[10px] font-black text-gray-400 uppercase tracking-widest mb-1">{label}</h3>
    <p className="text-navy font-bold text-sm tracking-tight">{value}</p>
  </div>
);

export default LeadDetailModal;
