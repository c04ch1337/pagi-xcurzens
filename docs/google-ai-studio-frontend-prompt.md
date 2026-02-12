# Google AI Studio Prompt: Phoenix Marie Chat-First Frontend UI

## Project Overview

Create a **professional, sleek, and modern chat-first frontend UI** for Phoenix Marie, a Sovereign AGI Orchestrator. The interface should prioritize conversational interaction while maintaining clean, uncluttered pages with intuitive navigation.

---

## Design Philosophy

### Core Principles
1. **Chat-First/Default**: The primary interface is a conversational chat window that opens immediately
2. **Professional & Sleek**: Modern, minimalist design with sophisticated color palette
3. **Clean & Uncluttered**: Each page focuses on essential information without overwhelming the user
4. **Responsive**: Works seamlessly across desktop, tablet, and mobile devices
5. **Accessibility**: WCAG 2.1 AA compliant with proper contrast ratios and keyboard navigation

### Visual Style
- **Color Scheme**: Dark theme with deep blues and purples (similar to existing: `--bg: #0b1020`, `--card: #121a33`)
- **Typography**: Clean sans-serif fonts (system-ui, -apple-system, Segoe UI, Roboto)
- **Spacing**: Generous whitespace, consistent padding/margins
- **Animations**: Subtle, purposeful transitions (200-300ms)
- **Borders**: Rounded corners (8-12px), subtle borders with low opacity

---

## Technical Architecture

### Frontend Stack
- **Framework**: React 18+ with TypeScript
- **Build Tool**: Vite
- **Styling**: CSS Modules or Tailwind CSS (your choice)
- **State Management**: React Context API or Zustand (lightweight)
- **HTTP Client**: Fetch API with custom wrapper
- **Real-time**: EventSource (Server-Sent Events) for streaming

### Backend Integration
- **Base URL**: `http://127.0.0.1:8001/api/v1`
- **Gateway Origin**: `http://127.0.0.1:8001`
- **Dev Server Port**: 3001 (Vite default)

---

## API Integration Requirements

### Essential Endpoints

#### 1. Health & Status
```typescript
// GET /api/v1/health
// Response: { "status": "ok" }

// GET /v1/status
// Response: {
//   "app_name": "UAC Gateway",
//   "port": 8001,
//   "llm_mode": "mock",
//   "slot_labels": {
//     "1": "Brand Voice",
//     "2": "Sales",
//     // ... up to 8 slots
//   }
// }
```

#### 2. Chat (Primary Interface)
```typescript
// POST /api/v1/chat
interface ChatRequest {
  prompt: string;
  stream: boolean;
  user_alias?: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  persona?: string;
}

// Non-streaming response:
interface ChatResponse {
  response: string;
  thought?: string;
  status: "ok" | "error" | "policy_violation";
  error?: string;
}

// Streaming response: Content-Type: text/plain; charset=utf-8
// Raw text chunks (not SSE format)
```

#### 3. Configuration
```typescript
// GET /api/v1/config
interface GatewayFeatureConfig {
  persona_mode: "counselor" | "companion";
  llm_model: string;
  moe_enabled: boolean;
  // ... other feature flags
}
```

#### 4. Persona Stream (SSE)
```typescript
// GET /api/v1/persona/stream
// EventSource connection for real-time updates

// Event types:
// 1. persona_heartbeat (every 4 hours)
{
  type: "persona_heartbeat",
  message: "Check-in message"
}

// 2. sentinel_update (velocity monitoring)
{
  type: "sentinel_update",
  velocity_score: 0-100,
  is_rage_detected: boolean
}

// 3. sovereign_reset_suggested
{
  type: "sovereign_reset_suggested",
  message?: string,
  health_reminder?: string
}
```

#### 5. Knowledge Base Status
```typescript
// GET /api/v1/kb-status
// Returns status of all 9 Knowledge Bases
interface KBStatus {
  [key: string]: {
    label: string;
    status: "active" | "inactive";
    record_count?: number;
  }
}
```

