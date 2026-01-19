<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import StatusCard from '../components/common/StatusCard.vue';
import StatusBadge from '../components/common/StatusBadge.vue';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const isEnabled = computed(() => !!repoStore.activeRepoId);

// Auto-refresh status every 5 seconds
const { isPolling } = usePolling(
  async () => {
    await repoStore.fetchStatus();
  },
  { interval: 5000, enabled: isEnabled }
);

const status = computed(() => repoStore.status);
const taskCounts = computed(() => status.value?.tasks?.by_status);

onMounted(() => {
  if (repoStore.activeRepoId) {
    repoStore.fetchStatus();
  }
});
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Dashboard</h1>
      <div class="flex items-center gap-2 text-sm text-gray-500">
        <LoadingSpinner v-if="isPolling" size="sm" />
        <span>Auto-refresh: 5s</span>
      </div>
    </div>

    <!-- No Repo Selected -->
    <div v-if="!repoStore.activeRepoId" class="bg-white rounded-lg shadow p-8 text-center">
      <p class="text-gray-600">No repository selected. Add a repository to get started.</p>
    </div>

    <!-- Status Cards -->
    <div v-else-if="taskCounts" class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
      <StatusCard label="Queued" :count="taskCounts.queued" color="indigo" />
      <StatusCard label="Running" :count="taskCounts.running" color="green" />
      <StatusCard label="Blocked" :count="taskCounts.blocked" color="red" />
      <StatusCard label="Needs Review" :count="taskCounts.needs_review" color="amber" />
      <StatusCard label="Merged" :count="taskCounts.merged" color="cyan" />
      <StatusCard label="Dropped" :count="taskCounts.dropped" color="gray" />
    </div>

    <!-- Convoys Section -->
    <div v-if="status" class="bg-white rounded-lg shadow">
      <div class="p-4 border-b border-gray-200 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Convoys</h2>
        <router-link to="/convoys" class="text-sm text-blue-600 hover:text-blue-800">
          View all
        </router-link>
      </div>
      <div class="divide-y divide-gray-200">
        <div
          v-for="convoy in status.convoys.slice(0, 5)"
          :key="convoy.convoy_id"
          class="p-4 hover:bg-gray-50"
        >
          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium">{{ convoy.title }}</div>
              <div class="text-sm text-gray-500">{{ convoy.convoy_id }}</div>
            </div>
            <StatusBadge :status="convoy.status" />
          </div>
          <div class="mt-2 text-sm text-gray-600">
            Tasks:
            <span class="text-indigo-600">{{ convoy.task_counts.queued }} queued</span>,
            <span class="text-green-600">{{ convoy.task_counts.running }} running</span>,
            <span class="text-red-600">{{ convoy.task_counts.blocked }} blocked</span>
          </div>
        </div>
        <div v-if="status.convoys.length === 0" class="p-4 text-gray-500 text-center">
          No convoys yet
        </div>
      </div>
    </div>

    <!-- Active Sessions Section -->
    <div v-if="status" class="bg-white rounded-lg shadow">
      <div class="p-4 border-b border-gray-200 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Active Sessions ({{ status.sessions.length }})</h2>
        <router-link to="/sessions" class="text-sm text-blue-600 hover:text-blue-800">
          View all
        </router-link>
      </div>
      <div class="divide-y divide-gray-200">
        <div
          v-for="session in status.sessions.slice(0, 5)"
          :key="session.session_id"
          class="p-4 hover:bg-gray-50"
        >
          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium font-mono text-sm">{{ session.session_id }}</div>
              <div class="text-sm text-gray-500">
                Engine: {{ session.engine }} | Task: {{ session.task_id }}
              </div>
            </div>
            <StatusBadge :status="session.status" />
          </div>
        </div>
        <div v-if="status.sessions.length === 0" class="p-4 text-gray-500 text-center">
          No active sessions
        </div>
      </div>
    </div>
  </div>
</template>
