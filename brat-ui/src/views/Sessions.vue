<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import bratApi from '../api/brat';
import type { Session } from '../types/brat';
import StatusBadge from '../components/common/StatusBadge.vue';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();
const sessions = ref<Session[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

// Log viewer state
const selectedSession = ref<Session | null>(null);
const logs = ref<string[]>([]);
const logsLoading = ref(false);
const autoScroll = ref(true);

const isEnabled = computed(() => !!repoStore.activeRepoId);

async function fetchSessions() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    sessions.value = await bratApi.listSessions(repoStore.activeRepoId);
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to fetch sessions';
  } finally {
    loading.value = false;
  }
}

async function stopSession(sessionId: string) {
  if (!repoStore.activeRepoId) return;
  if (!confirm('Are you sure you want to stop this session?')) return;
  try {
    await bratApi.stopSession(repoStore.activeRepoId, sessionId, 'ui-stop');
    await fetchSessions();
  } catch (e) {
    alert(e instanceof Error ? e.message : 'Failed to stop session');
  }
}

async function viewLogs(session: Session) {
  if (!repoStore.activeRepoId) return;
  selectedSession.value = session;
  logsLoading.value = true;
  try {
    const response = await bratApi.getSessionLogs(repoStore.activeRepoId, session.session_id, 200);
    logs.value = response.lines;
  } catch (e) {
    logs.value = [`Error loading logs: ${e instanceof Error ? e.message : 'Unknown error'}`];
  } finally {
    logsLoading.value = false;
  }
}

function closeLogs() {
  selectedSession.value = null;
  logs.value = [];
}

function formatTime(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

function formatDuration(startTs: number): string {
  const seconds = Math.floor(Date.now() / 1000 - startTs);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

// Auto-refresh sessions every 3 seconds
const { isPolling } = usePolling(fetchSessions, { interval: 3000, enabled: isEnabled });

onMounted(fetchSessions);
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Sessions</h1>
      <div class="flex items-center gap-2 text-sm text-gray-500">
        <LoadingSpinner v-if="isPolling" size="sm" />
        <span>Auto-refresh: 3s</span>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading && sessions.length === 0" class="flex justify-center py-8">
      <LoadingSpinner size="lg" />
    </div>

    <!-- Error -->
    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <!-- Session List -->
    <div v-else class="bg-white rounded-lg shadow">
      <div class="divide-y divide-gray-200">
        <div
          v-for="session in sessions"
          :key="session.session_id"
          class="p-4 hover:bg-gray-50"
        >
          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium font-mono text-sm">{{ session.session_id }}</div>
              <div class="text-sm text-gray-500 mt-1">
                Task: {{ session.task_id }} | Engine: {{ session.engine }}
              </div>
              <div class="text-xs text-gray-400 mt-1">
                Started: {{ formatTime(session.started_ts) }} ({{ formatDuration(session.started_ts) }})
              </div>
            </div>
            <div class="flex items-center gap-3">
              <StatusBadge :status="session.status" />
              <button
                @click="viewLogs(session)"
                class="text-sm text-blue-600 hover:text-blue-800"
              >
                View Logs
              </button>
              <button
                v-if="session.status !== 'exit'"
                @click="stopSession(session.session_id)"
                class="text-sm text-red-600 hover:text-red-800"
              >
                Stop
              </button>
            </div>
          </div>
          <div v-if="session.exit_reason" class="mt-2 text-sm text-gray-500">
            Exit: {{ session.exit_reason }} (code: {{ session.exit_code }})
          </div>
        </div>
        <div v-if="sessions.length === 0" class="p-8 text-center text-gray-500">
          No sessions
        </div>
      </div>
    </div>
  </div>

  <!-- Log Viewer Slide Panel -->
  <div
    v-if="selectedSession"
    class="fixed inset-y-0 right-0 w-1/2 bg-gray-900 shadow-xl z-50 flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-gray-700">
      <div>
        <h3 class="text-white font-medium">Session Logs: {{ selectedSession.session_id }}</h3>
        <p class="text-gray-400 text-sm">
          Task: {{ selectedSession.task_id }} | Engine: {{ selectedSession.engine }}
        </p>
      </div>
      <button @click="closeLogs" class="text-gray-400 hover:text-white">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Log Content -->
    <div class="flex-1 overflow-y-auto p-4 font-mono text-sm text-green-400">
      <div v-if="logsLoading" class="flex justify-center py-8">
        <LoadingSpinner size="lg" />
      </div>
      <pre v-else class="whitespace-pre-wrap">{{ logs.join('\n') }}</pre>
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-between p-4 border-t border-gray-700 text-sm text-gray-400">
      <label class="flex items-center gap-2">
        <input v-model="autoScroll" type="checkbox" class="rounded" />
        Auto-scroll
      </label>
      <span>Lines: {{ logs.length }}</span>
    </div>
  </div>
</template>
