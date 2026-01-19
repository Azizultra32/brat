<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  status: string;
}>();

const badgeClass = computed(() => {
  const base = 'px-2 py-1 rounded-full text-xs font-medium';
  const colors: Record<string, string> = {
    // Task statuses
    queued: 'bg-indigo-100 text-indigo-800',
    running: 'bg-green-100 text-green-800',
    blocked: 'bg-red-100 text-red-800',
    'needs-review': 'bg-amber-100 text-amber-800',
    merged: 'bg-cyan-100 text-cyan-800',
    dropped: 'bg-gray-100 text-gray-800',
    // Session statuses
    spawned: 'bg-purple-100 text-purple-800',
    ready: 'bg-blue-100 text-blue-800',
    handoff: 'bg-orange-100 text-orange-800',
    exit: 'bg-gray-100 text-gray-800',
    // Convoy statuses
    active: 'bg-green-100 text-green-800',
    paused: 'bg-yellow-100 text-yellow-800',
    complete: 'bg-cyan-100 text-cyan-800',
    failed: 'bg-red-100 text-red-800',
  };
  return `${base} ${colors[props.status] || 'bg-gray-100 text-gray-800'}`;
});
</script>

<template>
  <span :class="badgeClass">{{ status }}</span>
</template>
