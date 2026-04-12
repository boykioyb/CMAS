import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: () => import('@/pages/DashboardPage.vue'),
    },
    {
      path: '/accounts',
      name: 'accounts',
      component: () => import('@/pages/AccountsPage.vue'),
    },
    {
      path: '/usage',
      name: 'usage',
      component: () => import('@/pages/UsagePage.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/pages/SettingsPage.vue'),
    },
  ],
})

export default router
