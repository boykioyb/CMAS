<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCostUsageStore } from '@/stores/costUsageStore'
import { RefreshCw, ArrowUpDown, Filter, DollarSign, Zap, MessageSquare, Calendar, ChevronDown, BarChart3 } from 'lucide-vue-next'
import CostCharts from '@/components/usage/CostCharts.vue'
import type { CostUsageRecord } from '@/types'

const { t } = useI18n()
const store = useCostUsageStore()

const days = ref(7)
const showCharts = ref(true)
const filterAccount = ref('')
const filterModel = ref('')
const sortKey = ref<'timestamp' | 'account_email' | 'model_display' | 'total_tokens' | 'estimated_cost_usd'>('timestamp')
const sortAsc = ref(false)
const pageSize = 100
const currentPage = ref(1)

// Records filtered by day range (from store)
const dayRecords = computed(() => store.getRecords(days.value))

// Unique values for filters
const uniqueAccounts = computed(() => {
  const set = new Set(dayRecords.value.map(r => r.account_email))
  return Array.from(set).sort()
})

const uniqueModels = computed(() => {
  const set = new Set(dayRecords.value.map(r => r.model_display))
  return Array.from(set).sort()
})

// Filtered and sorted records
const allFiltered = computed(() => {
  let result = dayRecords.value

  if (filterAccount.value) {
    result = result.filter(r => r.account_email === filterAccount.value)
  }
  if (filterModel.value) {
    result = result.filter(r => r.model_display === filterModel.value)
  }

  result = [...result].sort((a, b) => {
    const key = sortKey.value
    let cmp = 0
    if (key === 'timestamp') {
      cmp = a.timestamp.localeCompare(b.timestamp)
    } else if (key === 'account_email') {
      cmp = a.account_email.localeCompare(b.account_email)
    } else if (key === 'model_display') {
      cmp = a.model_display.localeCompare(b.model_display)
    } else if (key === 'total_tokens') {
      cmp = a.total_tokens - b.total_tokens
    } else if (key === 'estimated_cost_usd') {
      cmp = a.estimated_cost_usd - b.estimated_cost_usd
    }
    return sortAsc.value ? cmp : -cmp
  })

  return result
})

const displayedRecords = computed(() => {
  return allFiltered.value.slice(0, currentPage.value * pageSize)
})

const hasMore = computed(() => displayedRecords.value.length < allFiltered.value.length)

// Summary stats
const totalCost = computed(() =>
  allFiltered.value.reduce((sum, r) => sum + r.estimated_cost_usd, 0)
)
const totalTokens = computed(() =>
  allFiltered.value.reduce((sum, r) => sum + r.total_tokens, 0)
)
const totalMessages = computed(() => allFiltered.value.length)

function toggleSort(key: typeof sortKey.value) {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = key
    sortAsc.value = false
  }
  currentPage.value = 1
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return n.toString()
}

function formatCost(n: number): string {
  if (n >= 1) return '$' + n.toFixed(2)
  if (n >= 0.01) return '$' + n.toFixed(3)
  return '$' + n.toFixed(4)
}

function formatTimestamp(ts: string): { date: string; time: string } {
  const d = new Date(ts)
  const now = new Date()
  const today = now.toISOString().slice(0, 10)
  const yesterday = new Date(now.getTime() - 86400000).toISOString().slice(0, 10)
  const dateStr = ts.slice(0, 10)
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })

  let date: string
  if (dateStr === today) {
    date = t('usage.today')
  } else if (dateStr === yesterday) {
    date = t('usage.yesterday')
  } else {
    date = d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }

  return { date, time }
}

function accountDisplay(r: CostUsageRecord): string {
  if (r.account_label) return r.account_label
  if (r.account_email === 'Unknown') return t('usage.unknown')
  const at = r.account_email.indexOf('@')
  return at > 0 ? r.account_email.substring(0, at) : r.account_email
}

function modelBadgeClass(model: string): string {
  const m = model.toLowerCase()
  if (m.includes('opus')) return 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300'
  if (m.includes('sonnet')) return 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300'
  if (m.includes('haiku')) return 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300'
  return 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300'
}

// On mount: ensure data is fresh (syncIfStale checks if >60s old)
onMounted(() => store.syncIfStale())
</script>

