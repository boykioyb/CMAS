<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/configStore'
import { useAccountStore } from '@/stores/accountStore'
import { useCostUsageStore } from '@/stores/costUsageStore'
import AppNavbar from '@/components/common/AppNavbar.vue'
import ToastContainer from '@/components/common/ToastContainer.vue'

const { locale } = useI18n()
const configStore = useConfigStore()
const accountStore = useAccountStore()
const costUsageStore = useCostUsageStore()

let usagePollingTimer: ReturnType<typeof setInterval> | null = null

function startUsagePolling() {
  stopUsagePolling()
  const interval = configStore.config.usage_refresh_interval
  if (interval > 0) {
    usagePollingTimer = setInterval(() => {
      accountStore.scrapeClaudeUsage()
      accountStore.fetchAllAccountUsage()
    }, interval * 1000)
  }
}

function stopUsagePolling() {
  if (usagePollingTimer) {
    clearInterval(usagePollingTimer)
    usagePollingTimer = null
  }
}

onMounted(() => {
  // Run in parallel, don't block rendering
  configStore.loadConfig().then(() => {
    configStore.applyTheme(configStore.config.theme)
    locale.value = configStore.config.language
    startUsagePolling()
  })
  accountStore.fetchAccounts().then(() => {
    // Background: fetch OAuth usage for all accounts so Accounts tab loads instantly
    accountStore.fetchAllAccountUsage()
  })
  costUsageStore.sync()
})

onUnmounted(() => {
  stopUsagePolling()
})

// Restart polling when interval changes
watch(() => configStore.config.usage_refresh_interval, () => {
  startUsagePolling()
})

watch(() => configStore.config.theme, (theme) => {
  configStore.applyTheme(theme)
})

// Listen for system theme changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
  if (configStore.config.theme === 'system') {
    configStore.applyTheme('system')
  }
})
</script>

<template>
  <div class="h-screen flex flex-col bg-background-light dark:bg-background-dark transition-colors duration-200">
    <!-- Titlebar drag region -->
    <div data-tauri-drag-region class="titlebar-drag h-8 w-full fixed top-0 left-0 z-50" />

    <AppNavbar />

    <main class="flex-1 overflow-auto pt-2">
      <div class="max-w-6xl mx-auto px-6 pb-6">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </div>
    </main>

    <ToastContainer />
  </div>
</template>

<style>
.fade-enter-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}
.fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.fade-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
