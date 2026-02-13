
import { GoogleGenAI } from "@google/genai";
import { InfrastructureData } from "../types";

// Sovereign Monolith: API key is fetched from the backend, not from frontend .env
let apiKey = '';

// Fetch API key from backend on module load
(async () => {
  try {
    const response = await fetch('/api/v1/config');
    const config = await response.json();
    apiKey = config.gemini_api_key || '';
  } catch (error) {
    console.error('Failed to fetch API key from backend:', error);
  }
})();

export const getAIInsight = async (data: InfrastructureData): Promise<string> => {
  const ai = new GoogleGenAI({ apiKey });
  
  const prompt = `
    Analyze the following XCURZENS system state and provide a single-sentence executive insight for The Creator (Root Sovereign).
    
    System Summary:
    - Total Leads: ${data.system_summary.total_leads}
    - High Intent Leads: ${data.system_summary.high_intent_leads}
    - Active Partners: ${data.system_summary.active_partners}
    
    Lead Samples:
    ${data.leads.slice(0, 3).map(l => `- ${l.query_snippet} (${l.city})`).join('\n')}
    
    Instruction: Be concise, authoritative, and mention any anomalies or key opportunities. Focus on high-intent conversion potential.
  `;

  try {
    const response = await ai.models.generateContent({
      model: "gemini-3-flash-preview",
      contents: prompt,
      config: {
        maxOutputTokens: 100,
        temperature: 0.7,
      },
    });

    return response.text || "Systems stable. High-intent flow optimal.";
  } catch (error) {
    console.error("Gemini Insight Error:", error);
    return "Error generating AI insight. Monitor lead velocity manually.";
  }
};
