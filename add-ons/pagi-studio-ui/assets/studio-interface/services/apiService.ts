import { ApiResponse, AppSettings, UserProfilePayload } from '../types';
import { GATEWAY_CHAT_URL, GATEWAY_STREAM_URL, API_BASE_URL } from '../src/api/config';

/**
 * Chat/API headers. Sovereignty: the Frontend never sends or receives LLM API keys.
 * The Gateway (Rust) loads OPENROUTER_API_KEY from .env and handles all OpenRouter calls.
 * This keeps the browser a "dumb terminal" and avoids Frontend Drift / leaky abstraction.
 */
function chatHeaders(settings?: AppSettings): Record<string, string> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  // KB-05: Sovereign Security Protocols (optional header for backend toggles)
  if (settings?.sovereignProtocols) {
    headers['X-Sovereign-Protocols'] = 'true';
  }
  return headers;
}

export const sendMessageToOrchestrator = async (
  prompt: string,
  settings: AppSettings,
  projectId?: string | null,
  threadId?: string | null
): Promise<ApiResponse> => {
  try {
    const response = await fetch(GATEWAY_CHAT_URL, {
      method: 'POST',
      headers: chatHeaders(settings),
      body: JSON.stringify({
        prompt,
        stream: settings.stream,
        user_alias: settings.userAlias,
        model: settings.llmModel,
        temperature: settings.llmTemperature,
        max_tokens: settings.llmMaxTokens,
        persona: settings.orchestratorPersona,
        ...(settings.preferredWorkspacePath?.trim() ? { preferred_workspace_path: settings.preferredWorkspacePath.trim() } : {}),
        ...(projectId?.trim() ? { project_id: projectId.trim() } : {}),
        ...(threadId?.trim() ? { thread_id: threadId.trim() } : {}),
      }),
    });

    if (!response.ok) {
      throw new Error(`Backend responded with status: ${response.status}`);
    }

    const data = await response.json();
    
    // Normalize response if the backend returns a simple "thought" string instead of layers
    if (data.thought && !data.thoughts) {
      data.thoughts = [{
        id: 'default-thought',
        title: 'Orchestrator Reasoning',
        content: data.thought,
        expanded: true
      }];
    }

    return data as ApiResponse;
  } catch (error) {
    console.error("API Error:", error);
    throw error;
  }
};

/** POST user profile to KB-01 (Pneuma) for the Discovery loop. Data is stored locally (Bare Metal). */
export const saveUserProfileToKb01 = async (
  payload: UserProfilePayload
): Promise<{ status: string; message?: string; error?: string }> => {
  const res = await fetch(`${API_BASE_URL}/onboarding/user-profile`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    return { status: 'error', error: data?.error ?? `HTTP ${res.status}` };
  }
  return data;
};

/** Mimir Pre-Flight Audio Check result. */
export interface MimirPreflightResult {
  loopback_active: boolean;
  mic_active: boolean;
  detected_devices: { inputs: string[]; outputs: string[] };
  user_advice?: string;
}

/** Mimir start response. */
export interface MimirStartResult {
  meeting_id: string;
  status: string;
}

/** Mimir stop response. */
export interface MimirStopResult {
  meeting_id: string;
  summary_path: string | null;
}

/** Mimir status response. */
export interface MimirStatusResult {
  recording: boolean;
  meeting_id: string | null;
}

export const getMimirPreflight = async (): Promise<MimirPreflightResult> => {
  const res = await fetch(`${API_BASE_URL}/mimir/preflight`);
  if (!res.ok) throw new Error(`Preflight: ${res.status}`);
  return res.json();
};

