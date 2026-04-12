<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { useUiStore } from '@/stores/uiStore'
import type { Account } from '@/types'
import { Edit3, Trash2, ExternalLink, RefreshCw, ShieldCheck } from 'lucide-vue-next'

const { t } = useI18n()
const accountStore = useAccountStore()
const uiStore = useUiStore()

defineProps<{
  accounts: Account[]
}>()

const emit = defineEmits<{
  switchVscode: [id: string]
  edit: [account: Account]
  delete: [id: string]
}>()

function progressColor(percent: number): string {
  if (percent < 50) return 'bg-blue-500'
  if (percent < 80) return 'bg-amber-500'
  return 'bg-red-500'
}

async function checkHealth(accountId: string) {
  try {
    const result = await accountStore.checkTokenHealth(accountId)
    if (result.valid) {
      const org = result.organization_name ? ` (${result.organization_name})` : ''
      uiStore.showToast('success', `Token OK${org}`)
    } else {
      uiStore.showToast('error', result.error_message || `Token ${result.status}`)
    }
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
    <div
      v-for="(account, idx) in accounts"
      :key="account.id"
      :class="[
        'bg-white dark:bg-gray-800 rounded-xl p-4 border shadow-sm card-hover animate-fade-in-up',
        account.is_active ? 'border-primary-300 dark:border-primary-600 ring-1 ring-primary-100 dark:ring-primary-900/30' : 'border-gray-100 dark:border-gray-700',
        `delay-${Math.min(idx + 1, 8)}`
      ]"
    >
      <!-- Header -->
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-2 min-w-0">
          <div class="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center shrink-0">
            <span class="text-xs font-bold text-primary-600 dark:text-primary-400">{{ account.email[0].toUpperCase() }}</span>
          </div>
          <div class="min-w-0">
            <div class="text-sm font-medium text-gray-900 dark:text-white truncate">
              {{ account.label || account.email }}
            </div>
            <div v-if="account.label" class="text-xs text-gray-500 truncate">{{ account.email }}</div>
          </div>
        </div>
        <div class="flex items-center gap-1 shrink-0">
          <span v-if="account.is_active" class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300">
            {{ t('accounts.active') }}
          </span>
          <span v-if="account.status === 'expired'" class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300">
            Expired
          </span>
          <span v-else-if="account.status === 'error'" class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300">
            Error
          </span>
          <span :class="[
            'px-1.5 py-0.5 rounded text-[10px] font-bold',
            account.plan === 'pro'
              ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
              : 'bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400'
          ]">
            {{ account.plan === 'pro' ? 'Pro' : 'Free' }}
          </span>
        </div>
      </div>

      <!-- Project -->
      <div v-if="account.projects.length > 0" class="mb-3">
        <div class="text-[10px] uppercase text-gray-400 font-semibold mb-1">{{ t('accounts.project') }}</div>
        <div class="text-xs text-gray-700 dark:text-gray-300 truncate">
          {{ account.projects[account.selected_project ?? 0]?.name || '-' }}
        </div>
      </div>

      <!-- Usage bars -->
      <div class="space-y-2.5 mb-4">
        <template v-if="accountStore.getAccountRealUsage(account.id)?.success">
          <!-- Session -->
          <div v-if="accountStore.getAccountRealUsage(account.id)!.session_percent != null">
            <div class="flex items-center justify-between mb-1">
              <span class="text-[10px] font-medium text-gray-500 dark:text-gray-400">Session</span>
              <span class="text-[10px] font-semibold text-gray-700 dark:text-gray-300">
                {{ accountStore.getAccountRealUsage(account.id)!.session_percent }}%
                <span v-if="accountStore.getAccountRealUsage(account.id)!.session_reset" class="font-normal text-gray-400">({{ accountStore.getAccountRealUsage(account.id)!.session_reset }})</span>
              </span>
            </div>
            <div class="w-full h-1.5 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden">
              <div
                :class="['h-full rounded-full transition-all duration-500', progressColor(accountStore.getAccountRealUsage(account.id)!.session_percent!)]"
                :style="{ width: Math.max(2, accountStore.getAccountRealUsage(account.id)!.session_percent!) + '%' }"
              />
            </div>
          </div>
          <!-- Weekly -->
          <div v-if="accountStore.getAccountRealUsage(account.id)!.weekly_all_percent != null">
            <div class="flex items-center justify-between mb-1">
              <span class="text-[10px] font-medium text-gray-500 dark:text-gray-400">Weekly</span>
              <span class="text-[10px] font-semibold text-gray-700 dark:text-gray-300">
                {{ accountStore.getAccountRealUsage(account.id)!.weekly_all_percent }}%
                <span v-if="accountStore.getAccountRealUsage(account.id)!.weekly_reset" class="font-normal text-gray-400">({{ accountStore.getAccountRealUsage(account.id)!.weekly_reset }})</span>
              </span>
            </div>
            <div class="w-full h-1.5 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden">
              <div
                :class="['h-full rounded-full transition-all duration-500', progressColor(accountStore.getAccountRealUsage(account.id)!.weekly_all_percent!)]"
                :style="{ width: Math.max(2, accountStore.getAccountRealUsage(account.id)!.weekly_all_percent!) + '%' }"
              />
            </div>
          </div>
          <!-- Sonnet -->
          <div v-if="accountStore.getAccountRealUsage(account.id)!.weekly_sonnet_percent != null">
            <div class="flex items-center justify-between mb-1">
              <span class="text-[10px] font-medium text-gray-500 dark:text-gray-400">Sonnet</span>
              <span class="text-[10px] font-semibold text-gray-700 dark:text-gray-300">{{ accountStore.getAccountRealUsage(account.id)!.weekly_sonnet_percent }}%</span>
            </div>
            <div class="w-full h-1.5 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden">
              <div
                :class="['h-full rounded-full transition-all duration-500', progressColor(accountStore.getAccountRealUsage(account.id)!.weekly_sonnet_percent!)]"
                :style="{ width: Math.max(2, accountStore.getAccountRealUsage(account.id)!.weekly_sonnet_percent!) + '%' }"
              />
            </div>
          </div>
        </template>

        <!-- Loading -->
        <div v-else-if="accountStore.isFetchingUsage(account.id)" class="flex items-center gap-1.5 py-2 text-[10px] text-gray-400">
          <RefreshCw :size="10" class="animate-spin" />
          Loading...
        </div>

        <!-- Error / no data -->
        <div v-else class="text-[10px] text-gray-400 py-1">
          <span v-if="accountStore.getAccountRealUsage(account.id)?.error_message" class="text-red-400">
            {{ accountStore.getAccountRealUsage(account.id)!.error_message }}
          </span>
          <button
            v-else
            @click="accountStore.fetchAccountUsage(account.id)"
            class="text-primary-500 hover:text-primary-600"
          >
            Sync usage
          </button>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-1 pt-3 border-t border-gray-100 dark:border-gray-700">
        <button
          @click="emit('switchVscode', account.id)"
          class="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg text-xs font-medium text-emerald-600 dark:text-emerald-400 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 transition-colors"
        >
          <ExternalLink :size="13" />
          VSCode
        </button>
        <button
          @click="accountStore.fetchAccountUsage(account.id)"
          :disabled="accountStore.isFetchingUsage(account.id)"
          class="cursor-pointer p-1.5 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors disabled:opacity-50"
          title="Sync usage"
        >
          <RefreshCw :size="13" :class="{ 'animate-spin': accountStore.isFetchingUsage(account.id) }" />
        </button>
        <button
          @click="checkHealth(account.id)"
          :disabled="accountStore.isHealthChecking(account.id)"
          class="cursor-pointer p-1.5 rounded-lg text-gray-400 hover:text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 transition-colors disabled:opacity-50"
          title="Check token"
        >
          <ShieldCheck :size="13" :class="{ 'animate-pulse': accountStore.isHealthChecking(account.id) }" />
        </button>
        <button
          @click="emit('edit', account)"
          class="p-1.5 rounded-lg text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
        >
          <Edit3 :size="13" />
        </button>
        <button
          @click="emit('delete', account.id)"
          class="p-1.5 rounded-lg text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
        >
          <Trash2 :size="13" />
        </button>
      </div>
    </div>
  </div>
</template>
