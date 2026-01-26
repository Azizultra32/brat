import { ref, onMounted, onUnmounted, readonly } from 'vue';
import type { Ref } from 'vue';
import type { BratEventData } from '../types/brat';

export interface UseWebSocketOptions {
  /** WebSocket URL (defaults to /api/v1/ws) */
  url?: string;
  /** Reconnect interval in milliseconds (default: 5000) */
  reconnectInterval?: number;
  /** Maximum reconnect attempts (default: 10, 0 = unlimited) */
  maxReconnectAttempts?: number;
  /** Whether to start connection immediately (default: true) */
  immediate?: boolean;
}

export interface WebSocketState {
  /** Whether the WebSocket is connected */
  connected: Ref<boolean>;
  /** Whether we're attempting to reconnect */
  reconnecting: Ref<boolean>;
  /** Current reconnect attempt number */
  reconnectAttempt: Ref<number>;
  /** Last error message */
  lastError: Ref<string | null>;
}

export function useWebSocket(
  onEvent: (event: BratEventData) => void,
  options: UseWebSocketOptions = {}
) {
  const {
    url = getWebSocketUrl(),
    reconnectInterval = 5000,
    maxReconnectAttempts = 10,
    immediate = true,
  } = options;

  const connected = ref(false);
  const reconnecting = ref(false);
  const reconnectAttempt = ref(0);
  const lastError = ref<string | null>(null);

  let ws: WebSocket | null = null;
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  let isManualDisconnect = false;

  function connect() {
    if (ws?.readyState === WebSocket.OPEN || ws?.readyState === WebSocket.CONNECTING) {
      return;
    }

    isManualDisconnect = false;
    lastError.value = null;

    try {
      ws = new WebSocket(url);

      ws.onopen = () => {
        connected.value = true;
        reconnecting.value = false;
        reconnectAttempt.value = 0;
        lastError.value = null;
        console.log('[WebSocket] Connected');
      };

      ws.onclose = (event) => {
        connected.value = false;
        console.log('[WebSocket] Disconnected', event.code, event.reason);

        if (!isManualDisconnect && shouldReconnect()) {
          scheduleReconnect();
        }
      };

      ws.onerror = (error) => {
        console.error('[WebSocket] Error:', error);
        lastError.value = 'Connection error';
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as BratEventData;
          onEvent(data);
        } catch (e) {
          console.error('[WebSocket] Failed to parse message:', e);
        }
      };
    } catch (e) {
      lastError.value = e instanceof Error ? e.message : 'Connection failed';
      if (shouldReconnect()) {
        scheduleReconnect();
      }
    }
  }

  function disconnect() {
    isManualDisconnect = true;
    clearReconnectTimeout();

    if (ws) {
      ws.close(1000, 'Client disconnect');
      ws = null;
    }

    connected.value = false;
    reconnecting.value = false;
    reconnectAttempt.value = 0;
  }

  function shouldReconnect(): boolean {
    if (maxReconnectAttempts === 0) {
      return true; // Unlimited reconnects
    }
    return reconnectAttempt.value < maxReconnectAttempts;
  }

  function scheduleReconnect() {
    clearReconnectTimeout();
    reconnecting.value = true;
    reconnectAttempt.value++;

    const delay = Math.min(
      reconnectInterval * Math.pow(1.5, reconnectAttempt.value - 1),
      30000 // Max 30 seconds
    );

    console.log(`[WebSocket] Reconnecting in ${delay}ms (attempt ${reconnectAttempt.value})`);

    reconnectTimeout = setTimeout(() => {
      connect();
    }, delay);
  }

  function clearReconnectTimeout() {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }
  }

  onMounted(() => {
    if (immediate) {
      connect();
    }
  });

  onUnmounted(() => {
    disconnect();
  });

  return {
    // State (readonly)
    connected: readonly(connected),
    reconnecting: readonly(reconnecting),
    reconnectAttempt: readonly(reconnectAttempt),
    lastError: readonly(lastError),
    // Actions
    connect,
    disconnect,
  };
}

/**
 * Get WebSocket URL from current location.
 * Uses wss:// for https://, ws:// for http://
 */
function getWebSocketUrl(): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = window.location.host;

  // In development, use the Vite proxy
  if (import.meta.env.DEV) {
    // Vite dev server doesn't proxy WebSocket by default, so use the API_BASE
    const apiBase = import.meta.env.VITE_API_BASE || '';
    if (apiBase) {
      // Extract host from API_BASE if it's an absolute URL
      const match = apiBase.match(/^(wss?|https?):\/\/([^/]+)/);
      if (match) {
        const wsProtocol = match[1].startsWith('https') || match[1] === 'wss' ? 'wss:' : 'ws:';
        return `${wsProtocol}//${match[2]}/api/v1/ws`;
      }
    }
  }

  return `${protocol}//${host}/api/v1/ws`;
}
