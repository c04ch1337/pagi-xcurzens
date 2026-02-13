import React from 'react';

const BaseSvg = ({ children, className }: { children?: React.ReactNode; className?: string }) => (
  <svg
    viewBox="0 0 400 300"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={`w-full h-full bg-[#051C55] ${className || ''}`}
    preserveAspectRatio="xMidYMid slice"
  >
    <rect width="400" height="300" fill="#051C55" />
    {/* Subtle Grid Background */}
    <pattern id="grid" x="0" y="0" width="40" height="40" patternUnits="userSpaceOnUse">
      <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(255,255,255,0.03)" strokeWidth="1" />
    </pattern>
    <rect width="400" height="300" fill="url(#grid)" />
    {children}
  </svg>
);

export const NetworkTopologyImg = ({ className }: { className?: string }) => (
  <BaseSvg className={className}>
    <g stroke="#FA921C" strokeWidth="2" strokeLinecap="round">
      <circle cx="200" cy="150" r="8" fill="#FA921C" fillOpacity="0.2" />
      <circle cx="100" cy="80" r="4" fill="#051C55" />
      <circle cx="300" cy="80" r="4" fill="#051C55" />
      <circle cx="100" cy="220" r="4" fill="#051C55" />
      <circle cx="300" cy="220" r="4" fill="#051C55" />
      <circle cx="50" cy="150" r="3" fill="#051C55" opacity="0.5" />
      <circle cx="350" cy="150" r="3" fill="#051C55" opacity="0.5" />
      
      {/* Connections */}
      <line x1="200" y1="150" x2="100" y2="80" opacity="0.6" />
      <line x1="200" y1="150" x2="300" y2="80" opacity="0.6" />
      <line x1="200" y1="150" x2="100" y2="220" opacity="0.6" />
      <line x1="200" y1="150" x2="300" y2="220" opacity="0.6" />
      <line x1="100" y1="80" x2="50" y2="150" opacity="0.3" />
      <line x1="100" y1="220" x2="50" y2="150" opacity="0.3" />
      <line x1="300" y1="80" x2="350" y2="150" opacity="0.3" />
      <line x1="300" y1="220" x2="350" y2="150" opacity="0.3" />
    </g>
    <circle cx="200" cy="150" r="30" stroke="#FA921C" strokeOpacity="0.1" strokeWidth="1" />
    <circle cx="200" cy="150" r="60" stroke="#FA921C" strokeOpacity="0.05" strokeWidth="1" />
  </BaseSvg>
);

export const SecureStackImg = ({ className }: { className?: string }) => (
  <BaseSvg className={className}>
    <g transform="translate(125, 75)">
      {/* Back Layer */}
      <rect x="20" y="-10" width="110" height="140" rx="4" fill="#FA921C" fillOpacity="0.1" />
      
      {/* Server Unit 1 */}
      <rect x="0" y="0" width="150" height="30" rx="4" fill="#0A2569" stroke="#FA921C" strokeWidth="1" strokeOpacity="0.5" />
      <circle cx="15" cy="15" r="3" fill="#FA921C" />
      <circle cx="25" cy="15" r="3" fill="#FA921C" fillOpacity="0.3" />
      
      {/* Server Unit 2 */}
      <rect x="0" y="40" width="150" height="30" rx="4" fill="#0A2569" stroke="#FA921C" strokeWidth="1" strokeOpacity="0.5" />
      <circle cx="15" cy="55" r="3" fill="#FA921C" />
      <circle cx="25" cy="55" r="3" fill="#FA921C" fillOpacity="0.3" />
      
      {/* Server Unit 3 */}
      <rect x="0" y="80" width="150" height="30" rx="4" fill="#0A2569" stroke="#FA921C" strokeWidth="1" strokeOpacity="0.5" />
      <circle cx="15" cy="95" r="3" fill="#FA921C" />
      <circle cx="25" cy="95" r="3" fill="#FA921C" fillOpacity="0.3" />
      
       {/* Server Unit 4 */}
       <rect x="0" y="120" width="150" height="30" rx="4" fill="#0A2569" stroke="#FA921C" strokeWidth="1" strokeOpacity="0.5" />
      <circle cx="15" cy="135" r="3" fill="#FA921C" />
      <circle cx="25" cy="135" r="3" fill="#FA921C" fillOpacity="0.3" />
    </g>
  </BaseSvg>
);

