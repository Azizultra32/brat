<script setup lang="ts">
defineProps<{
  label: string;
  count: number;
  color?: 'blue' | 'green' | 'red' | 'amber' | 'cyan' | 'gray' | 'indigo';
}>();

const colorClasses: Record<string, string> = {
  blue: 'bg-blue-100 text-blue-800 border-blue-200',
  green: 'bg-green-100 text-green-800 border-green-200',
  red: 'bg-red-100 text-red-800 border-red-200',
  amber: 'bg-amber-100 text-amber-800 border-amber-200',
  cyan: 'bg-cyan-100 text-cyan-800 border-cyan-200',
  gray: 'bg-gray-100 text-gray-800 border-gray-200',
  indigo: 'bg-indigo-100 text-indigo-800 border-indigo-200',
};
</script>

<template>
  <div
    :class="[
      'p-4 rounded-lg border',
      colorClasses[color || 'gray']
    ]"
  >
    <div class="text-3xl font-bold">{{ count }}</div>
    <div class="text-sm font-medium mt-1">{{ label }}</div>
  </div>
</template>