export const postMimirStart = async (projectId?: string | null): Promise<MimirStartResult> => {
  const res = await fetch(`${API_BASE_URL}/mimir/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_id: projectId ?? undefined }),
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data?.error ?? `Start: ${res.status}`);
  return data;
};

export const postMimirStop = async (): Promise<MimirStopResult> => {
  const res = await fetch(`${API_BASE_URL}/mimir/stop`, { method: 'POST' });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data?.error ?? `Stop: ${res.status}`);
  return data;
};

export const getMimirStatus = async (): Promise<MimirStatusResult> => {
  const res = await fetch(`${API_BASE_URL}/mimir/status`);
  if (!res.ok) return { recording: false, meeting_id: null };
  return res.json();
};

/** Result of POST /api/v1/mimir/synthesize (KB-Linker: meeting summary × infrastructure vitality). */
export interface MimirSynthesizeResult {
  mermaid: string;
  summary: string;
  alerts: Array<{ kind: string; message: string; meeting_mention?: string; current_value: string }>;
  vitality_snapshot: {
    cpu_usage_pct: number;
    ram_used_pct: number;
    ram_summary: string;
    disks: Array<{ name: string; mount_point: string; used_pct: number; total_gb: number; available_gb: number }>;
  };
}

export const postMimirSynthesize = async (
  summaryPath: string,
  projectId?: string | null
): Promise<MimirSynthesizeResult> => {
  const res = await fetch(`${API_BASE_URL}/mimir/synthesize`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      summary_path: summaryPath,
      ...(projectId?.trim() ? { project_id: projectId.trim() } : {}),
    }),
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data?.error ?? `Synthesize: ${res.status}`);
  return data as MimirSynthesizeResult;
};

/** Response from GET /api/v1/vault/protected-terms. With project_id: global, local, merged. Without: terms only. */
export interface VaultProtectedTermsResponse {
  terms: string[];
  scope?: 'global' | 'project';
  global?: string[];
  local?: string[];
  merged?: string[];
  project_id?: string;
}

/** GET /api/v1/vault/protected-terms – SAO protected terms. Optional projectId returns global + local + merged. */
export const getVaultProtectedTerms = async (projectId?: string | null): Promise<VaultProtectedTermsResponse> => {
  const url = projectId?.trim()
    ? `${API_BASE_URL}/vault/protected-terms?project_id=${encodeURIComponent(projectId.trim())}`
    : `${API_BASE_URL}/vault/protected-terms`;
  const res = await fetch(url);
  const data = await res.json().catch(() => ({ terms: [] }));
  if (!res.ok) throw new Error(data?.error ?? `Terms: ${res.status}`);
  return data;
};

/** POST /api/v1/vault/protected-terms – Save SAO protected terms. Global (default) or project scope when projectId set. */
export const postVaultProtectedTerms = async (
  terms: string[],
  projectId?: string | null
): Promise<{ success: boolean; terms: string[]; message?: string; scope?: string }> => {
  const res = await fetch(`${API_BASE_URL}/vault/protected-terms`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ terms, ...(projectId?.trim() ? { project_id: projectId.trim() } : {}) }),
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data?.error ?? `Save terms: ${res.status}`);
  return data;
};

/** POST /api/v1/vault/redact-test – Preview sanitized text. Optional projectId uses merged global + project .sao_policy. */
export const postVaultRedactTest = async (
  text: string,
  projectId?: string | null
): Promise<{ original: string; sanitized: string }> => {
  const res = await fetch(`${API_BASE_URL}/vault/redact-test`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text, ...(projectId?.trim() ? { project_id: projectId.trim() } : {}) }),
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data?.error ?? `Redact test: ${res.status}`);
  return data;
};

/** GET /api/v1/projects/associations – List project–folder associations (for Scope dropdown). */
export interface ProjectAssociationRecord {
  project_id: string;
  local_path: string;
  master_analysis: boolean;
}

export const getProjectAssociations = async (): Promise<ProjectAssociationRecord[]> => {
  const res = await fetch(`${API_BASE_URL}/projects/associations`);
  const data = await res.json().catch(() => ({ associations: [] }));
  return data.associations ?? [];
};

/** GET /api/v1/config/vault/status – Keyring vault status (which keys are stored). */
export const getVaultStatus = async (): Promise<{ openrouter_in_vault: boolean; pagi_llm_in_vault: boolean }> => {
  const res = await fetch(`${API_BASE_URL}/config/vault/status`);
  if (!res.ok) return { openrouter_in_vault: false, pagi_llm_in_vault: false };
  return res.json();
};

/** Milestone Trigger payload from SSE event milestone_suggest. */
export interface MilestoneSuggestPayload {
  project_id: string;
  message: string;
}

/** Chunk from stream: content string, expert-routing meta (MoE), milestone-suggest, or status (Strategic Timing Phase 2). */
export type StreamChunk =
  | string
  | { __expertRouting: string }
  | { __milestoneSuggest: MilestoneSuggestPayload }
  | { __streamingStatus: string };

export const streamMessageToOrchestrator = async function* (
  prompt: string,
  settings: AppSettings,
  projectId?: string | null,
  threadId?: string | null
): AsyncGenerator<StreamChunk, void, unknown> {
  try {
    // Use SSE endpoint when streaming so we receive milestone_suggest and other events.
    const response = await fetch(GATEWAY_STREAM_URL, {
      method: 'POST',
      headers: chatHeaders(settings),
      body: JSON.stringify({
        prompt,
        stream: true,
        user_alias: settings.userAlias,
        model: settings.llmModel,
        temperature: settings.llmTemperature,
        max_tokens: settings.llmMaxTokens,
        persona: settings.orchestratorPersona,
        ...(settings.preferredWorkspacePath?.trim() ? { preferred_workspace_path: settings.preferredWorkspacePath.trim() } : {}),
        ...(projectId?.trim() ? { project_id: projectId.trim() } : {}),
        ...(threadId?.trim() ? { thread_id: threadId.trim() } : {}),
      }),
    });

    if (!response.ok) {
      throw new Error(`Backend responded with status: ${response.status}`);
    }

    if (!response.body) {
      throw new Error("Response body is not readable");
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    const contentType = response.headers.get('content-type') ?? '';
    const isSSE = contentType.includes('text/event-stream');

    try {
      if (isSSE) {
        let buffer = '';
        let lastEvent = '';
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split(/\r?\n/);
          buffer = lines.pop() ?? '';

          for (const line of lines) {
            const trimmed = line.trimEnd();
            if (!trimmed || trimmed.startsWith(':')) continue;
            if (trimmed.startsWith('event:')) {
              lastEvent = trimmed.replace(/^event:\s?/, '').trim();
              continue;
            }
            if (trimmed.startsWith('data:')) {
              const data = trimmed.replace(/^data:\s?/, '');
              if (lastEvent === 'milestone_suggest') {
                try {
                  const payload = JSON.parse(data) as MilestoneSuggestPayload;
                  yield { __milestoneSuggest: payload };
                } catch {
                  /* ignore */
                }
                lastEvent = '';
                continue;
              }
              if (lastEvent === 'status') {
                yield { __streamingStatus: data };
                lastEvent = '';
                continue;
              }
              if (lastEvent === 'token' || lastEvent === '') {
                if (data.startsWith('{') || data.startsWith('[')) {
                  try {
                    const parsed: any = JSON.parse(data);
                    const content =
                      typeof parsed === 'string'
                        ? parsed
                        : (parsed?.content ?? parsed?.token ?? parsed?.text ?? data);
                    yield String(content);
                  } catch {
                    yield data;
                  }
                } else {
                  yield data;
                }
              }
              lastEvent = '';
            }
          }
        }
        const tail = (buffer ?? '').trim();
        if (tail.startsWith('data:')) {
          yield tail.replace(/^data:\s?/, '');
        }
      } else {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          const chunk = decoder.decode(value, { stream: true });
          if (chunk) yield chunk;
        }
      }
    } finally {
      reader.releaseLock();
    }
  } catch (error) {
    console.error("Stream API Error:", error);
    throw error;
  }
};
