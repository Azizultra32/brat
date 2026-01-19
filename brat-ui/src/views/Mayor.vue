<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import bratApi from '../api/brat';
import type { MayorStatus, MayorMessage } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

// State
const mayorStatus = ref<MayorStatus | null>(null);
const messages = ref<MayorMessage[]>([]);
const inputMessage = ref('');
const loading = ref(false);
const sending = ref(false);
const error = ref<string | null>(null);

// Refs
const messagesContainer = ref<HTMLElement | null>(null);

const isActive = computed(() => mayorStatus.value?.active ?? false);
const isEnabled = computed(() => !!repoStore.activeRepoId && isActive.value);

// Fetch mayor status
async function fetchStatus() {
  if (!repoStore.activeRepoId) return;
  try {
    mayorStatus.value = await bratApi.getMayorStatus(repoStore.activeRepoId);
  } catch (e) {
    mayorStatus.value = { active: false };
  }
}

// Fetch conversation history
async function fetchHistory() {
  if (!repoStore.activeRepoId || !isActive.value) return;
  try {
    const response = await bratApi.getMayorHistory(repoStore.activeRepoId, 100);
    parseHistoryLines(response.lines);
  } catch (e) {
    // Silently fail
  }
}

// Parse history lines into messages
function parseHistoryLines(lines: string[]) {
  const newMessages: MayorMessage[] = [];
  let currentMessage: MayorMessage | null = null;

  for (const line of lines) {
    if (line.startsWith('>>> ')) {
      // User message
      if (currentMessage) newMessages.push(currentMessage);
      currentMessage = {
        type: 'user',
        content: line.substring(4),
      };
    } else if (line.trim() === '') {
      // Empty line - might end a message
      if (currentMessage) {
        newMessages.push(currentMessage);
        currentMessage = null;
      }
    } else {
      // Mayor response line
      if (currentMessage?.type === 'user') {
        newMessages.push(currentMessage);
        currentMessage = {
          type: 'mayor',
          content: line,
        };
      } else if (currentMessage?.type === 'mayor') {
        currentMessage.content += '\n' + line;
      } else {
        currentMessage = {
          type: 'mayor',
          content: line,
        };
      }
    }
  }

  if (currentMessage) newMessages.push(currentMessage);
  messages.value = newMessages;
}

// Start Mayor
async function startMayor() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    const response = await bratApi.startMayor(repoStore.activeRepoId);
    mayorStatus.value = { active: true, session_id: response.session_id };
    // Add initial response
    if (response.response.length > 0) {
      messages.value = [{
        type: 'mayor',
        content: response.response.join('\n'),
      }];
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to start Mayor';
  } finally {
    loading.value = false;
  }
}

// Stop Mayor
async function stopMayor() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    await bratApi.stopMayor(repoStore.activeRepoId);
    mayorStatus.value = { active: false };
    messages.value = [];
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to stop Mayor';
  } finally {
    loading.value = false;
  }
}

// Send message to Mayor
async function sendMessage() {
  if (!repoStore.activeRepoId || !inputMessage.value.trim() || sending.value) return;

  const message = inputMessage.value.trim();
  inputMessage.value = '';

  // Add user message immediately
  messages.value.push({
    type: 'user',
    content: message,
  });

  await scrollToBottom();

  sending.value = true;
  error.value = null;

  try {
    const response = await bratApi.askMayor(repoStore.activeRepoId, message);
    messages.value.push({
      type: 'mayor',
      content: response.response.join('\n'),
    });
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to send message';
    // Add error as mayor message
    messages.value.push({
      type: 'mayor',
      content: `Error: ${error.value}`,
    });
  } finally {
    sending.value = false;
    await scrollToBottom();
  }
}

async function scrollToBottom() {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

// Poll history when active
const { } = usePolling(fetchHistory, { interval: 3000, enabled: isEnabled });

onMounted(async () => {
  await fetchStatus();
  if (isActive.value) {
    await fetchHistory();
  }
});

// Watch for active status changes
watch(isActive, async (active) => {
  if (active) {
    await fetchHistory();
  }
});
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold text-gray-900">Mayor</h1>
        <span
          :class="[
            'flex items-center gap-2 px-3 py-1 rounded-full text-sm font-medium',
            isActive ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
          ]"
        >
          <span :class="['w-2 h-2 rounded-full', isActive ? 'bg-green-500' : 'bg-gray-400']"></span>
          {{ isActive ? 'Active' : 'Inactive' }}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="!isActive"
          @click="startMayor"
          class="btn-primary"
          :disabled="loading || !repoStore.activeRepoId"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Start Mayor</span>
        </button>
        <button
          v-else
          @click="stopMayor"
          class="btn-danger"
          :disabled="loading"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Stop Mayor</span>
        </button>
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="bg-red-50 text-red-700 p-3 rounded-lg mb-4">
      {{ error }}
    </div>

    <!-- Chat Container -->
    <div class="flex-1 bg-white rounded-lg shadow flex flex-col overflow-hidden">
      <!-- Messages -->
      <div
        ref="messagesContainer"
        class="flex-1 overflow-y-auto p-4 space-y-4"
      >
        <div v-if="!isActive && messages.length === 0" class="text-center py-16 text-gray-500">
          <p class="text-lg mb-2">Mayor is not active</p>
          <p class="text-sm">Start the Mayor to begin interacting with the AI orchestrator.</p>
        </div>

        <div
          v-for="(msg, index) in messages"
          :key="index"
          :class="[
            'max-w-3xl',
            msg.type === 'user' ? 'ml-auto' : ''
          ]"
        >
          <!-- User Message -->
          <div
            v-if="msg.type === 'user'"
            class="bg-blue-600 text-white px-4 py-2 rounded-lg rounded-br-none"
          >
            <span class="text-blue-200 font-mono mr-2">>>></span>
            {{ msg.content }}
          </div>

          <!-- Mayor Response -->
          <div
            v-else
            class="bg-gray-100 px-4 py-3 rounded-lg rounded-bl-none"
          >
            <div class="prose prose-sm max-w-none" v-html="formatMessage(msg.content)"></div>
          </div>
        </div>

        <!-- Sending indicator -->
        <div v-if="sending" class="flex items-center gap-2 text-gray-500">
          <LoadingSpinner size="sm" />
          <span>Mayor is thinking...</span>
        </div>
      </div>

      <!-- Input -->
      <div class="border-t border-gray-200 p-4">
        <div class="flex gap-2">
          <input
            v-model="inputMessage"
            type="text"
            placeholder="Ask the Mayor..."
            class="flex-1 input"
            :disabled="!isActive || sending"
            @keydown.enter="sendMessage"
          />
          <button
            @click="sendMessage"
            class="btn-primary"
            :disabled="!isActive || !inputMessage.trim() || sending"
          >
            <span v-if="sending">...</span>
            <span v-else>Send</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
// Simple markdown-ish formatting
function formatMessage(content: string): string {
  return content
    // Escape HTML
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    // Bold
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    // Code blocks
    .replace(/```([\s\S]*?)```/g, '<pre class="bg-gray-800 text-green-400 p-2 rounded mt-2 mb-2 overflow-x-auto"><code>$1</code></pre>')
    // Inline code
    .replace(/`([^`]+)`/g, '<code class="bg-gray-200 px-1 rounded">$1</code>')
    // Line breaks
    .replace(/\n/g, '<br />');
}

export { formatMessage };
</script>
