<script setup lang="ts">

export interface Notification {
  id: number;
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
  detail?: string;
}

const props = defineProps<{
  notifications: Notification[];
}>();

const emit = defineEmits<{
  (e: 'dismiss', id: number): void;
}>();

const iconMap = {
  success: {
    icon: 'M5 13l4 4L19 7',
    color: 'text-green-500 bg-green-100',
  },
  error: {
    icon: 'M6 18L18 6M6 6l12 12',
    color: 'text-red-500 bg-red-100',
  },
  info: {
    icon: 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
    color: 'text-blue-500 bg-blue-100',
  },
  warning: {
    icon: 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z',
    color: 'text-yellow-500 bg-yellow-100',
  },
};

function getIconInfo(type: Notification['type']) {
  return iconMap[type] || iconMap.info;
}
</script>

<template>
  <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
    <transition-group name="notification">
      <div
        v-for="notification in notifications"
        :key="notification.id"
        class="bg-white rounded-lg shadow-lg border border-gray-200 p-4 flex items-start gap-3"
      >
        <!-- Icon -->
        <div
          :class="[
            'flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center',
            getIconInfo(notification.type).color
          ]"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              :d="getIconInfo(notification.type).icon"
            />
          </svg>
        </div>

        <!-- Content -->
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium text-gray-900">{{ notification.message }}</p>
          <p v-if="notification.detail" class="text-xs text-gray-500 mt-1">
            {{ notification.detail }}
          </p>
        </div>

        <!-- Dismiss Button -->
        <button
          @click="emit('dismiss', notification.id)"
          class="flex-shrink-0 text-gray-400 hover:text-gray-600"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.notification-enter-active {
  transition: all 0.3s ease-out;
}

.notification-leave-active {
  transition: all 0.2s ease-in;
}

.notification-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.notification-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