<template>
  <div class="space-y-5">
    <!-- Header -->
    <div class="flex items-center justify-between animate-fade-in-up">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">
        {{ t('usage.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <!-- Period selector -->
        <div class="flex items-center bg-gray-100 dark:bg-gray-800 rounded-lg p-0.5">
          <button
            v-for="d in [3, 7, 14, 30]"
            :key="d"
            @click="days = d; currentPage = 1"
            :class="[
              'px-3 py-1 rounded-md text-xs font-medium transition-colors',
              days === d
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200'
            ]"
          >
            {{ d }}{{ t('usage.days') }}
          </button>
        </div>

        <button
          @click="store.sync()"
          :disabled="store.syncing"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 disabled:opacity-50 transition-colors"
        >
          <RefreshCw :size="14" :class="{ 'animate-spin': store.syncing }" />
          {{ t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- Summary cards -->
    <div class="grid grid-cols-3 gap-3 animate-fade-in-up delay-1">
      <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <div class="flex items-center gap-2 mb-1">
          <DollarSign :size="14" class="text-emerald-500" />
          <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('usage.totalCost') }}</span>
        </div>
        <p class="text-lg font-bold text-gray-900 dark:text-white">{{ formatCost(totalCost) }}</p>
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <div class="flex items-center gap-2 mb-1">
          <Zap :size="14" class="text-amber-500" />
          <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('usage.totalTokens') }}</span>
        </div>
        <p class="text-lg font-bold text-gray-900 dark:text-white">{{ formatTokens(totalTokens) }}</p>
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <div class="flex items-center gap-2 mb-1">
          <MessageSquare :size="14" class="text-blue-500" />
          <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('usage.totalMessages') }}</span>
        </div>
        <p class="text-lg font-bold text-gray-900 dark:text-white">{{ totalMessages.toLocaleString() }}</p>
      </div>
    </div>

    <!-- Charts toggle + section -->
    <div class="flex items-center justify-between animate-fade-in-up delay-2">
      <button
        @click="showCharts = !showCharts"
        :class="[
          'flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors',
          showCharts
            ? 'bg-primary-100 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300'
            : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700'
        ]"
      >
        <BarChart3 :size="14" />
        {{ t('charts.toggle') }}
      </button>
    </div>

    <transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="opacity-0 -translate-y-2 max-h-0"
      enter-to-class="opacity-100 translate-y-0 max-h-[2000px]"
      leave-active-class="transition-all duration-200 ease-in"
      leave-from-class="opacity-100 translate-y-0 max-h-[2000px]"
      leave-to-class="opacity-0 -translate-y-2 max-h-0"
    >
      <CostCharts v-if="showCharts && allFiltered.length > 0" :records="allFiltered" />
    </transition>

    <!-- Filters -->
    <div class="flex items-center gap-3 animate-fade-in-up delay-3">
      <Filter :size="14" class="text-gray-400" />
      <select
        v-model="filterAccount"
        class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="">{{ t('usage.allAccounts') }}</option>
        <option v-for="acc in uniqueAccounts" :key="acc" :value="acc">{{ acc }}</option>
      </select>
      <select
        v-model="filterModel"
        class="px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="">{{ t('usage.allModels') }}</option>
        <option v-for="m in uniqueModels" :key="m" :value="m">{{ m }}</option>
      </select>
      <span class="text-xs text-gray-400 ml-auto">{{ allFiltered.length.toLocaleString() }} {{ t('usage.records') }}</span>
    </div>

    <!-- Table -->
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm overflow-hidden animate-fade-in-up delay-3">
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-gray-100 dark:border-gray-700">
              <th
                @click="toggleSort('timestamp')"
                class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 select-none"
              >
                <span class="flex items-center gap-1">
                  <Calendar :size="12" />
                  {{ t('usage.time') }}
                  <ArrowUpDown :size="12" v-if="sortKey === 'timestamp'" :class="sortAsc ? 'rotate-180' : ''" />
                </span>
              </th>
              <th
                @click="toggleSort('account_email')"
                class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 select-none"
              >
                <span class="flex items-center gap-1">
                  {{ t('usage.account') }}
                  <ArrowUpDown :size="12" v-if="sortKey === 'account_email'" />
                </span>
              </th>
              <th
                @click="toggleSort('model_display')"
                class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 select-none"
              >
                <span class="flex items-center gap-1">
                  {{ t('usage.model') }}
                  <ArrowUpDown :size="12" v-if="sortKey === 'model_display'" />
                </span>
              </th>
              <th
                @click="toggleSort('total_tokens')"
                class="text-right px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 select-none"
              >
                <span class="flex items-center justify-end gap-1">
                  {{ t('usage.tokens') }}
                  <ArrowUpDown :size="12" v-if="sortKey === 'total_tokens'" />
                </span>
              </th>
              <th
                @click="toggleSort('estimated_cost_usd')"
                class="text-right px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:text-gray-700 dark:hover:text-gray-200 select-none"
              >
                <span class="flex items-center justify-end gap-1">
                  {{ t('usage.cost') }}
                  <ArrowUpDown :size="12" v-if="sortKey === 'estimated_cost_usd'" />
                </span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(record, idx) in displayedRecords"
              :key="`${record.timestamp}-${record.session_id}-${idx}`"
              :class="[
                'border-b border-gray-50 dark:border-gray-700/50 hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors',
                idx % 2 === 0 ? '' : 'bg-gray-50/50 dark:bg-gray-800/50'
              ]"
            >
              <td class="px-4 py-2.5 whitespace-nowrap">
                <span class="text-gray-900 dark:text-gray-100 font-medium">{{ formatTimestamp(record.timestamp).time }}</span>
                <span class="text-xs text-gray-400 ml-1.5">{{ formatTimestamp(record.timestamp).date }}</span>
              </td>
              <td class="px-4 py-2.5">
                <span class="text-gray-700 dark:text-gray-300">{{ accountDisplay(record) }}</span>
              </td>
              <td class="px-4 py-2.5">
                <span :class="['inline-flex px-2 py-0.5 rounded-full text-xs font-medium', modelBadgeClass(record.model_display)]">
                  {{ record.model_display }}
                </span>
              </td>
              <td class="px-4 py-2.5 text-right tabular-nums">
                <span class="text-gray-900 dark:text-gray-100">{{ formatTokens(record.total_tokens) }}</span>
                <div class="text-[10px] text-gray-400 mt-0.5 space-x-1">
                  <span v-if="record.cache_read_tokens" class="text-blue-400">{{ formatTokens(record.cache_read_tokens) }} cache</span>
                  <span>{{ formatTokens(record.input_tokens) }} in</span>
                  <span class="text-orange-400">{{ formatTokens(record.output_tokens) }} out</span>
                </div>
              </td>
              <td class="px-4 py-2.5 text-right font-medium tabular-nums">
                <span :class="record.estimated_cost_usd >= 0.1 ? 'text-amber-600 dark:text-amber-400' : 'text-gray-500 dark:text-gray-400'">
                  {{ formatCost(record.estimated_cost_usd) }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Load more -->
      <div v-if="hasMore" class="py-3 text-center border-t border-gray-100 dark:border-gray-700">
        <button
          @click="currentPage++"
          class="inline-flex items-center gap-1 px-4 py-1.5 text-sm text-primary-600 dark:text-primary-400 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          <ChevronDown :size="14" />
          {{ t('usage.loadMore') }}
        </button>
      </div>

      <!-- Empty state -->
      <div v-if="!store.syncing && allFiltered.length === 0" class="py-12 text-center">
        <DollarSign :size="40" class="mx-auto mb-3 text-gray-300 dark:text-gray-600" />
        <p class="text-sm text-gray-500 dark:text-gray-400">{{ t('usage.noData') }}</p>
      </div>

      <!-- Loading state (only on first load) -->
      <div v-if="store.syncing && dayRecords.length === 0" class="py-12 text-center">
        <RefreshCw :size="24" class="mx-auto mb-3 text-primary-500 animate-spin" />
        <p class="text-sm text-gray-500 dark:text-gray-400">{{ t('common.loading') }}</p>
      </div>
    </div>

    <!-- Footer note -->
    <p class="text-xs text-gray-400 dark:text-gray-500 text-center animate-fade-in-up delay-5">
      {{ t('usage.costNote') }}
    </p>
  </div>
</template>
