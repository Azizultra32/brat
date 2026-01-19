<script setup lang="ts">
import { onMounted } from 'vue';
import { useRepoStore } from './stores/repo';
import Sidebar from './components/layout/Sidebar.vue';
import Header from './components/layout/Header.vue';

const repoStore = useRepoStore();

onMounted(() => {
  repoStore.fetchRepos();
});
</script>

<template>
  <div class="flex h-screen bg-gray-100">
    <!-- Sidebar -->
    <Sidebar />

    <!-- Main Content -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Header -->
      <Header />

      <!-- Page Content -->
      <main class="flex-1 overflow-y-auto p-6">
        <router-view />
      </main>
    </div>
  </div>
</template>
