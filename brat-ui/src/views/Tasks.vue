<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useRepoStore } from '../stores/repo';
import bratApi from '../api/brat';
import type { Task, Convoy } from '../types/brat';
import StatusBadge from '../components/common/StatusBadge.vue';
import Modal from '../components/common/Modal.vue';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();
const tasks = ref<Task[]>([]);
const convoys = ref<Convoy[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

// Filters
const filterConvoy = ref<string>('');
const filterStatus = ref<string>('');

// Create task modal
const showCreateModal = ref(false);
const newTask = ref({ convoy_id: '', title: '', body: '' });
const creating = ref(false);
const createError = ref<string | null>(null);

// Expanded task
const expandedTaskId = ref<string | null>(null);

const statuses = ['queued', 'running', 'blocked', 'needs-review', 'merged', 'dropped'];

async function fetchTasks() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    const filters: { convoy?: string; status?: string } = {};
    if (filterConvoy.value) filters.convoy = filterConvoy.value;
    if (filterStatus.value) filters.status = filterStatus.value;
    tasks.value = await bratApi.listTasks(repoStore.activeRepoId, filters);
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to fetch tasks';
  } finally {
    loading.value = false;
  }
}

async function fetchConvoys() {
  if (!repoStore.activeRepoId) return;
  try {
    convoys.value = await bratApi.listConvoys(repoStore.activeRepoId);
  } catch (e) {
    // Silently fail
  }
}

async function updateTaskStatus(taskId: string, status: string) {
  if (!repoStore.activeRepoId) return;
  try {
    await bratApi.updateTask(repoStore.activeRepoId, taskId, { status });
    await fetchTasks();
  } catch (e) {
    alert(e instanceof Error ? e.message : 'Failed to update task');
  }
}

async function createTask() {
  if (!repoStore.activeRepoId || !newTask.value.convoy_id || !newTask.value.title.trim()) return;
  creating.value = true;
  createError.value = null;
  try {
    await bratApi.createTask(repoStore.activeRepoId, {
      convoy_id: newTask.value.convoy_id,
      title: newTask.value.title.trim(),
      body: newTask.value.body.trim() || undefined,
    });
    showCreateModal.value = false;
    newTask.value = { convoy_id: '', title: '', body: '' };
    await fetchTasks();
  } catch (e) {
    createError.value = e instanceof Error ? e.message : 'Failed to create task';
  } finally {
    creating.value = false;
  }
}

function toggleExpanded(taskId: string) {
  expandedTaskId.value = expandedTaskId.value === taskId ? null : taskId;
}

// Watch filters
watch([filterConvoy, filterStatus], () => {
  fetchTasks();
});

onMounted(() => {
  fetchTasks();
  fetchConvoys();
});
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Tasks</h1>
      <button
        @click="showCreateModal = true"
        class="btn-primary flex items-center gap-2"
        :disabled="!repoStore.activeRepoId || convoys.length === 0"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Create Task
      </button>
    </div>

    <!-- Filters -->
    <div class="flex gap-4">
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">Convoy</label>
        <select v-model="filterConvoy" class="input">
          <option value="">All Convoys</option>
          <option v-for="convoy in convoys" :key="convoy.convoy_id" :value="convoy.convoy_id">
            {{ convoy.title }}
          </option>
        </select>
      </div>
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">Status</label>
        <select v-model="filterStatus" class="input">
          <option value="">All Statuses</option>
          <option v-for="status in statuses" :key="status" :value="status">
            {{ status }}
          </option>
        </select>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex justify-center py-8">
      <LoadingSpinner size="lg" />
    </div>

    <!-- Error -->
    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <!-- Task List -->
    <div v-else class="bg-white rounded-lg shadow">
      <div class="divide-y divide-gray-200">
        <div
          v-for="task in tasks"
          :key="task.task_id"
          class="p-4 hover:bg-gray-50 cursor-pointer"
          @click="toggleExpanded(task.task_id)"
        >
          <div class="flex items-center justify-between">
            <div class="flex-1">
              <div class="font-medium">{{ task.title }}</div>
              <div class="text-sm text-gray-500 font-mono">
                {{ task.task_id }} | Convoy: {{ task.convoy_id }}
              </div>
            </div>
            <div class="flex items-center gap-2" @click.stop>
              <select
                :value="task.status"
                @change="updateTaskStatus(task.task_id, ($event.target as HTMLSelectElement).value)"
                class="text-sm border border-gray-300 rounded px-2 py-1"
              >
                <option v-for="status in statuses" :key="status" :value="status">
                  {{ status }}
                </option>
              </select>
              <StatusBadge :status="task.status" />
            </div>
          </div>

          <!-- Expanded Details -->
          <div
            v-if="expandedTaskId === task.task_id && task.body"
            class="mt-4 p-3 bg-gray-50 rounded text-sm text-gray-700 whitespace-pre-wrap"
          >
            {{ task.body }}
          </div>
        </div>
        <div v-if="tasks.length === 0" class="p-8 text-center text-gray-500">
          No tasks found
        </div>
      </div>
    </div>
  </div>

  <!-- Create Task Modal -->
  <Modal
    :show="showCreateModal"
    title="Create New Task"
    @close="showCreateModal = false"
  >
    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">
          Convoy <span class="text-red-500">*</span>
        </label>
        <select v-model="newTask.convoy_id" class="input w-full">
          <option value="" disabled>Select a convoy</option>
          <option v-for="convoy in convoys" :key="convoy.convoy_id" :value="convoy.convoy_id">
            {{ convoy.title }}
          </option>
        </select>
      </div>
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">
          Title <span class="text-red-500">*</span>
        </label>
        <input
          v-model="newTask.title"
          type="text"
          class="input w-full"
          placeholder="Fix ZeroDivisionError in process_data()"
        />
      </div>
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">
          Instructions
        </label>
        <textarea
          v-model="newTask.body"
          rows="6"
          class="input w-full"
          placeholder="Detailed instructions for the task..."
        ></textarea>
      </div>
      <div v-if="createError" class="text-red-600 text-sm">
        {{ createError }}
      </div>
    </div>

    <template #footer>
      <button
        @click="showCreateModal = false"
        class="btn-secondary"
        :disabled="creating"
      >
        Cancel
      </button>
      <button
        @click="createTask"
        class="btn-primary"
        :disabled="creating || !newTask.convoy_id || !newTask.title.trim()"
      >
        <LoadingSpinner v-if="creating" size="sm" />
        <span v-else>Create Task</span>
      </button>
    </template>
  </Modal>
</template>
