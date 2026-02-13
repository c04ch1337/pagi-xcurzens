
import React from 'react';

interface AIInsightBarProps {
  insight: string;
}

const AIInsightBar: React.FC<AIInsightBarProps> = ({ insight }) => {
  return (
    <div className="bg-navy-900 border-l-4 orange-border navy-bg p-4 rounded-r shadow-lg relative overflow-hidden">
      {/* Background Decor */}
      <div className="absolute -right-4 -bottom-4 opacity-5 pointer-events-none">
        <svg className="w-32 h-32 text-white" fill="currentColor" viewBox="0 0 20 20">
          <path d="M10 2a8 8 0 100 16 8 8 0 000-16zM9 10a1 1 0 112 0 1 1 0 01-2 0zM9 13a1 1 0 112 0 1 1 0 01-2 0z" />
        </svg>
      </div>

      <div className="flex items-center gap-4 relative z-10">
        <div className="flex-shrink-0">
          <div className="w-10 h-10 rounded-full bg-white/10 flex items-center justify-center border border-white/20">
            <svg className="w-6 h-6 text-orange-400 animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
        </div>
        <div>
          <h3 className="text-[10px] font-black text-orange-500 uppercase tracking-[0.2em] mb-0.5">Sovereign AI Insight</h3>
          <p className="text-white font-medium text-sm leading-relaxed italic opacity-90">
            "{insight}"
          </p>
        </div>
      </div>
    </div>
  );
};

export default AIInsightBar;
