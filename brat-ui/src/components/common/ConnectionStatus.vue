<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  connected: boolean;
  reconnecting: boolean;
  reconnectAttempt: number;
}>();

const statusText = computed(() => {
  if (props.connected) return 'Connected';
  if (props.reconnecting) return `Reconnecting (${props.reconnectAttempt})...`;
  return 'Disconnected';
});

const statusColor = computed(() => {
  if (props.connected) return 'text-green-600 bg-green-100';
  if (props.reconnecting) return 'text-yellow-600 bg-yellow-100';
  return 'text-red-600 bg-red-100';
});

const dotColor = computed(() => {
  if (props.connected) return 'bg-green-500';
  if (props.reconnecting) return 'bg-yellow-500 animate-pulse';
  return 'bg-red-500';
});
</script>

<template>
  <div
    :class="[
      'flex items-center gap-2 px-2 py-1 rounded-md text-xs font-medium',
      statusColor
    ]"
    :title="statusText"
  >
    <span :class="['w-2 h-2 rounded-full', dotColor]"></span>
    <span class="hidden sm:inline">{{ statusText }}</span>
  </div>
</template>