export const CoastalMapImg = ({ className }: { className?: string }) => (
  <BaseSvg className={className}>
    <path 
      d="M-10 50 Q 50 40 80 80 T 150 120 T 250 100 T 350 180 T 410 160 V 310 H -10 Z" 
      fill="#FA921C" 
      fillOpacity="0.05" 
      stroke="#FA921C" 
      strokeWidth="1" 
      strokeOpacity="0.3"
    />
    <g>
      {/* Location Markers */}
      <circle cx="150" cy="120" r="4" fill="#FA921C" />
      <circle cx="150" cy="120" r="12" stroke="#FA921C" strokeWidth="1" strokeOpacity="0.3" />
      
      <circle cx="250" cy="100" r="3" fill="#fff" fillOpacity="0.5" />
      <circle cx="350" cy="180" r="3" fill="#fff" fillOpacity="0.5" />
      <circle cx="80" cy="80" r="3" fill="#fff" fillOpacity="0.5" />
    </g>
    <path d="M 150 120 L 250 100 L 350 180" stroke="white" strokeOpacity="0.1" strokeDasharray="4 4" />
  </BaseSvg>
);

export const PartnerLinkImg = ({ className }: { className?: string }) => (
  <BaseSvg className={className}>
    <g transform="translate(200, 150) rotate(-45)">
       {/* Hexagon 1 */}
       <path 
        d="M -30 0 L -15 -26 L 15 -26 L 30 0 L 15 26 L -15 26 Z" 
        fill="none" 
        stroke="#FA921C" 
        strokeWidth="2" 
        transform="translate(-20, 0)"
       />
       {/* Hexagon 2 */}
       <path 
        d="M -30 0 L -15 -26 L 15 -26 L 30 0 L 15 26 L -15 26 Z" 
        fill="none" 
        stroke="#fff" 
        strokeWidth="2" 
        strokeOpacity="0.5"
        transform="translate(20, 0)"
       />
       {/* Connection Points */}
       <circle cx="0" cy="0" r="4" fill="#FA921C" />
       <line x1="-20" y1="0" x2="20" y2="0" stroke="#FA921C" strokeWidth="1" opacity="0.5" />
    </g>
  </BaseSvg>
);

export const DataStreamImg = ({ className }: { className?: string }) => (
  <BaseSvg className={className}>
    <g opacity="0.8">
      <rect x="50" y="220" width="30" height="80" fill="#FA921C" fillOpacity="0.2" />
      <rect x="90" y="180" width="30" height="120" fill="#FA921C" fillOpacity="0.4" />
      <rect x="130" y="240" width="30" height="60" fill="#FA921C" fillOpacity="0.3" />
      <rect x="170" y="140" width="30" height="160" fill="#FA921C" fillOpacity="0.6" />
      <rect x="210" y="190" width="30" height="110" fill="#FA921C" fillOpacity="0.5" />
      <rect x="250" y="100" width="30" height="200" fill="#FA921C" fillOpacity="0.8" />
      <rect x="290" y="160" width="30" height="140" fill="#FA921C" fillOpacity="0.7" />
      <rect x="330" y="210" width="30" height="90" fill="#FA921C" fillOpacity="0.4" />
    </g>
    <polyline 
      points="65,220 105,180 145,240 185,140 225,190 265,100 305,160 345,210" 
      fill="none" 
      stroke="#fff" 
      strokeWidth="2" 
      strokeOpacity="0.5"
    />
  </BaseSvg>
);
