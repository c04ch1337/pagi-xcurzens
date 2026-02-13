import { GoogleGenAI, Modality } from "@google/genai/web";
import { ScoutRequest } from "../types";

// Sovereign Monolith: API key is fetched from the backend, not from frontend .env
// The Rust gateway will inject configuration via /api/v1/config or HTML template
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

const getAI = (): GoogleGenAI => new GoogleGenAI({ apiKey });

export const streamScoutResponse = async (
  request: ScoutRequest,
  onChunk: (text: string) => void
) => {
  const brandName = request.businessName || "XCURZENS Scout";
  try {
    const ai = getAI();
    const responseStream = await ai.models.generateContentStream({
      model: "gemini-3-flash-preview",
      contents: `User Query: ${request.query}\nContext: City ${request.city}, Weather ${request.weather}`,
      config: {
        systemInstruction: `You are the ${brandName}, a sovereign agentic monolith designed for coastal adventures and travel logistics. 
        You were engineered by The Creator.
        You speak with precision, authority, and a focus on coastal destinations.
        
        STRICT FORMATTING RULES:
        1. Use HTML for formatting. 
        2. Important terms or highlights should be wrapped in <span style="color: #FA921C; font-weight: bold;"> (Orange)</span>.
        3. Headers or structured sections should be wrapped in <div style="color: #051C55; font-weight: bold; border-bottom: 1px solid #FA921C; margin-top: 10px;"> (Navy with Orange underline)</div>.
        4. Keep your responses actionable and relevant to the user's city (${request.city}) and weather (${request.weather}) if provided.
        5. DO NOT use markdown like ** or ##. Use HTML tags only.`,
        temperature: 0.7,
      },
    });

    for await (const chunk of responseStream) {
      const text = chunk.text;
      if (text) {
        onChunk(text);
      }
    }
  } catch (error) {
    console.error("Scout Error:", error);
    onChunk("SYSTEM ERROR: Scout communication link severed. Please check API credentials.");
  }
};

export const generateSpeech = async (text: string) => {
  try {
    const ai = getAI();
    const response = await ai.models.generateContent({
      model: "gemini-2.5-flash-preview-tts",
      contents: [{ parts: [{ text: `Read this as XCURZENS Scout: ${text}` }] }],
      config: {
        responseModalities: [Modality.AUDIO],
        speechConfig: {
          voiceConfig: {
            prebuiltVoiceConfig: { voiceName: 'Kore' },
          },
        },
      },
    });

    return response.candidates?.[0]?.content?.parts?.[0]?.inlineData?.data;
  } catch (error) {
    console.error("TTS Error:", error);
    return null;
  }
};