#### 6. Wellness & Balance
```typescript
// POST /api/v1/soma/balance
interface BalanceRequest {
  spirit: number; // 1-10
  mind: number;   // 1-10
  body: number;   // 1-10
}

// GET /api/v1/skills/wellness-report
interface WellnessReport {
  pillars: {
    spirit: number;
    mind: number;
    body: number;
  };
  individuation_score: number;
  summary: string;
  is_critical: boolean;
  flags: string[];
  entries_used: number;
}
```

#### 7. Logs (SSE)
```typescript
// GET /api/v1/logs
// EventSource connection for gateway logs
// Real-time log streaming for debugging
```

#### 8. Settings
```typescript
// GET/POST /api/v1/settings/persona
// Body: { "mode": "counselor" | "companion" }

// GET/POST /api/v1/settings/moe
// Body: { "enabled": boolean }
```

---

## UI Components & Layout

### 1. Main Layout (Chat-First)

```
┌─────────────────────────────────────────────────────────┐
│  [Logo] Phoenix Marie              [Settings] [Profile] │ Header
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌───────────────────────────────────────────────────┐ │
│  │                                                   │ │
│  │  Chat Messages Area                              │ │
│  │  (Scrollable, auto-scroll to bottom)             │ │
│  │                                                   │ │
│  │  ┌─────────────────────────────────────────┐    │ │
│  │  │ User: Hello, how are you?               │    │ │
│  │  └─────────────────────────────────────────┘    │ │
│  │                                                   │ │
│  │    ┌─────────────────────────────────────────┐  │ │
│  │    │ Phoenix: I'm doing well, thank you!     │  │ │
│  │    └─────────────────────────────────────────┘  │ │
│  │                                                   │ │
│  └───────────────────────────────────────────────────┘ │
│                                                         │
│  ┌───────────────────────────────────────────────────┐ │
│  │ [Type your message...]              [Send] [🎤]  │ │ Input
│  └───────────────────────────────────────────────────┘ │
│                                                         │
│  [💬 Chat] [📊 Wellness] [🧠 Knowledge] [⚙️ Settings]  │ Bottom Nav
└─────────────────────────────────────────────────────────┘
```

### 2. Chat Interface (Default View)

**Features:**
- **Message Bubbles**: User messages on right (blue), AI on left (gray/purple)
- **Timestamps**: Subtle, below each message
- **Typing Indicator**: Animated dots when AI is responding
- **Streaming**: Real-time token display as AI generates response
- **Message Actions**: Copy, regenerate, pin (hover to reveal)
- **Context Indicators**: Show when Kardia (relationship) context is active
- **Thought Display**: Optional toggle to show AI's internal reasoning

**Input Area:**
- **Multi-line Support**: Auto-expand textarea (max 5 lines)
- **Send Button**: Disabled when empty, enabled when text present
- **Voice Input**: Optional microphone button for future voice integration
- **Keyboard Shortcuts**: Enter to send, Shift+Enter for new line

### 3. Wellness Tab

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│  Wellness Report                          [Refresh]     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Individuation Score: 7.2/10                    │   │
│  │  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Spirit     │  │     Mind     │  │     Body     │ │
│  │     8.5      │  │     6.3      │  │     7.1      │ │
│  │   ━━━━━━━━   │  │   ━━━━━━━━   │  │   ━━━━━━━━   │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                         │
│  Summary:                                               │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Your spiritual practice is strong, but mental   │   │
│  │ clarity could use attention. Consider...        │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Flags: [Shadow Dominance] [Puer Aeternus]            │
│                                                         │
│  [Check In Now]                                         │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- **Pillar Visualization**: Three cards showing Spirit/Mind/Body scores (7-day average)
- **Individuation Score**: Large progress bar with numeric value
- **Summary Text**: AI-generated wellness insights
- **Flags**: Badges for psychological patterns (e.g., "Shadow Dominance")
- **Critical Warning**: Red banner if `is_critical` is true
- **Check-In Button**: Opens Balance Check Modal

### 4. Balance Check Modal

