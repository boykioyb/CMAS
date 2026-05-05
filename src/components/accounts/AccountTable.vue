<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { useUiStore } from '@/stores/uiStore'
import type { Account, Project } from '@/types'
import ProjectPicker from '@/components/projects/ProjectPicker.vue'
import { Edit3, Trash2, ExternalLink, RefreshCw, ShieldCheck, LogIn } from 'lucide-vue-next'

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
  reauth: [account: Account]
}>()

function progressColor(percent: number): string {
  if (percent < 50) return 'bg-blue-500'
  if (percent < 80) return 'bg-amber-500'
  return 'bg-red-500'
}


async function pickProject(accountId: string, project: Project) {
  try {
    await accountStore.linkProject(accountId, project.id)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function unlinkProject(accountId: string, projectId: string) {
  try {
    await accountStore.unlinkProject(accountId, projectId)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function selectProjectId(accountId: string, projectId: string | null) {
  try {
    await accountStore.setSelectedProjectId(accountId, projectId)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function checkHealth(accountId: string) {
  try {
    const result = await accountStore.checkTokenHealth(accountId)
    if (result.valid) {
      const org = result.organization_name ? ` (${result.organization_name})` : ''
      if (result.status === 'refreshed') {
        uiStore.showToast('success', t('accounts.tokenRefreshed'))
      } else {
        uiStore.showToast('success', t('accounts.tokenOk', { org }))
      }
    } else if (result.status === 'expired' || result.status === 'auth_error') {
      uiStore.showToast('error', t('accounts.tokenRefreshFailed'))
    } else {
      uiStore.showToast('error', result.error_message || `Token ${result.status}`)
    }
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function handleSyncUsage(accountId: string) {
  try {
    const data = await accountStore.fetchAccountUsage(accountId)
    if (data.success) {
      uiStore.showToast('success', t('common.success'))
    } else if (data.error_message?.includes('Rate limited')) {
      uiStore.showToast('warning', t('accounts.rateLimited', { count: 1 }))
    } else if (!data.authenticated) {
      uiStore.showToast('error', t('accounts.tokenRefreshFailed'))
    } else {
      uiStore.showToast('warning', data.error_message || t('common.error'))
    }
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}
</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm overflow-hidden animate-fade-in-up">
    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 dark:border-gray-700">
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('accounts.email') }}</th>
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('accounts.project') }}</th>
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('accounts.usageColumn') }}</th>
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('common.status') }}</th>
            <th class="text-right px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('common.actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="account in accounts"
            :key="account.id"
            class="border-b border-gray-50 dark:border-gray-700/50 hover:bg-gray-50/50 dark:hover:bg-gray-700/30 transition-colors"
          >
            <!-- Email -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center shrink-0">
                  <span class="text-xs font-bold text-primary-600 dark:text-primary-400">{{ account.email[0].toUpperCase() }}</span>
                </div>
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {{ account.label || account.email }}
                    </span>
                    <span v-if="account.is_active" class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300">
                      {{ t('accounts.active') }}
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
                  <div v-if="account.label" class="text-xs text-gray-500 truncate">{{ account.email }}</div>
                </div>
              </div>
            </td>

            <!-- Project selector -->
            <td class="px-4 py-3">
              <ProjectPicker
                :account-id="account.id"
                :linked-ids="account.project_ids"
                :selected-id="account.selected_project_id ?? null"
                @pick="(p) => pickProject(account.id, p)"
                @unlink="(id) => unlinkProject(account.id, id)"
                @selection-change="(id) => selectProjectId(account.id, id)"
              />
            </td>

            <!-- Usage bars -->
            <td class="px-4 py-3">
              <template v-if="accountStore.getAccountRealUsage(account.id)?.success">
                <div class="space-y-1.5 min-w-[160px]">
                  <div v-if="accountStore.getAccountRealUsage(account.id)!.session_percent != null">
                    <div class="flex items-center justify-between mb-0.5">
                      <span class="text-[10px] text-gray-500">{{ t('accounts.sessionLabel') }}</span>
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
                  <div v-if="accountStore.getAccountRealUsage(account.id)!.weekly_all_percent != null">
                    <div class="flex items-center justify-between mb-0.5">
                      <span class="text-[10px] text-gray-500">{{ t('accounts.weeklyLabel') }}</span>
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
                </div>
              </template>
              <div v-else-if="accountStore.isFetchingUsage(account.id)" class="flex items-center gap-1 text-[10px] text-gray-400">
                <RefreshCw :size="10" class="animate-spin" />
              </div>
              <button
                v-else
                @click="handleSyncUsage(account.id)"
                class="text-[10px] text-primary-500 hover:text-primary-600"
              >
                {{ t('accounts.syncUsage') }}
              </button>
            </td>

            <!-- Status -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-1.5">
                <span :class="[
                  'inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium',
                  account.usage.is_rate_limited ? 'bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300' :
                  account.status === 'expired' ? 'bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300' :
                  account.status === 'ok' ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300' :
                  'bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300'
                ]">
                  <span :class="[
                    'w-1.5 h-1.5 rounded-full',
                    account.usage.is_rate_limited ? 'bg-red-500' :
                    account.status === 'expired' ? 'bg-amber-500' :
                    account.status === 'ok' ? 'bg-emerald-500' : 'bg-red-500'
                  ]" />
                  {{
                    account.usage.is_rate_limited ? t('dashboard.rateLimited') :
                    account.status === 'expired' ? t('accounts.expired') :
                    account.status === 'ok' ? t('dashboard.active') :
                    t('common.error')
                  }}
                </span>
              </div>
            </td>

            <!-- Actions -->
            <td class="px-4 py-3">
              <div class="flex items-center justify-end gap-1">
                <button
                  @click="handleSyncUsage(account.id)"
                  :disabled="accountStore.isFetchingUsage(account.id)"
                  class="cursor-pointer p-1.5 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors disabled:opacity-50"
                  :title="t('accounts.syncUsage')"
                >
                  <RefreshCw :size="15" :class="{ 'animate-spin': accountStore.isFetchingUsage(account.id) }" />
                </button>
                <button
                  @click="checkHealth(account.id)"
                  :disabled="accountStore.isHealthChecking(account.id)"
                  class="cursor-pointer p-1.5 rounded-lg text-gray-400 hover:text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 transition-colors disabled:opacity-50"
                  :title="t('accounts.checkToken')"
                >
                  <ShieldCheck :size="15" :class="{ 'animate-pulse': accountStore.isHealthChecking(account.id) }" />
                </button>
                <button
                  v-if="account.status === 'expired' || account.status === 'error'"
                  @click="emit('reauth', account)"
                  class="cursor-pointer p-1.5 rounded-lg text-amber-500 hover:text-amber-600 hover:bg-amber-50 dark:hover:bg-amber-900/20 transition-colors"
                  :title="t('accounts.reauth')"
                >
                  <LogIn :size="15" />
                </button>
                <button
                  @click="emit('switchVscode', account.id)"
                  :disabled="!account.project_ids.length"
                  :class="[
                    'p-1.5 rounded-lg transition-colors',
                    account.project_ids.length
                      ? 'text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20'
                      : 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
                  ]"
                  :title="account.project_ids.length ? t('accounts.switchAndVscode') : t('accounts.addProjectFirst')"
                >
                  <ExternalLink :size="15" />
                </button>
                <button
                  @click="emit('edit', account)"
                  class="p-1.5 rounded-lg text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  :title="t('accounts.editLabel')"
                >
                  <Edit3 :size="15" />
                </button>
                <button
                  @click="emit('delete', account.id)"
                  class="p-1.5 rounded-lg text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                  :title="t('accounts.deleteAccount')"
                >
                  <Trash2 :size="15" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
