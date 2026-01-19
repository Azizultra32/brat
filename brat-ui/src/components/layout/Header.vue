<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRepoStore } from '../../stores/repo';
import bratApi from '../../api/brat';
import type { MayorStatus } from '../../types/brat';

const repoStore = useRepoStore();
const showRepoModal = ref(false);
const newRepoPath = ref('');
const mayorStatus = ref<MayorStatus | null>(null);

const activeRepo = computed(() => repoStore.activeRepo);

async function fetchMayorStatus() {
  if (!repoStore.activeRepoId) return;
  try {
    mayorStatus.value = await bratApi.getMayorStatus(repoStore.activeRepoId);
  } catch {
    mayorStatus.value = null;
  }
}

async function addRepo() {
  if (!newRepoPath.value.trim()) return;
  try {
    await repoStore.registerRepo(newRepoPath.value.trim());
    showRepoModal.value = false;
    newRepoPath.value = '';
  } catch (e) {
    // Error handled in store
  }
}

onMounted(() => {
  fetchMayorStatus();
  // Poll mayor status every 10 seconds
  setInterval(fetchMayorStatus, 10000);
});
</script>

<template>
  <header class="bg-white border-b border-gray-200 px-6 py-3 flex items-center justify-between">
    <!-- Left: Repo Selector -->
    <div class="flex items-center gap-4">
      <div class="flex items-center gap-2">
        <label class="text-sm font-medium text-gray-600">Repository:</label>
        <select
          v-if="repoStore.repos.length > 0"
          :value="repoStore.activeRepoId"
          @change="repoStore.selectRepo(($event.target as HTMLSelectElement).value)"
          class="px-3 py-1.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option v-for="repo in repoStore.repos" :key="repo.id" :value="repo.id">
            {{ repo.name }}
          </option>
        </select>
        <span v-else class="text-sm text-gray-500">No repos</span>
        <button
          @click="showRepoModal = true"
          class="px-2 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg"
          title="Add repository"
        >
          +
        </button>
      </div>

      <div v-if="activeRepo" class="text-sm text-gray-500">
        {{ activeRepo.path }}
      </div>
    </div>

    <!-- Right: Mayor Status & Actions -->
    <div class="flex items-center gap-4">
      <!-- Mayor Status -->
      <div class="flex items-center gap-2">
        <span
          :class="[
            'w-2 h-2 rounded-full',
            mayorStatus?.active ? 'bg-green-500' : 'bg-gray-400'
          ]"
        ></span>
        <span class="text-sm text-gray-600">
          Mayor: {{ mayorStatus?.active ? 'Active' : 'Inactive' }}
        </span>
      </div>

      <!-- Refresh Button -->
      <button
        @click="repoStore.fetchStatus()"
        class="px-3 py-1.5 text-sm bg-blue-100 text-blue-700 hover:bg-blue-200 rounded-lg flex items-center gap-1"
        :disabled="repoStore.loading"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        Refresh
      </button>
    </div>
  </header>

  <!-- Add Repo Modal -->
  <div v-if="showRepoModal" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg shadow-xl p-6 w-96">
      <h3 class="text-lg font-semibold mb-4">Add Repository</h3>
      <input
        v-model="newRepoPath"
        type="text"
        placeholder="/path/to/repository"
        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 mb-4"
        @keydown.enter="addRepo"
      />
      <div class="flex justify-end gap-2">
        <button
          @click="showRepoModal = false"
          class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg"
        >
          Cancel
        </button>
        <button
          @click="addRepo"
          class="px-4 py-2 text-sm bg-blue-600 text-white hover:bg-blue-700 rounded-lg"
        >
          Add
        </button>
      </div>
    </div>
  </div>
</template>