**Triggered by:**
- Persona heartbeat (every 4 hours via SSE)
- Manual "Check In Now" button
- Sovereign reset suggestion

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│  How are you feeling?                          [×]      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Spirit (1-10)                                          │
│  ○ ○ ○ ○ ○ ○ ○ ● ○ ○                                   │
│                                                         │
│  Mind (1-10)                                            │
│  ○ ○ ○ ○ ○ ● ○ ○ ○ ○                                   │
│                                                         │
│  Body (1-10)                                            │
│  ○ ○ ○ ○ ○ ○ ● ○ ○ ○                                   │
│                                                         │
│                    [Submit]                             │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- **Interactive Sliders**: Click or drag to select 1-10 for each pillar
- **Visual Feedback**: Selected value highlights, shows number
- **Submit**: POST to `/api/v1/soma/balance`
- **Dismissible**: Can close without submitting (but encouraged to complete)

### 5. Knowledge Base Panel

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│  Knowledge Bases                          [Refresh]     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │ 🔒 KB-01: Ethos (Core Identity)      [ACTIVE]  │   │
│  │    Records: 42 | Last updated: 2h ago          │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │ 📚 KB-02: Technical (Rust docs)      [ACTIVE]  │   │
│  │    Records: 1,234 | Last updated: 5m ago       │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │ 💬 KB-03: Persona (Tone)             [ACTIVE]  │   │
│  │    Records: 89 | Last updated: 1d ago          │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  ... (continue for all 9 KBs)                           │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- **Status Indicators**: Green (active), Gray (inactive), Red (error)
- **Firewall Badges**: 🔒 for CORE ONLY, ⚠️ for RESTRICTED
- **Record Counts**: Show number of entries in each KB
- **Last Updated**: Relative timestamps
- **Expandable**: Click to see more details (future enhancement)

### 6. Settings Panel

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│  Settings                                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Persona Mode                                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │ ○ Counselor (Sage, supportive)                 │   │
│  │ ● Companion (Warm, conversational)             │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Intelligence Layer                                     │
│  ┌─────────────────────────────────────────────────┐   │
│  │ [✓] Enable Mixture of Experts (MoE)            │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Orchestrator Endpoint                                  │
│  ┌─────────────────────────────────────────────────┐   │
│  │ http://127.0.0.1:8001/api/v1/chat              │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Archetype (Optional)                                   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Birth Sign: [Aquarius ▼]                       │   │
│  │ Ascendant: [Leo ▼]                             │   │
│  │ Jungian Shadow: [Anima ▼]                      │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  [Save Changes]                                         │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- **Persona Toggle**: Radio buttons for Counselor/Companion mode
- **MoE Toggle**: Checkbox for enabling/disabling Mixture of Experts
- **Endpoint Config**: Editable text field for API base URL
- **Archetype Settings**: Dropdowns for astrological/Jungian preferences
- **Save Button**: Persists changes to backend

### 7. Warden Sidebar (Optional, Collapsible)

**Layout:**
```
┌─────────────────┐
│  Sentinel       │
│  Status: CALM   │
│                 │
│  Velocity       │
│  ━━━━━━━━━━━━━  │
│  35/100         │
│                 │
│  Domain         │
│  Integrity      │
│  ✓ No alerts    │
│                 │
└─────────────────┘
```

**Features:**
- **Sentinel Badge**: Color-coded (green=calm, yellow=high, red=rage)
- **Velocity Bar**: Visual progress bar showing 0-100 score
- **Domain Integrity**: Shows absurdity log count and resource drain alerts
- **Auto-update**: Listens to `/api/v1/persona/stream` for sentinel_update events

### 8. Log Terminal (Developer Mode)

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│  Gateway Logs                    [Clear] [Pause] [×]    │
├─────────────────────────────────────────────────────────┤
│  [2026-02-09 19:26:45] INFO: Gateway started on 8001   │
│  [2026-02-09 19:26:46] DEBUG: KB-01 loaded (42 records)│
│  [2026-02-09 19:26:47] INFO: Chat request from user_123│
│  [2026-02-09 19:26:48] DEBUG: ModelRouter invoked      │
│  ...                                                    │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- **Real-time Streaming**: EventSource connection to `/api/v1/logs`
- **Auto-scroll**: Follows new logs as they arrive
- **Pause/Resume**: Stop auto-scroll to review logs
- **Clear**: Wipe current log buffer
- **Monospace Font**: For readability
- **Color Coding**: INFO (white), DEBUG (gray), WARN (yellow), ERROR (red)

