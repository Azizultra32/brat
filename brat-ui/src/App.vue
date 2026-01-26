<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRepoStore } from './stores/repo';
import { useWebSocket } from './composables/useWebSocket';
import type { BratEventData } from './types/brat';
import type { Notification } from './components/common/NotificationToast.vue';
import Sidebar from './components/layout/Sidebar.vue';
import Header from './components/layout/Header.vue';
import ConnectionStatus from './components/common/ConnectionStatus.vue';
import NotificationToast from './components/common/NotificationToast.vue';

const repoStore = useRepoStore();

// Notifications state
const notifications = ref<Notification[]>([]);
let notificationId = 0;

function notify(type: Notification['type'], message: string, detail?: string) {
  const id = ++notificationId;
  notifications.value.push({ id, type, message, detail });

  // Auto-dismiss after 5 seconds
  setTimeout(() => {
    dismissNotification(id);
  }, 5000);
}

function dismissNotification(id: number) {
  notifications.value = notifications.value.filter(n => n.id !== id);
}

// Handle WebSocket events
function handleEvent(event: BratEventData) {
  // Refresh status on relevant events
  const shouldRefresh = [
    'TaskUpdated',
    'SessionStarted',
    'SessionExited',
    'MergeCompleted',
    'MergeFailed',
    'MergeRolledBack',
  ].includes(event.type);

  if (shouldRefresh) {
    repoStore.fetchStatus();
  }

  // Show notifications for important events
  switch (event.type) {
    case 'TaskUpdated':
      notify('info', `Task ${event.data.task_id} updated`, `Status: ${event.data.status}`);
      break;
    case 'SessionStarted':
      notify('info', `Session started`, `Task: ${event.data.task_id}, Engine: ${event.data.engine}`);
      break;
    case 'SessionExited':
      notify(
        event.data.exit_code === 0 ? 'success' : 'warning',
        `Session exited`,
        `Task: ${event.data.task_id}, Exit code: ${event.data.exit_code}`
      );
      break;
    case 'MergeCompleted':
      notify('success', 'Merge completed', `Task: ${event.data.task_id}`);
      break;
    case 'MergeFailed':
      notify('error', 'Merge failed', `Task: ${event.data.task_id}: ${event.data.error}`);
      break;
    case 'MergeRolledBack':
      notify('warning', 'Merge rolled back', `Task: ${event.data.task_id}, will retry`);
      break;
  }
}

// Initialize WebSocket
const { connected, reconnecting, reconnectAttempt } = useWebSocket(handleEvent);

onMounted(() => {
  repoStore.fetchRepos();
});
</script>

<template>
  <div class="flex h-screen bg-gray-100">
    <!-- Sidebar -->
    <Sidebar />

    <!-- Main Content -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Header -->
      <Header />

      <!-- Connection Status Bar -->
      <div class="bg-white border-b border-gray-200 px-6 py-1 flex items-center justify-end">
        <ConnectionStatus
          :connected="connected"
          :reconnecting="reconnecting"
          :reconnect-attempt="reconnectAttempt"
        />
      </div>

      <!-- Page Content -->
      <main class="flex-1 overflow-y-auto p-6">
        <router-view />
      </main>
    </div>

    <!-- Notification Toasts -->
    <NotificationToast
      :notifications="notifications"
      @dismiss="dismissNotification"
    />
  </div>
</template>
