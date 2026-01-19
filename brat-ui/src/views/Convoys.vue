<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRepoStore } from '../stores/repo';
import bratApi from '../api/brat';
import type { Convoy } from '../types/brat';
import StatusBadge from '../components/common/StatusBadge.vue';
import Modal from '../components/common/Modal.vue';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();
const convoys = ref<Convoy[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const showCreateModal = ref(false);

// Create convoy form
const newConvoy = ref({ title: '', body: '' });
const creating = ref(false);
const createError = ref<string | null>(null);

async function fetchConvoys() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    convoys.value = await bratApi.listConvoys(repoStore.activeRepoId);
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to fetch convoys';
  } finally {
    loading.value = false;
  }
}

async function createConvoy() {
  if (!repoStore.activeRepoId || !newConvoy.value.title.trim()) return;
  creating.value = true;
  createError.value = null;
  try {
    await bratApi.createConvoy(repoStore.activeRepoId, {
      title: newConvoy.value.title.trim(),
      body: newConvoy.value.body.trim() || undefined,
    });
    showCreateModal.value = false;
    newConvoy.value = { title: '', body: '' };
    await fetchConvoys();
  } catch (e) {
    createError.value = e instanceof Error ? e.message : 'Failed to create convoy';
  } finally {
    creating.value = false;
  }
}

onMounted(fetchConvoys);
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Convoys</h1>
      <button
        @click="showCreateModal = true"
        class="btn-primary flex items-center gap-2"
        :disabled="!repoStore.activeRepoId"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Create Convoy
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex justify-center py-8">
      <LoadingSpinner size="lg" />
    </div>

    <!-- Error -->
    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <!-- Convoy List -->
    <div v-else class="bg-white rounded-lg shadow">
      <div class="divide-y divide-gray-200">
        <div
          v-for="convoy in convoys"
          :key="convoy.convoy_id"
          class="p-4 hover:bg-gray-50"
        >
          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium text-lg">{{ convoy.title }}</div>
              <div class="text-sm text-gray-500 font-mono">{{ convoy.convoy_id }}</div>
            </div>
            <StatusBadge :status="convoy.status" />
          </div>
          <div v-if="convoy.body" class="mt-2 text-gray-600 text-sm">
            {{ convoy.body }}
          </div>
        </div>
        <div v-if="convoys.length === 0" class="p-8 text-center text-gray-500">
          No convoys yet. Create one to get started.
        </div>
      </div>
    </div>
  </div>

  <!-- Create Convoy Modal -->
  <Modal
    :show="showCreateModal"
    title="Create New Convoy"
    @close="showCreateModal = false"
  >
    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">
          Title <span class="text-red-500">*</span>
        </label>
        <input
          v-model="newConvoy.title"
          type="text"
          class="input w-full"
          placeholder="Bug Fixes - Sprint 12"
        />
      </div>
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-1">
          Description
        </label>
        <textarea
          v-model="newConvoy.body"
          rows="4"
          class="input w-full"
          placeholder="Description of the convoy..."
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
        @click="createConvoy"
        class="btn-primary"
        :disabled="creating || !newConvoy.title.trim()"
      >
        <LoadingSpinner v-if="creating" size="sm" />
        <span v-else>Create Convoy</span>
      </button>
    </template>
  </Modal>
</template>
