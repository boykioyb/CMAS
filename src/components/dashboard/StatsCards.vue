<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { Users, Gauge, CalendarClock, Activity } from 'lucide-vue-next'

const { t } = useI18n()
const accountStore = useAccountStore()

const real = computed(() => accountStore.realUsage)

const cards = computed(() => [
  {
    icon: Users,
    value: accountStore.accounts.length,
    label: t('dashboard.totalAccounts'),
    color: 'bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400',
  },
  {
    icon: Gauge,
    value: real.value?.session_percent != null ? `${real.value.session_percent}%` : '—',
    sub: real.value?.session_reset,
    label: 'Session',
    color: 'bg-purple-50 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400',
    warn: (real.value?.session_percent ?? 0) >= 80,
  },
  {
    icon: CalendarClock,
    value: real.value?.weekly_all_percent != null ? `${real.value.weekly_all_percent}%` : '—',
    sub: real.value?.weekly_reset,
    label: 'Weekly',
    color: 'bg-emerald-50 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400',
    warn: (real.value?.weekly_all_percent ?? 0) >= 80,
  },
  {
    icon: Activity,
    value: real.value?.success
      ? (real.value?.session_percent != null && real.value.session_percent >= 90 ? t('dashboard.rateLimited') : t('dashboard.active'))
      : '—',
    label: t('dashboard.status'),
    color: real.value?.session_percent != null && real.value.session_percent >= 90
      ? 'bg-red-50 dark:bg-red-900/30 text-red-600 dark:text-red-400'
      : 'bg-emerald-50 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400',
    warn: (real.value?.session_percent ?? 0) >= 90,
  },
])
</script>

<template>
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <div
      v-for="(card, i) in cards"
      :key="i"
      :class="['bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-100 dark:border-gray-700 shadow-sm card-hover animate-fade-in-up', `delay-${i + 1}`]"
    >
      <div :class="['w-9 h-9 rounded-lg flex items-center justify-center mb-3', card.color]">
        <component :is="card.icon" :size="18" />
      </div>
      <div :class="[
        'text-2xl font-bold',
        card.warn ? 'text-red-600 dark:text-red-400' : 'text-gray-900 dark:text-white'
      ]">{{ card.value }}</div>
      <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{{ card.label }}</div>
      <div v-if="card.sub" class="text-[10px] text-gray-400 mt-0.5">{{ card.sub }}</div>
    </div>
  </div>
</template>
