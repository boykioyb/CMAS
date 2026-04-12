<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useConfigStore } from '@/stores/configStore'
import { LayoutDashboard, Users, DollarSign, Settings, Sun, Moon, Monitor } from 'lucide-vue-next'
import { computed } from 'vue'

const { t, locale } = useI18n()
const route = useRoute()
const router = useRouter()
const configStore = useConfigStore()

const navItems = computed(() => [
  { path: '/', name: 'dashboard', label: t('nav.dashboard'), icon: LayoutDashboard },
  { path: '/accounts', name: 'accounts', label: t('nav.accounts'), icon: Users },
  { path: '/usage', name: 'usage', label: t('nav.usage'), icon: DollarSign },
  { path: '/settings', name: 'settings', label: t('nav.settings'), icon: Settings },
])

const themeIcon = computed(() => {
  if (configStore.config.theme === 'dark') return Moon
  if (configStore.config.theme === 'light') return Sun
  return Monitor
})

function cycleTheme() {
  const themes = ['light', 'dark', 'system'] as const
  const idx = themes.indexOf(configStore.config.theme as any)
  const next = themes[(idx + 1) % themes.length]
  configStore.updateConfig({ theme: next })
}

function toggleLanguage() {
  const next = locale.value === 'vi' ? 'en' : 'vi'
  locale.value = next
  configStore.updateConfig({ language: next as 'vi' | 'en' })
}
</script>

<template>
  <nav data-tauri-drag-region class="titlebar-drag sticky top-0 z-40 flex items-center justify-between px-6 pt-9 pb-4 bg-background-light/80 dark:bg-background-dark/80 backdrop-blur-md">
    <!-- Logo -->
    <div class="titlebar-no-drag flex items-center gap-2">
      <div class="w-8 h-8 rounded-lg bg-primary-500 flex items-center justify-center">
        <span class="text-white font-bold text-sm">C</span>
      </div>
      <span class="font-semibold text-gray-900 dark:text-white text-sm hidden sm:inline">CMAS</span>
    </div>

    <!-- Nav pills -->
    <div class="titlebar-no-drag flex items-center bg-gray-100 dark:bg-gray-800 rounded-full p-1">
      <button
        v-for="item in navItems"
        :key="item.path"
        @click="router.push(item.path)"
        :class="[
          'flex items-center gap-2 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200',
          route.path === item.path
            ? 'bg-gray-900 text-white dark:bg-white dark:text-gray-900 shadow-sm'
            : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
        ]"
      >
        <component :is="item.icon" :size="16" />
        <span class="hidden md:inline">{{ item.label }}</span>
      </button>
    </div>

    <!-- Right controls -->
    <div class="titlebar-no-drag flex items-center gap-2">
      <button
        @click="cycleTheme"
        class="p-2 rounded-lg text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        :title="configStore.config.theme"
      >
        <component :is="themeIcon" :size="18" />
      </button>

      <button
        @click="toggleLanguage"
        class="px-2.5 py-1 rounded-lg text-xs font-bold text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors border border-gray-200 dark:border-gray-700"
      >
        {{ locale === 'vi' ? 'VI' : 'EN' }}
      </button>
    </div>
  </nav>
</template>