---

## Page Structure (Keep Clean & Uncluttered)

### Navigation Pattern
- **Bottom Tab Bar** (Mobile-first, works on desktop too)
  - 💬 Chat (default)
  - 📊 Wellness
  - 🧠 Knowledge
  - ⚙️ Settings
- **Top Header** (Minimal)
  - Logo/Title on left
  - Status indicator (connection state)
  - Profile/Settings icon on right

### Page Transitions
- **Smooth Fade**: 200ms opacity transition between views
- **Preserve State**: Chat history persists when switching tabs
- **Loading States**: Skeleton screens, not spinners

---

## Responsive Design

### Breakpoints
- **Mobile**: < 768px (single column, bottom nav)
- **Tablet**: 768px - 1024px (two columns, side nav optional)
- **Desktop**: > 1024px (multi-column, sidebars visible)

### Mobile Optimizations
- **Touch Targets**: Minimum 44x44px for buttons
- **Swipe Gestures**: Swipe left/right to switch tabs
- **Collapsible Sections**: Accordion-style for KB list
- **Bottom Sheet**: Modals slide up from bottom on mobile

---

## Accessibility

### WCAG 2.1 AA Compliance
- **Contrast Ratios**: 4.5:1 for normal text, 3:1 for large text
- **Keyboard Navigation**: Tab order, focus indicators, Escape to close modals
- **Screen Readers**: ARIA labels, semantic HTML, live regions for chat
- **Focus Management**: Trap focus in modals, return focus on close
- **Skip Links**: "Skip to main content" for keyboard users

### Color Blindness
- **Don't Rely on Color Alone**: Use icons + text for status
- **High Contrast Mode**: Support Windows/macOS high contrast themes

---

## Performance Considerations

### Optimization Strategies
- **Code Splitting**: Lazy load Settings, Wellness, Knowledge tabs
- **Virtual Scrolling**: For long chat histories (react-window)
- **Debounce**: Input fields (300ms) to reduce API calls
- **Memoization**: React.memo for message components
- **Image Optimization**: WebP format, lazy loading
- **Bundle Size**: Keep initial bundle < 200KB gzipped

### Caching
- **Service Worker**: Cache static assets (optional PWA)
- **LocalStorage**: Persist user preferences, draft messages
- **Session Storage**: Temporary chat state

---

## Error Handling

### User-Friendly Messages
- **Connection Error**: "Unable to reach Phoenix Marie. Check your connection."
- **Policy Violation**: "This request violates safety policies. Please rephrase."
- **Rate Limit**: "Too many requests. Please wait a moment."
- **Server Error**: "Something went wrong. We're looking into it."

### Retry Logic
- **Exponential Backoff**: 1s, 2s, 4s, 8s for failed requests
- **Max Retries**: 3 attempts before showing error
- **Manual Retry**: Button to retry failed requests

### Offline Support
- **Offline Indicator**: Banner at top when disconnected
- **Queue Messages**: Store unsent messages, send when reconnected
- **Graceful Degradation**: Show cached data when offline

---

## Security Considerations

### API Key Management
- **Environment Variables**: Store `PAGI_API_KEY` securely
- **Never Log Keys**: Redact sensitive data in logs
- **HTTPS Only**: Enforce secure connections in production

### Input Sanitization
- **XSS Prevention**: Escape user input before rendering
- **SQL Injection**: Backend handles, but validate on frontend too
- **CSRF Protection**: Use tokens for state-changing requests

### Shadow Key (Encrypted Journals)
- **Secure Storage**: Never store in localStorage without encryption
- **Prompt on Access**: Ask for key when viewing encrypted entries
- **Session-only**: Clear from memory on logout/close

---

## Implementation Checklist

