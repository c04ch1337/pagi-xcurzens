import React, { useState, useEffect, useRef } from 'react';
import { GeoContext, Message, MessageRole } from './types';
import { streamScoutResponse, generateSpeech } from './services/geminiService';
import { marked } from 'marked';

// Types & Helpers
declare global {
  interface Window {
    SpeechRecognition: any;
    webkitSpeechRecognition: any;
  }
}

const STORAGE_KEY = 'xcurzens_scout_history';
const UI_SETTINGS_KEY = 'xcurzens_ui_settings';
const AUDIO_SETTINGS_KEY = 'xcurzens_audio_settings';

const DEFAULT_BRAND = "XCURZENS SCOUT";

const INITIAL_MESSAGE: Message = {
  id: 'init',
  role: MessageRole.SCOUT,
  text: 'Welcome to your sovereign agentic interface for coastal intelligence. Ask me about adventures in Corpus Christi or your current location.',
  timestamp: Date.now(),
};

interface UISettings {
  logoUrl: string;
  customCss: string;
  businessName: string;
}

interface AudioSettings {
  volume: number;
  isMuted: boolean;
  autoPlay: boolean;
}

function decode(base64: string) {
  const binaryString = atob(base64);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

async function decodeAudioData(
  data: Uint8Array,
  ctx: AudioContext,
  sampleRate: number,
  numChannels: number,
): Promise<AudioBuffer> {
  const dataInt16 = new Int16Array(data.buffer);
  const frameCount = dataInt16.length / numChannels;
  const buffer = ctx.createBuffer(numChannels, frameCount, sampleRate);

  for (let channel = 0; channel < numChannels; channel++) {
    const channelData = buffer.getChannelData(channel);
    for (let i = 0; i < frameCount; i++) {
      channelData[i] = dataInt16[i * numChannels + channel] / 32768.0;
    }
  }
  return buffer;
}

const App: React.FC = () => {
  // Chat History Logic
  const [messages, setMessages] = useState<Message[]>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        return Array.isArray(parsed) && parsed.length > 0 ? parsed : [INITIAL_MESSAGE];
      } catch (e) {
        return [INITIAL_MESSAGE];
      }
    }
    return [INITIAL_MESSAGE];
  });
  
  const [inputValue, setInputValue] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [isVocalizing, setIsVocalizing] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [systemStatus, setSystemStatus] = useState<'Stable' | 'Offline'>('Stable');
  const [geoContext, setGeoContext] = useState<GeoContext>({
    city: '[Detecting...]',
    weather: '[Detecting...]'
  });

  // UI Settings State
  const [uiSettings, setUiSettings] = useState<UISettings>(() => {
    const saved = localStorage.getItem(UI_SETTINGS_KEY);
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        return {
          logoUrl: parsed.logoUrl || '',
          customCss: parsed.customCss || '',
          businessName: parsed.businessName || DEFAULT_BRAND
        };
      } catch (e) {}
    }
    return { logoUrl: '', customCss: '', businessName: DEFAULT_BRAND };
  });

  // Audio Configuration State
  const [audioSettings, setAudioSettings] = useState<AudioSettings>(() => {
    const saved = localStorage.getItem(AUDIO_SETTINGS_KEY);
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        return {
          volume: parsed.volume ?? 0.7,
          isMuted: parsed.isMuted ?? false,
          autoPlay: parsed.autoPlay ?? true
        };
      } catch (e) {}
    }
    return { volume: 0.7, isMuted: false, autoPlay: true };
  });

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [settingsForm, setSettingsForm] = useState<UISettings>(uiSettings);

  // Sync Document Title
  useEffect(() => {
    document.title = uiSettings.businessName;
  }, [uiSettings.businessName]);

  // Shared Refs
  const scrollRef = useRef<HTMLDivElement>(null);
  const recognitionRef = useRef<any>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const gainNodeRef = useRef<GainNode | null>(null);
  const customStyleRef = useRef<HTMLStyleElement | null>(null);
  const activeSourceRef = useRef<AudioBufferSourceNode | null>(null);

  // Initialize Audio Engine
  useEffect(() => {
    audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)({ sampleRate: 24000 });
    gainNodeRef.current = audioContextRef.current.createGain();
    gainNodeRef.current.connect(audioContextRef.current.destination);
  }, []);

  // Save Settings
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(messages));
  }, [messages]);

  useEffect(() => {
    localStorage.setItem(UI_SETTINGS_KEY, JSON.stringify(uiSettings));
    if (customStyleRef.current) customStyleRef.current.textContent = uiSettings.customCss;
  }, [uiSettings]);

  useEffect(() => {
    localStorage.setItem(AUDIO_SETTINGS_KEY, JSON.stringify(audioSettings));
    if (gainNodeRef.current) {
      const targetVolume = audioSettings.isMuted ? 0 : audioSettings.volume;
      gainNodeRef.current.gain.setTargetAtTime(targetVolume, audioContextRef.current?.currentTime || 0, 0.1);
    }
  }, [audioSettings]);

  // Resume Audio on Interaction
  const resumeAudio = async () => {
    if (audioContextRef.current?.state === 'suspended') {
      await audioContextRef.current.resume();
    }
  };

  // Auto-scroll
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
    }
  }, [messages, isStreaming]);

  // Geo-Context Loader
  useEffect(() => {
    const fetchGeoData = async () => {
      if (navigator.geolocation) {
        navigator.geolocation.getCurrentPosition(async (pos) => {
          try {
            const geoRes = await fetch(`https://nominatim.openstreetmap.org/reverse?format=json&lat=${pos.coords.latitude}&lon=${pos.coords.longitude}`);
            const geoData = await geoRes.json();
            const city = geoData.address.city || geoData.address.town || "Coastal Region";
            setGeoContext(prev => ({ ...prev, city }));
          } catch (e) {}
        });
      }
    };
    fetchGeoData();
  }, []);

  // Speech Recognition Logic
  useEffect(() => {
    const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
    if (SpeechRecognition) {
      const recognition = new SpeechRecognition();
      recognition.onstart = () => setIsListening(true);
      recognition.onend = () => setIsListening(false);
      recognition.onresult = (event: any) => {
        const transcript = event.results[0][0].transcript;
        if (transcript) setInputValue(prev => (prev ? prev + ' ' : '') + transcript);
      };
      recognitionRef.current = recognition;
    }
  }, []);

  const toggleListening = () => {
    resumeAudio();
    if (!recognitionRef.current) return alert("Vocal interface unavailable.");
    isListening ? recognitionRef.current.stop() : recognitionRef.current.start();
  };

  const stopCurrentAudio = () => {
    if (activeSourceRef.current) {
      try { activeSourceRef.current.stop(); } catch (e) {}
      activeSourceRef.current = null;
    }
    setIsVocalizing(null);
  };

  const playScoutAudio = async (text: string, messageId: string) => {
    await resumeAudio();
    if (isVocalizing === messageId) {
      stopCurrentAudio();
      return;
    }
    stopCurrentAudio();
    if (!audioContextRef.current || !gainNodeRef.current) return;
    setIsVocalizing(messageId);
    
    try {
      const base64Audio = await generateSpeech(text.replace(/<[^>]*>/g, ''));
      if (base64Audio) {
        const audioBuffer = await decodeAudioData(decode(base64Audio), audioContextRef.current, 24000, 1);
        const source = audioContextRef.current.createBufferSource();
        source.buffer = audioBuffer;
        source.connect(gainNodeRef.current);
        source.onended = () => { if (activeSourceRef.current === source) setIsVocalizing(null); };
        activeSourceRef.current = source;
        source.start();
      } else {
        setIsVocalizing(null);
      }
    } catch (e) {
      setIsVocalizing(null);
    }
  };

  const handleSend = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!inputValue.trim() || isStreaming) return;
    await resumeAudio();

    const userQuery = inputValue.trim();
    setInputValue('');
    if (isListening) recognitionRef.current?.stop();

    let nextMessages = [...messages];
    if (editingId) {
      const idx = nextMessages.findIndex(m => m.id === editingId);
      if (idx !== -1) nextMessages = nextMessages.slice(0, idx);
      setEditingId(null);
    }

    const userMessage = { id: Date.now().toString(), role: MessageRole.USER, text: userQuery, timestamp: Date.now() };
    const scoutMessageId = (Date.now() + 1).toString();
    const scoutPlaceholder = { id: scoutMessageId, role: MessageRole.SCOUT, text: '', timestamp: Date.now() };

    setMessages([...nextMessages, userMessage, scoutPlaceholder]);
    setIsStreaming(true);

    let accumulatedText = '';
    await streamScoutResponse(
      { query: userQuery, city: geoContext.city, weather: geoContext.weather, businessName: uiSettings.businessName },
      (chunk) => {
        accumulatedText += chunk;
        setMessages(prev => prev.map(m => m.id === scoutMessageId ? { ...m, text: accumulatedText } : m));
      }
    );

    setIsStreaming(false);
    if (audioSettings.autoPlay && !audioSettings.isMuted) playScoutAudio(accumulatedText, scoutMessageId);
  };

  const renderBrand = (name: string, isPreview = false) => {
    const brand = name || DEFAULT_BRAND;
    const parts = brand.split(' ');
    const last = parts.length > 1 ? parts.pop() : null;
    return (
      <span className={`font-black tracking-tighter uppercase ${isPreview ? 'text-2xl' : 'text-lg md:text-xl'}`}>
        <span className="text-white">{parts.join(' ')}</span> {last && <span className="text-[#FA921C]">{last}</span>}
      </span>
    );
  };

  const getVolumeIcon = () => {
    if (audioSettings.isMuted || audioSettings.volume === 0) return 'fa-volume-mute';
    if (audioSettings.volume < 0.3) return 'fa-volume-off';
    if (audioSettings.volume < 0.7) return 'fa-volume-low';
    return 'fa-volume-high';
  };

  return (
    <div className="flex flex-col h-screen font-sans selection:bg-[#FA921C] selection:text-[#051C55]">
      {/* Dynamic Style Injection */}
      <style id="xcurzens-dynamic-styles">{uiSettings.customCss}</style>

      {/* Header */}
      <header className="fixed top-0 left-0 right-0 h-16 bg-[#051C55] text-white flex items-center justify-between px-4 md:px-6 z-50 border-b border-[#FA921C] shadow-lg">
        <div className="flex items-center space-x-3">
          {uiSettings.logoUrl ? (
            <img src={uiSettings.logoUrl} alt="Logo" className="h-10 w-auto object-contain" />
          ) : (
            <div className="w-8 h-8 bg-[#FA921C] rounded-full flex items-center justify-center"><i className="fas fa-radar text-[#051C55] text-sm animate-pulse"></i></div>
          )}
          {renderBrand(uiSettings.businessName)}
        </div>
        
        <div className="flex items-center space-x-2 md:space-x-4">
          {/* Quick Audio Controls */}
          <div className="hidden sm:flex items-center bg-white/5 border border-white/10 rounded-full px-4 py-1.5 space-x-3">
             <button 
              onClick={() => { setAudioSettings(prev => ({ ...prev, autoPlay: !prev.autoPlay })); if(audioSettings.autoPlay) stopCurrentAudio(); }}
              className={`flex items-center space-x-2 px-2 py-0.5 rounded-full transition-all ${audioSettings.autoPlay ? 'bg-[#FA921C]/20 text-[#FA921C]' : 'bg-gray-700 text-gray-400 opacity-50'}`}
              title="Auto-Play Responses"
             >
               <i className="fas fa-robot text-[10px]"></i>
               <span className="text-[9px] font-black uppercase tracking-widest">Auto</span>
             </button>
             <div className="flex items-center space-x-2 border-l border-white/10 pl-3">
                <button onClick={() => setAudioSettings(prev => ({ ...prev, isMuted: !prev.isMuted }))} className={`transition-all hover:scale-110 ${audioSettings.isMuted ? 'text-red-400' : 'text-[#FA921C]'}`}>
                    <i className={`fas ${getVolumeIcon()} text-sm`}></i>
                </button>
                <input type="range" min="0" max="1" step="0.05" value={audioSettings.volume} onChange={(e) => setAudioSettings(prev => ({ ...prev, volume: parseFloat(e.target.value), isMuted: false }))} className="w-16 lg:w-24 h-1 rounded-lg appearance-none cursor-pointer accent-[#FA921C] bg-white/10" />
             </div>
          </div>

          <button onClick={() => { setSettingsForm(uiSettings); setIsSettingsOpen(true); }} className="w-10 h-10 flex items-center justify-center rounded-full bg-white/10 hover:bg-[#FA921C] hover:text-[#051C55] transition-all"><i className="fas fa-sliders"></i></button>
          
          <button onClick={() => { if(window.confirm("PURGE HISTORY?")) setMessages([INITIAL_MESSAGE]); }} className="flex lg:flex items-center space-x-2 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-500 px-3 py-1.5 rounded-full transition-all text-[10px] font-black uppercase tracking-widest">
            <i className="fas fa-trash-can"></i>
            <span className="hidden lg:inline">Purge</span>
          </button>

          <div className="hidden md:flex bg-white/10 px-3 py-1.5 rounded-full text-[10px] font-bold border border-white/20 uppercase tracking-widest">
            <i className="fas fa-location-dot mr-2 text-[#FA921C]"></i>{geoContext.city}
          </div>
        </div>
      </header>

      {/* Message Feed */}
      <main ref={scrollRef} className="flex-1 mt-16 mb-24 overflow-y-auto p-4 md:p-8 space-y-6 bg-[#F0F8FF]">
        <div className="max-w-4xl mx-auto space-y-8 pb-4">
          {messages.map((msg) => (
            <div key={msg.id} className={`flex flex-col ${msg.role === MessageRole.USER ? 'items-end' : 'items-start'} animate-fade-in group`}>
              <div className="flex items-start max-w-full relative">
                {msg.role === MessageRole.SCOUT && (
                   <div className="flex flex-col space-y-2 mr-3 opacity-0 group-hover:opacity-100 transition-opacity sticky top-0">
                      <button onClick={() => { navigator.clipboard.writeText(msg.text.replace(/<[^>]*>/g, '')); setCopiedId(msg.id); setTimeout(() => setCopiedId(null), 2000); }} className={`w-10 h-10 rounded-full flex items-center justify-center border transition-all ${copiedId === msg.id ? 'bg-green-500 border-green-500 text-white' : 'bg-white border-gray-200 text-gray-400 hover:text-[#FA921C] hover:border-[#FA921C]'}`}>
                        <i className={`fas ${copiedId === msg.id ? 'fa-check' : 'fa-copy'}`}></i>
                      </button>
                      <button onClick={() => playScoutAudio(msg.text, msg.id)} className={`w-10 h-10 rounded-full flex items-center justify-center border transition-all ${isVocalizing === msg.id ? 'bg-[#FA921C] border-[#FA921C] text-[#051C55]' : 'bg-white border-gray-200 text-gray-400 hover:text-[#051C55] hover:border-[#051C55]'}`}>
                        <i className={`fas ${isVocalizing === msg.id ? 'fa-stop' : 'fa-volume-up'}`}></i>
                      </button>
                   </div>
                )}
                <div className={`max-w-[85%] md:max-w-[75%] rounded-2xl p-5 shadow-sm transition-all relative ${msg.role === MessageRole.USER ? 'bg-white text-[#051C55] border border-gray-200 rounded-tr-none' : 'bg-[#051C55] text-white rounded-tl-none border-l-4 border-[#FA921C] shadow-lg shadow-[#051C55]/10'}`}>
                  {msg.role === MessageRole.SCOUT && (
                    <div className="mb-3 border-b border-white/10 pb-2">
                      <span className="text-[10px] uppercase tracking-[0.2em] text-[#FA921C] font-black">{uiSettings.businessName}</span>
                    </div>
                  )}
                  {msg.role === MessageRole.USER && msg.id === (messages[messages.length-1].id === msg.id ? msg.id : messages[messages.length-2].id) && (
                    <div className="flex items-center justify-end mb-2">
                      <button onClick={() => setInputValue(msg.text)} className="text-[9px] font-black uppercase text-[#051C55]/40 hover:text-[#FA921C] flex items-center space-x-1"><i className="fas fa-pencil-alt"></i><span>Recycle Turn</span></button>
                    </div>
                  )}
                  <div className={`text-sm md:text-base leading-relaxed break-words prose max-w-none ${msg.role === MessageRole.SCOUT ? 'prose-invert' : 'prose-slate'}`} dangerouslySetInnerHTML={{ __html: marked.parse(msg.text) }} />
                </div>
              </div>
              <div className={`mt-1.5 text-[9px] opacity-40 uppercase tracking-[0.15em] font-black px-1 ${msg.role === MessageRole.USER ? 'mr-1' : 'ml-14'}`}>{new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</div>
            </div>
          ))}
          {isStreaming && (
            <div className="flex flex-col items-start ml-14">
               <div className="bg-[#051C55] text-white rounded-2xl rounded-tl-none border-l-4 border-[#FA921C] p-5 shadow-lg animate-pulse">
                 <div className="flex space-x-2"><div className="w-2 h-2 bg-[#FA921C] rounded-full animate-bounce"></div><div className="w-2 h-2 bg-[#FA921C] rounded-full animate-bounce [animation-delay:-.3s]"></div><div className="w-2 h-2 bg-[#FA921C] rounded-full animate-bounce [animation-delay:-.5s]"></div></div>
               </div>
            </div>
          )}
        </div>
      </main>

      {/* UI Personalization Modal */}
      {isSettingsOpen && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/70 backdrop-blur-md animate-fade-in">
          <div className="bg-[#051C55] border-2 border-[#FA921C] rounded-[2rem] w-full max-w-xl p-8 shadow-2xl relative overflow-y-auto max-h-[90vh]">
            <button onClick={() => setIsSettingsOpen(false)} className="absolute top-6 right-6 text-white/30 hover:text-white"><i className="fas fa-times text-2xl"></i></button>
            <div className="mb-8"><h2 className="text-3xl font-black text-white uppercase tracking-tighter flex items-center"><i className="fas fa-sliders text-[#FA921C] mr-4"></i> CONFIG</h2></div>
            
            <div className="space-y-6">
              {/* Branding Section */}
              <div className="space-y-4">
                <h3 className="text-[#FA921C] text-[10px] font-black uppercase tracking-[0.3em] border-b border-white/10 pb-2">Branding Identity</h3>
                <div className="space-y-2">
                   <label className="block text-white/40 text-[9px] uppercase font-bold">Business Name</label>
                   <input type="text" value={settingsForm.businessName} onChange={(e) => setSettingsForm({ ...settingsForm, businessName: e.target.value })} className="w-full bg-white/5 border border-white/10 rounded-xl py-3 px-4 text-white focus:outline-none focus:border-[#FA921C]" />
                </div>
              </div>

              {/* Audio Module Section */}
              <div className="space-y-4 bg-white/5 p-4 rounded-2xl border border-white/10">
                <h3 className="text-[#FA921C] text-[10px] font-black uppercase tracking-[0.3em] border-b border-white/10 pb-2">Audio Module</h3>
                <div className="flex items-center justify-between">
                   <span className="text-xs text-white">Auto-Vocalize Responses</span>
                   <button onClick={() => setAudioSettings(prev => ({ ...prev, autoPlay: !prev.autoPlay }))} className={`w-12 h-6 rounded-full transition-all relative ${audioSettings.autoPlay ? 'bg-[#FA921C]' : 'bg-white/10'}`}>
                      <div className={`absolute top-1 w-4 h-4 bg-[#051C55] rounded-full transition-all ${audioSettings.autoPlay ? 'left-7' : 'left-1'}`}></div>
                   </button>
                </div>
                <div className="space-y-2">
                   <div className="flex justify-between text-[9px] text-white/40 uppercase font-bold">
                      <span>Master Volume</span>
                      <span>{Math.round(audioSettings.volume * 100)}%</span>
                   </div>
                   <input type="range" min="0" max="1" step="0.05" value={audioSettings.volume} onChange={(e) => setAudioSettings(prev => ({ ...prev, volume: parseFloat(e.target.value), isMuted: false }))} className="w-full h-1 bg-white/10 rounded-lg appearance-none accent-[#FA921C]" />
                </div>
              </div>

              <div className="flex space-x-4 pt-4">
                <button onClick={() => { setUiSettings(settingsForm); setIsSettingsOpen(false); }} className="flex-1 bg-[#FA921C] text-[#051C55] font-black py-4 rounded-xl uppercase tracking-widest hover:bg-[#ffb45e] transition-all">Apply Logic</button>
                <button onClick={() => { if(window.confirm("RESET?")) { setUiSettings({ logoUrl: '', customCss: '', businessName: DEFAULT_BRAND }); setIsSettingsOpen(false); } }} className="text-red-500 text-[10px] font-black uppercase underline decoration-red-500/30">Factory Reset</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Input Terminal */}
      <footer className="fixed bottom-0 left-0 right-0 h-24 bg-[#051C55] border-t border-[#FA921C]/30 px-4 md:px-8 z-50 shadow-[0_-10px_30px_rgba(0,0,0,0.3)]">
        <form onSubmit={handleSend} className="max-w-4xl mx-auto h-full flex flex-col justify-center">
          <div className="flex items-center space-x-3 md:space-x-5">
            <div className="relative flex-1 group">
              <input type="text" value={inputValue} onChange={(e) => setInputValue(e.target.value)} placeholder={isListening ? "Streaming voice..." : "Ask the Scout..."} className="w-full bg-white/5 border border-white/10 rounded-full py-3.5 pl-8 pr-16 text-white placeholder-white/20 focus:outline-none focus:ring-2 focus:ring-[#FA921C]/40 transition-all" disabled={isStreaming} />
              <button type="button" onClick={toggleListening} className={`absolute right-2 top-1/2 -translate-y-1/2 w-11 h-11 rounded-full flex items-center justify-center transition-all ${isListening ? 'bg-[#FA921C] text-[#051C55] scale-110 shadow-[0_0_20px_rgba(250,146,28,0.5)]' : 'bg-white/5 text-white hover:text-[#FA921C]'}`}><i className={`fas ${isListening ? 'fa-waveform animate-pulse' : 'fa-microphone'}`}></i></button>
            </div>
            <button type="submit" disabled={isStreaming || !inputValue.trim()} className="w-12 h-12 md:w-auto md:px-10 bg-[#FA921C] hover:bg-[#ffb45e] disabled:bg-gray-800 disabled:text-gray-500 text-[#051C55] font-black rounded-full transition-all flex items-center justify-center shadow-lg active:scale-95 group">
              <span className="hidden md:inline mr-3 tracking-[0.2em] uppercase text-xs">Transmit</span><i className="fas fa-paper-plane group-hover:rotate-12 transition-transform"></i>
            </button>
          </div>
        </form>
      </footer>

      <style>{`
        .animate-fade-in { animation: fadeIn 0.4s cubic-bezier(0.16, 1, 0.3, 1); }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(15px); } to { opacity: 1; transform: translateY(0); } }
        input[type=range]::-webkit-slider-runnable-track { height: 4px; background: rgba(255,255,255,0.1); border-radius: 2px; }
        input[type=range]::-webkit-slider-thumb { -webkit-appearance: none; height: 14px; width: 14px; border-radius: 50%; background: #FA921C; cursor: pointer; margin-top: -5px; box-shadow: 0 0 10px rgba(250,146,28,0.4); border: 2px solid #051C55; }
      `}</style>
    </div>
  );
};

export default App;