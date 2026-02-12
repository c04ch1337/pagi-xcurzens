/**
 * Type declarations for mermaid (used by DiagramViewer).
 * Install the package with: npm install
 */
declare module 'mermaid' {
  export interface MermaidConfig {
    startOnLoad?: boolean;
    theme?: string;
    securityLevel?: 'strict' | 'loose' | 'antiscript' | 'sandbox';
    fontFamily?: string;
    themeVariables?: Record<string, string | boolean>;
  }

  export interface RenderResult {
    svg: string;
  }

  export function initialize(config: MermaidConfig): void;
  export function render(id: string, content: string): Promise<RenderResult>;

  const mermaid: {
    initialize: typeof initialize;
    render: typeof render;
  };
  export default mermaid;
}
