/**
 * DiagramViewer Component
 * 
 * PaperBanana-style diagram rendering for Phoenix Marie.
 * Renders Mermaid diagrams as interactive SVG with zoom/pan capabilities.
 * Supports local-only rendering to maintain sovereignty.
 */

import React, { useEffect, useRef, useState } from 'react';
import mermaid from 'mermaid';
import { ZoomIn, ZoomOut, Maximize2, Download } from 'lucide-react';

interface DiagramMetadata {
  created_at?: string;
  kb_key?: string;
  title?: string;
}

interface DiagramEnvelope {
  type: 'diagram';
  format: 'mermaid' | 'dot';
  content: string;
  metadata?: DiagramMetadata;
}

interface DiagramViewerProps {
  diagram: DiagramEnvelope;
  className?: string;
}

// Initialize Mermaid with secure, local-only configuration (Dark Mode Optimized)
mermaid.initialize({
  startOnLoad: false,
  theme: 'dark',
  securityLevel: 'strict', // Prevents external resource loading
  fontFamily: 'ui-monospace, monospace',
  themeVariables: {
    darkMode: true,
    primaryColor: '#3b82f6',
    primaryTextColor: '#e5e7eb',
    primaryBorderColor: '#60a5fa',
    lineColor: '#9ca3af',
    secondaryColor: '#8b5cf6',
    tertiaryColor: '#10b981',
    background: '#1f2937',
    mainBkg: '#1f2937',
    secondBkg: '#374151',
    tertiaryBkg: '#4b5563',
    textColor: '#e5e7eb',
    border1: '#4b5563',
    border2: '#6b7280',
  },
});

export const DiagramViewer: React.FC<DiagramViewerProps> = ({ diagram, className = '' }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [svgContent, setSvgContent] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState<number>(1);
  const [isFullscreen, setIsFullscreen] = useState<boolean>(false);

  useEffect(() => {
    const renderDiagram = async () => {
      if (!diagram.content || diagram.format !== 'mermaid') {
        setError('Unsupported diagram format. Currently only Mermaid is supported.');
        return;
      }

      try {
        // Generate unique ID for this diagram
        const id = `mermaid-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
        
        // Render the Mermaid diagram
        const { svg } = await mermaid.render(id, diagram.content);
        setSvgContent(svg);
        setError(null);
      } catch (err) {
        console.error('Mermaid rendering error:', err);
        setError(`Failed to render diagram: ${err instanceof Error ? err.message : 'Unknown error'}`);
      }
    };

    renderDiagram();
  }, [diagram.content, diagram.format]);

  const handleZoomIn = () => {
    setZoom((prev) => Math.min(prev + 0.2, 3));
  };

  const handleZoomOut = () => {
    setZoom((prev) => Math.max(prev - 0.2, 0.5));
  };

  const handleResetZoom = () => {
    setZoom(1);
  };

  const handleFullscreen = () => {
    if (!containerRef.current) return;

    if (!isFullscreen) {
      if (containerRef.current.requestFullscreen) {
        containerRef.current.requestFullscreen();
      }
    } else {
      if (document.exitFullscreen) {
        document.exitFullscreen();
      }
    }
    setIsFullscreen(!isFullscreen);
  };

  const handleDownload = () => {
    if (!svgContent) return;

    // Create a blob from the SVG content
    const blob = new Blob([svgContent], { type: 'image/svg+xml' });
    const url = URL.createObjectURL(blob);
    
    // Create download link
    const a = document.createElement('a');
    a.href = url;
    a.download = `phoenix-diagram-${diagram.metadata?.kb_key || Date.now()}.svg`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  if (error) {
    return (
      <div className={`diagram-viewer-error border border-red-500/30 bg-red-950/20 rounded-lg p-4 ${className}`}>
        <div className="flex items-start gap-3">
          <div className="text-red-400 text-sm">⚠️</div>
          <div>
            <div className="text-red-400 font-semibold mb-1">Diagram Rendering Error</div>
            <div className="text-red-300/80 text-sm">{error}</div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div 
      ref={containerRef}
      className={`diagram-viewer border border-blue-500/30 bg-gray-900/50 rounded-lg overflow-hidden ${className}`}
    >
      {/* Header with metadata and controls */}
      <div className="diagram-header flex items-center justify-between px-4 py-2 bg-gray-800/50 border-b border-gray-700">
        <div className="flex items-center gap-3">
          <div className="text-blue-400 text-sm font-semibold">
            📊 {diagram.metadata?.title || 'Phoenix Diagram'}
          </div>
          {diagram.metadata?.kb_key && (
            <div className="text-gray-400 text-xs">
              KB-05: {diagram.metadata.kb_key}
            </div>
          )}
        </div>
        
        {/* Control buttons */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleZoomOut}
            className="p-1.5 hover:bg-gray-700 rounded transition-colors"
            title="Zoom Out"
          >
            <ZoomOut className="w-4 h-4 text-gray-300" />
          </button>
          <button
            onClick={handleResetZoom}
            className="px-2 py-1 text-xs hover:bg-gray-700 rounded transition-colors text-gray-300"
            title="Reset Zoom"
          >
            {Math.round(zoom * 100)}%
          </button>
          <button
            onClick={handleZoomIn}
            className="p-1.5 hover:bg-gray-700 rounded transition-colors"
            title="Zoom In"
          >
            <ZoomIn className="w-4 h-4 text-gray-300" />
          </button>
          <div className="w-px h-4 bg-gray-600 mx-1" />
          <button
            onClick={handleFullscreen}
            className="p-1.5 hover:bg-gray-700 rounded transition-colors"
            title="Fullscreen"
          >
            <Maximize2 className="w-4 h-4 text-gray-300" />
          </button>
          <button
            onClick={handleDownload}
            className="p-1.5 hover:bg-gray-700 rounded transition-colors"
            title="Download SVG"
          >
            <Download className="w-4 h-4 text-gray-300" />
          </button>
        </div>
      </div>

      {/* Diagram content with zoom/pan */}
      <div className="diagram-content overflow-auto p-6 bg-gray-900/30" style={{ minHeight: '300px' }}>
        {svgContent ? (
          <div 
            className="diagram-svg-container mermaid flex items-center justify-center"
            style={{ 
              transform: `scale(${zoom})`,
              transformOrigin: 'center center',
              transition: 'transform 0.2s ease-out'
            }}
            dangerouslySetInnerHTML={{ __html: svgContent }}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-gray-400">
            <div className="animate-pulse">Rendering diagram...</div>
          </div>
        )}
      </div>

      {/* Footer with timestamp */}
      {diagram.metadata?.created_at && (
        <div className="diagram-footer px-4 py-2 bg-gray-800/30 border-t border-gray-700 text-xs text-gray-500">
          Generated: {new Date(diagram.metadata.created_at).toLocaleString()}
        </div>
      )}
    </div>
  );
};

export default DiagramViewer;
