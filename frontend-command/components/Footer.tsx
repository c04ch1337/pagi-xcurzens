
import React from 'react';

const Footer: React.FC = () => {
  return (
    <footer className="fixed bottom-0 left-0 right-0 h-10 navy-bg flex items-center justify-between px-6 text-[10px] text-gray-400 font-bold tracking-widest uppercase z-50 border-t border-white/5">
      <div className="flex items-center gap-4">
        <span>Bare Metal Infrastructure</span>
        <span className="w-1 h-1 rounded-full bg-gray-600"></span>
        <span className="flex items-center gap-1">
          <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
          Live Bandwidth Monitoring: 1.2 GB/s
        </span>
      </div>
      <div className="hidden md:block">
        &copy; {new Date().getFullYear()} XCURZENS NEXUS • SECURE ROOT ACCESS ONLY
      </div>
      <div className="flex items-center gap-4">
        <span className="text-orange-500">Node-07 Active</span>
        <span className="w-1 h-1 rounded-full bg-gray-600"></span>
        <span>AES-256 Encrypted Session</span>
      </div>
    </footer>
  );
};

export default Footer;