### Phase 1: Core Chat (MVP)
- [ ] Set up Vite + React + TypeScript project
- [ ] Create API service layer (`/src/services/api.ts`)
- [ ] Implement chat interface with message bubbles
- [ ] Add streaming support (plain text chunks)
- [ ] Connect to `/api/v1/chat` endpoint
- [ ] Add loading/error states
- [ ] Implement responsive layout (mobile-first)

### Phase 2: Essential Features
- [ ] Add bottom navigation (Chat, Wellness, Knowledge, Settings)
- [ ] Implement Settings panel (persona mode, MoE toggle)
- [ ] Add health check (`/api/v1/health`)
- [ ] Fetch and display gateway config (`/api/v1/config`)
- [ ] Add connection status indicator
- [ ] Implement keyboard shortcuts (Enter to send)

### Phase 3: Wellness & Balance
- [ ] Create Balance Check Modal (Spirit/Mind/Body sliders)
- [ ] Connect to `/api/v1/soma/balance` endpoint
- [ ] Implement Wellness tab with pillar visualization
- [ ] Fetch wellness report (`/api/v1/skills/wellness-report`)
- [ ] Add individuation score display
- [ ] Show flags and critical warnings

### Phase 4: Real-time Features
- [ ] Set up EventSource for `/api/v1/persona/stream`
- [ ] Handle `persona_heartbeat` events (open Balance Modal)
- [ ] Handle `sentinel_update` events (update Warden sidebar)
- [ ] Handle `sovereign_reset_suggested` events (show toast)
- [ ] Add Warden sidebar (collapsible)
- [ ] Implement velocity score visualization

### Phase 5: Knowledge Base
- [ ] Create Knowledge Base panel
- [ ] Fetch KB status (`/api/v1/kb-status`)
- [ ] Display 9 KBs with labels from `/v1/status`
- [ ] Show firewall status (CORE ONLY, RESTRICTED)
- [ ] Add record counts and last updated timestamps
- [ ] Implement refresh functionality

### Phase 6: Advanced Features
- [ ] Add Log Terminal (developer mode)
- [ ] Connect to `/api/v1/logs` SSE stream
- [ ] Implement log filtering and search
- [ ] Add message actions (copy, regenerate, pin)
- [ ] Implement thought display toggle
- [ ] Add voice input button (placeholder for future)

### Phase 7: Polish & Optimization
- [ ] Add smooth page transitions
- [ ] Implement skeleton loading states
- [ ] Add toast notifications for events
- [ ] Optimize bundle size (code splitting)
- [ ] Add virtual scrolling for long chats
- [ ] Implement offline support
- [ ] Add PWA manifest (optional)

### Phase 8: Testing & Accessibility
- [ ] Test keyboard navigation
- [ ] Verify WCAG 2.1 AA compliance
- [ ] Test with screen readers
- [ ] Test on mobile devices
- [ ] Test error scenarios
- [ ] Test streaming edge cases
- [ ] Performance audit (Lighthouse)

---

## File Structure

