import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Repo, StatusOutput } from '../types/brat';
import bratApi from '../api/brat';

export const useRepoStore = defineStore('repo', () => {
  // State
  const repos = ref<Repo[]>([]);
  const activeRepoId = ref<string | null>(null);
  const status = ref<StatusOutput | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Getters
  const activeRepo = computed(() =>
    repos.value.find(r => r.id === activeRepoId.value) || null
  );

  const hasRepos = computed(() => repos.value.length > 0);

  // Actions
  async function fetchRepos() {
    loading.value = true;
    error.value = null;
    try {
      repos.value = await bratApi.listRepos();
      // Auto-select first repo if none selected
      const firstRepo = repos.value[0];
      if (!activeRepoId.value && firstRepo) {
        activeRepoId.value = firstRepo.id;
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch repos';
    } finally {
      loading.value = false;
    }
  }

  async function registerRepo(path: string) {
    loading.value = true;
    error.value = null;
    try {
      const result = await bratApi.registerRepo(path);
      if (result.success && result.repo) {
        repos.value.push(result.repo);
        activeRepoId.value = result.repo.id;
      } else {
        throw new Error(result.error || 'Failed to register repo');
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to register repo';
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function fetchStatus() {
    if (!activeRepoId.value) return;

    loading.value = true;
    error.value = null;
    try {
      status.value = await bratApi.getStatus(activeRepoId.value);
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch status';
    } finally {
      loading.value = false;
    }
  }

  function selectRepo(repoId: string) {
    activeRepoId.value = repoId;
    status.value = null;
  }

  return {
    // State
    repos,
    activeRepoId,
    status,
    loading,
    error,
    // Getters
    activeRepo,
    hasRepos,
    // Actions
    fetchRepos,
    registerRepo,
    fetchStatus,
    selectRepo,
  };
});
