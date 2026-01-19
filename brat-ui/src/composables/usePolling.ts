import { ref, onMounted, onUnmounted, watch } from 'vue';
import type { Ref } from 'vue';

export interface UsePollingOptions {
  /** Polling interval in milliseconds */
  interval?: number;
  /** Whether to start polling immediately */
  immediate?: boolean;
  /** Whether polling is enabled */
  enabled?: Ref<boolean> | boolean;
}

export function usePolling(
  callback: () => Promise<void> | void,
  options: UsePollingOptions = {}
) {
  const { interval = 5000, immediate = true, enabled = true } = options;

  const isPolling = ref(false);
  const lastError = ref<Error | null>(null);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  const isEnabled = typeof enabled === 'boolean' ? ref(enabled) : enabled;

  async function poll() {
    if (!isEnabled.value) return;

    isPolling.value = true;
    lastError.value = null;
    try {
      await callback();
    } catch (e) {
      lastError.value = e instanceof Error ? e : new Error(String(e));
    } finally {
      isPolling.value = false;
    }
  }

  function start() {
    if (intervalId) return;

    if (immediate) {
      poll();
    }

    intervalId = setInterval(poll, interval);
  }

  function stop() {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  function restart() {
    stop();
    start();
  }

  // Watch enabled state
  watch(isEnabled, (newEnabled) => {
    if (newEnabled) {
      start();
    } else {
      stop();
    }
  });

  onMounted(() => {
    if (isEnabled.value) {
      start();
    }
  });

  onUnmounted(() => {
    stop();
  });

  return {
    isPolling,
    lastError,
    poll,
    start,
    stop,
    restart,
  };
}
