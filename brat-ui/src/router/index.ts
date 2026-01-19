import { createRouter, createWebHistory } from 'vue-router';

const routes = [
  {
    path: '/',
    redirect: '/dashboard',
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('../views/Dashboard.vue'),
  },
  {
    path: '/convoys',
    name: 'Convoys',
    component: () => import('../views/Convoys.vue'),
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('../views/Tasks.vue'),
  },
  {
    path: '/sessions',
    name: 'Sessions',
    component: () => import('../views/Sessions.vue'),
  },
  {
    path: '/mayor',
    name: 'Mayor',
    component: () => import('../views/Mayor.vue'),
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