```
pagi-frontend-v2/
├── public/
│   ├── favicon.ico
│   └── manifest.json
├── src/
│   ├── components/
│   │   ├── Chat/
│   │   │   ├── ChatInterface.tsx
│   │   │   ├── MessageBubble.tsx
│   │   │   ├── InputArea.tsx
│   │   │   └── TypingIndicator.tsx
│   │   ├── Wellness/
│   │   │   ├── WellnessTab.tsx
│   │   │   ├── BalanceCheckModal.tsx
│   │   │   ├── PillarCard.tsx
│   │   │   └── IndividuationScore.tsx
│   │   ├── Knowledge/
│   │   │   ├── KnowledgePanel.tsx
│   │   │   └── KBCard.tsx
│   │   ├── Settings/
│   │   │   ├── SettingsPanel.tsx
│   │   │   └── PersonaToggle.tsx
│   │   ├── Warden/
│   │   │   ├── WardenSidebar.tsx
│   │   │   └── VelocityBar.tsx
│   │   ├── Logs/
│   │   │   └── LogTerminal.tsx
│   │   ├── Layout/
│   │   │   ├── Header.tsx
│   │   │   ├── BottomNav.tsx
│   │   │   └── MainLayout.tsx
│   │   └── Common/
│   │       ├── Button.tsx
│   │       ├── Modal.tsx
│   │       ├── Toast.tsx
│   │       └── LoadingSpinner.tsx
│   ├── services/
│   │   ├── api.ts
│   │   ├── sse.ts
│   │   └── storage.ts
│   ├── hooks/
│   │   ├── useChat.ts
│   │   ├── usePersonaStream.ts
│   │   ├── useWellness.ts
│   │   └── useKnowledgeBase.ts
│   ├── types/
│   │   ├── api.ts
│   │   ├── chat.ts
│   │   └── wellness.ts
│   ├── utils/
│   │   ├── formatters.ts
│   │   ├── validators.ts
│   │   └── constants.ts
│   ├── styles/
│   │   ├── globals.css
│   │   ├── variables.css
│   │   └── themes.css
│   ├── App.tsx
│   ├── main.tsx
│   └── vite-env.d.ts
├── .env.example
├── .gitignore
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

---

## Example Code Snippets

### API Service Layer

```typescript
// src/services/api.ts
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://127.0.0.1:8001/api/v1';

export interface ChatRequest {
  prompt: string;
  stream: boolean;
  user_alias?: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  persona?: string;
}

export interface ChatResponse {
  response: string;
  thought?: string;
  status: 'ok' | 'error' | 'policy_violation';
  error?: string;
}

export async function sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
  const response = await fetch(`${API_BASE_URL}/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }

  return response.json();
}

export async function* streamChatMessage(request: ChatRequest): AsyncGenerator<string> {
  const response = await fetch(`${API_BASE_URL}/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...request, stream: true }),
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }

  const reader = response.body?.getReader();
  const decoder = new TextDecoder();

  if (!reader) throw new Error('No response body');

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    yield decoder.decode(value, { stream: true });
  }
}

export async function getHealth(): Promise<{ status: string }> {
  const response = await fetch(`${API_BASE_URL}/health`);
  return response.json();
}

export async function getConfig(): Promise<GatewayFeatureConfig> {
  const response = await fetch(`${API_BASE_URL}/config`);
  return response.json();
}

export async function getKBStatus(): Promise<Record<string, any>> {
  const response = await fetch(`${API_BASE_URL}/kb-status`);
  return response.json();
}

export async function submitBalance(balance: { spirit: number; mind: number; body: number }): Promise<void> {
  await fetch(`${API_BASE_URL}/soma/balance`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(balance),
  });
}

export async function getWellnessReport(): Promise<WellnessReport> {
  const response = await fetch(`${API_BASE_URL}/skills/wellness-report`);
  return response.json();
}
```

### Persona Stream Hook

```typescript
// src/hooks/usePersonaStream.ts
import { useEffect, useState } from 'react';

interface PersonaEvent {
  type: 'persona_heartbeat' | 'sentinel_update' | 'sovereign_reset_suggested';
  message?: string;
  velocity_score?: number;
  is_rage_detected?: boolean;
  health_reminder?: string;
}

export function usePersonaStream(onEvent: (event: PersonaEvent) => void) {
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const eventSource = new EventSource(`${API_BASE_URL}/persona/stream`);

    eventSource.onopen = () => setConnected(true);
    eventSource.onerror = () => setConnected(false);

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as PersonaEvent;
        onEvent(data);
      } catch (error) {
        console.error('Failed to parse persona event:', error);
      }
    };

    return () => {
      eventSource.close();
      setConnected(false);
    };
  }, [onEvent]);

  return { connected };
}
```

### Chat Interface Component

```typescript
// src/components/Chat/ChatInterface.tsx
import React, { useState, useRef, useEffect } from 'react';
import { sendChatMessage, streamChatMessage } from '../../services/api';
import MessageBubble from './MessageBubble';
import InputArea from './InputArea';
import TypingIndicator from './TypingIndicator';

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  thought?: string;
}

