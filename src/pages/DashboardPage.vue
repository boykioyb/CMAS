<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useAccountStore } from '@/stores/accountStore'
import { useUiStore } from '@/stores/uiStore'
import { Plus, RefreshCw, ArrowRight, Download } from 'lucide-vue-next'
import StatsCards from '@/components/dashboard/StatsCards.vue'
import CurrentAccount from '@/components/dashboard/CurrentAccount.vue'
import BestAccountSuggestion from '@/components/dashboard/BestAccountSuggestion.vue'
import { ref } from 'vue'

const { t } = useI18n()
const router = useRouter()
const accountStore = useAccountStore()
const uiStore = useUiStore()
const refreshing = ref(false)

async function handleRefresh() {
  refreshing.value = true
  try {
    await accountStore.fetchAccounts()
    uiStore.showToast('success', t('common.success'))
  } catch (e) {
    uiStore.showToast('error', String(e))
  } finally {
    refreshing.value = false
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between animate-fade-in-up">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">
        {{ t('dashboard.greeting') }} 👋
      </h1>
      <div class="flex items-center gap-2">
        <button
          @click="router.push('/accounts')"
          class="flex items-center gap-2 px-3 py-2 rounded-lg border border-gray-200 dark:border-gray-700 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        >
          <Plus :size="16" />
          {{ t('dashboard.addAccount') }}
        </button>
        <button
          @click="handleRefresh"
          :disabled="refreshing"
          class="flex items-center gap-2 px-3 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 disabled:opacity-50 transition-colors"
        >
          <RefreshCw :size="16" :class="{ 'animate-spin': refreshing }" />
          {{ t('dashboard.refreshQuota') }}
        </button>
      </div>
    </div>

    <!-- Stats -->
    <StatsCards />

    <!-- Main content grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 animate-fade-in-up delay-3">
      <CurrentAccount />
      <BestAccountSuggestion />
    </div>

    <!-- Quick links -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 animate-fade-in-up delay-5">
      <button
        @click="router.push('/accounts')"
        class="flex items-center justify-between p-4 bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm hover:border-primary-300 dark:hover:border-primary-600 card-hover group"
      >
        <span class="text-sm font-medium text-primary-600 dark:text-primary-400">{{ t('dashboard.viewAll') }}</span>
        <ArrowRight :size="16" class="text-primary-400 group-hover:translate-x-1 transition-transform" />
      </button>
      <button
        class="flex items-center justify-between p-4 bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm hover:border-primary-300 dark:hover:border-primary-600 card-hover group"
      >
        <span class="text-sm font-medium text-primary-600 dark:text-primary-400">{{ t('dashboard.exportData') }}</span>
        <Download :size="16" class="text-primary-400 group-hover:translate-y-0.5 transition-transform" />
      </button>
    </div>
  </div>
</template>
