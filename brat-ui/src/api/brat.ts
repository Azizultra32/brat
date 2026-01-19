import type {
  Repo,
  StatusOutput,
  Convoy,
  Task,
  Session,
  MayorStatus,
  MayorAskResponse,
  MayorHistoryResponse,
  SessionLogsResponse,
  CreateConvoyRequest,
  CreateTaskRequest,
  UpdateTaskRequest,
} from '../types/brat';

// Use relative URL for dev server proxy, or absolute URL for production
const API_BASE = import.meta.env.VITE_API_BASE || '/api/v1';

// Helper function for API requests
async function apiRequest<T>(
  url: string,
  options?: RequestInit
): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || `HTTP error ${response.status}`);
  }

  return response.json();
}

export const bratApi = {
  // Health check
  async health(): Promise<{ status: string; version: string; uptime_secs: number }> {
    return apiRequest(`${API_BASE}/health`);
  },

  // Repository management
  async listRepos(): Promise<Repo[]> {
    return apiRequest(`${API_BASE}/repos`);
  },

  async registerRepo(path: string): Promise<{ success: boolean; repo?: Repo; error?: string }> {
    return apiRequest(`${API_BASE}/repos`, {
      method: 'POST',
      body: JSON.stringify({ path }),
    });
  },

  async unregisterRepo(repoId: string): Promise<void> {
    await fetch(`${API_BASE}/repos/${repoId}`, { method: 'DELETE' });
  },

  // Repository status
  async getStatus(repoId: string): Promise<StatusOutput> {
    return apiRequest(`${API_BASE}/repos/${repoId}/status`);
  },

  // Convoy management
  async listConvoys(repoId: string): Promise<Convoy[]> {
    return apiRequest(`${API_BASE}/repos/${repoId}/convoys`);
  },

  async getConvoy(repoId: string, convoyId: string): Promise<Convoy> {
    return apiRequest(`${API_BASE}/repos/${repoId}/convoys/${convoyId}`);
  },

  async createConvoy(repoId: string, data: CreateConvoyRequest): Promise<Convoy> {
    return apiRequest(`${API_BASE}/repos/${repoId}/convoys`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  // Task management
  async listTasks(
    repoId: string,
    filters?: { convoy?: string; status?: string }
  ): Promise<Task[]> {
    const params = new URLSearchParams();
    if (filters?.convoy) params.set('convoy', filters.convoy);
    if (filters?.status) params.set('status', filters.status);
    const query = params.toString();
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks${query ? `?${query}` : ''}`);
  },

  async getTask(repoId: string, taskId: string): Promise<Task> {
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks/${taskId}`);
  },

  async createTask(repoId: string, data: CreateTaskRequest): Promise<Task> {
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  async updateTask(repoId: string, taskId: string, data: UpdateTaskRequest): Promise<Task> {
    return apiRequest(`${API_BASE}/repos/${repoId}/tasks/${taskId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
  },

  // Session management
  async listSessions(repoId: string, taskId?: string): Promise<Session[]> {
    const query = taskId ? `?task=${taskId}` : '';
    return apiRequest(`${API_BASE}/repos/${repoId}/sessions${query}`);
  },

  async getSession(repoId: string, sessionId: string): Promise<Session> {
    return apiRequest(`${API_BASE}/repos/${repoId}/sessions/${sessionId}`);
  },

  async stopSession(repoId: string, sessionId: string, reason?: string): Promise<void> {
    await fetch(`${API_BASE}/repos/${repoId}/sessions/${sessionId}/stop`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: reason || 'ui-stop' }),
    });
  },

  // Session logs
  async getSessionLogs(
    repoId: string,
    sessionId: string,
    lines: number = 100
  ): Promise<SessionLogsResponse> {
    return apiRequest(`${API_BASE}/repos/${repoId}/sessions/${sessionId}/logs?lines=${lines}`);
  },

  // Mayor management
  async getMayorStatus(repoId: string): Promise<MayorStatus> {
    return apiRequest(`${API_BASE}/repos/${repoId}/mayor/status`);
  },

  async startMayor(
    repoId: string,
    message?: string
  ): Promise<{ session_id: string; response: string[] }> {
    return apiRequest(`${API_BASE}/repos/${repoId}/mayor/start`, {
      method: 'POST',
      body: JSON.stringify({ message }),
    });
  },

  async stopMayor(repoId: string): Promise<{ success: boolean }> {
    return apiRequest(`${API_BASE}/repos/${repoId}/mayor/stop`, {
      method: 'POST',
    });
  },

  async askMayor(repoId: string, message: string): Promise<MayorAskResponse> {
    return apiRequest(`${API_BASE}/repos/${repoId}/mayor/ask`, {
      method: 'POST',
      body: JSON.stringify({ message }),
    });
  },

  async getMayorHistory(repoId: string, lines: number = 50): Promise<MayorHistoryResponse> {
    return apiRequest(`${API_BASE}/repos/${repoId}/mayor/history?lines=${lines}`);
  },
};

export default bratApi;