export default function ChatInterface() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, streamingContent]);

  const handleSendMessage = async (content: string) => {
    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      content,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setIsLoading(true);
    setStreamingContent('');

    try {
      // Streaming mode
      let fullResponse = '';
      for await (const chunk of streamChatMessage({
        prompt: content,
        stream: true,
        user_alias: 'default_user',
      })) {
        fullResponse += chunk;
        setStreamingContent(fullResponse);
      }

      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: fullResponse,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);
      setStreamingContent('');
    } catch (error) {
      console.error('Chat error:', error);
      // Show error message
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="chat-interface">
      <div className="messages-container">
        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
        {streamingContent && (
          <MessageBubble
            message={{
              id: 'streaming',
              role: 'assistant',
              content: streamingContent,
              timestamp: new Date(),
            }}
          />
        )}
        {isLoading && !streamingContent && <TypingIndicator />}
        <div ref={messagesEndRef} />
      </div>
      <InputArea onSend={handleSendMessage} disabled={isLoading} />
    </div>
  );
}
```

---

## Color Palette

### Dark Theme (Default)
```css
:root {
  /* Background */
  --bg-primary: #0b1020;
  --bg-secondary: #121a33;
  --bg-tertiary: #1a2444;
  
  /* Text */
  --text-primary: #e7eaf3;
  --text-secondary: #aab3d1;
  --text-muted: #6b7599;
  
  /* Borders */
  --border-primary: #263158;
  --border-secondary: #1e2847;
  
  /* Accent Colors */
  --accent-blue: #4a90e2;
  --accent-purple: #8b5cf6;
  --accent-green: #10b981;
  --accent-amber: #f59e0b;
  --accent-red: #ef4444;
  
  /* Persona Modes */
  --counselor-accent: #10b981; /* Sage green */
  --companion-accent: #f59e0b; /* Warm amber */
  
  /* Sentinel Status */
  --sentinel-calm: #10b981;
  --sentinel-high: #f59e0b;
  --sentinel-rage: #ef4444;
  
  /* Shadows */
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.4);
  --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.5);
}
```

---

## Final Notes

### Design Inspiration
- **Linear**: Clean, minimal, fast
- **Notion**: Organized, intuitive navigation
- **ChatGPT**: Conversational, streaming responses
- **Vercel**: Modern, professional aesthetic

### Key Differentiators
1. **Chat-First**: Unlike typical dashboards, chat is the primary interface
2. **Wellness Integration**: Unique Spirit/Mind/Body tracking
3. **Sovereign AI**: Emphasis on user autonomy and privacy
4. **Real-time Awareness**: Sentinel monitoring and persona heartbeats

### Success Criteria
- [ ] User can start chatting within 3 seconds of page load
- [ ] No page feels cluttered or overwhelming
- [ ] All interactions feel smooth and responsive (< 100ms feedback)
- [ ] Mobile experience is as good as desktop
- [ ] Accessibility score > 95 (Lighthouse)
- [ ] Bundle size < 200KB gzipped

---

## Deliverables

Please provide:

1. **Complete React + TypeScript + Vite project** with all components
2. **API service layer** with full integration to Phoenix Marie backend
3. **Responsive CSS** (or Tailwind config) for all breakpoints
4. **README.md** with setup instructions and architecture overview
5. **Environment variables** template (`.env.example`)
6. **Type definitions** for all API responses and component props
7. **Error handling** for all API calls and edge cases
8. **Accessibility features** (ARIA labels, keyboard nav, focus management)

---

## Questions to Consider

Before starting implementation, please confirm:

1. **Styling approach**: CSS Modules, Tailwind, or styled-components?
2. **State management**: Context API, Zustand, or Redux Toolkit?
3. **Testing**: Jest + React Testing Library?
4. **Linting**: ESLint + Prettier config?
5. **Deployment target**: Static hosting (Vercel, Netlify) or self-hosted?

---

**End of Prompt**

This prompt provides comprehensive guidance for building a professional, chat-first frontend UI for Phoenix Marie. The design prioritizes clean, uncluttered pages with intuitive navigation while maintaining full integration with the existing backend API.
