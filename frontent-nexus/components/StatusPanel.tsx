import React from 'react';
import { RegistrationStatus } from '../types';
import { CheckCircleIcon, AlertTriangleIcon } from './Icons';

interface StatusPanelProps {
  status: RegistrationStatus;
  message?: string;
}

export const StatusPanel: React.FC<StatusPanelProps> = ({ status, message }) => {
  if (status === 'idle' || status === 'submitting') {
    return <div id="status-panel" className="h-0 opacity-0 overflow-hidden transition-all" />;
  }

  const isSuccess = status === 'success';

  return (
    <div 
      id="status-panel" 
      className={`mt-6 p-6 rounded-lg border backdrop-blur-md animate-fade-in-up transition-all duration-500
        ${isSuccess 
          ? 'bg-nexus-orange/10 border-nexus-orange text-white' 
          : 'bg-red-500/10 border-red-500 text-white'
        }`}
    >
      <div className="flex items-start gap-4">
        <div className={`p-2 rounded-full ${isSuccess ? 'bg-nexus-orange' : 'bg-red-500'}`}>
          {isSuccess ? (
            <CheckCircleIcon className="w-6 h-6 text-white" />
          ) : (
            <AlertTriangleIcon className="w-6 h-6 text-white" />
          )}
        </div>
        <div>
          <h3 className="text-lg font-bold mb-1">
            {isSuccess ? 'Infrastructure Synchronized' : 'Bandwidth Error'}
          </h3>
          <p className="text-sm opacity-90 leading-relaxed font-light">
            {message}
          </p>
        </div>
      </div>
    </div>
  );
};