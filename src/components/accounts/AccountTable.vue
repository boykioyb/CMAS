<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { useUiStore } from '@/stores/uiStore'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { Account } from '@/types'
import { Edit3, Trash2, ExternalLink, FolderPlus, X, RefreshCw, ShieldCheck } from 'lucide-vue-next'

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


async function addProject(accountId: string) {
  const folder = await openDialog({ directory: true, multiple: false, title: t('accounts.selectFolder') })
  if (!folder) return
  try {
    await accountStore.addProject(accountId, folder as string)
    uiStore.showToast('success', t('common.success'))
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function removeProject(accountId: string, index: number) {
  try {
    await accountStore.removeProject(accountId, index)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function selectProject(accountId: string, index: number) {
  await accountStore.setSelectedProject(accountId, index)
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
  <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm overflow-hidden animate-fade-in-up">
    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-100 dark:border-gray-700">
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('accounts.email') }}</th>
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">{{ t('accounts.project') }}</th>
            <th class="text-left px-4 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">Usage</th>
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
              <div class="flex items-center gap-1.5 min-w-[180px]">
                <select
                  v-if="account.projects.length > 0"
                  :value="account.selected_project ?? ''"
                  @change="selectProject(account.id, Number(($event.target as HTMLSelectElement).value))"
                  class="flex-1 px-2 py-1 text-xs border border-gray-200 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-1 focus:ring-primary-500 outline-none truncate"
                >
                  <option v-for="(proj, idx) in account.projects" :key="idx" :value="idx">
                    {{ proj.name }}
                  </option>
                </select>
                <span v-else class="text-xs text-gray-400 italic">{{ t('accounts.noProjects') }}</span>
                <button
                  @click="addProject(account.id)"
                  class="p-1 rounded text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 hover:text-primary-500 transition-colors shrink-0"
                  :title="t('accounts.addProject')"
                >
                  <FolderPlus :size="14" />
                </button>
                <button
                  v-if="account.projects.length > 0 && account.selected_project != null"
                  @click="removeProject(account.id, account.selected_project!)"
                  class="p-1 rounded text-gray-400 hover:bg-red-50 dark:hover:bg-red-900/20 hover:text-red-500 transition-colors shrink-0"
                  :title="t('common.delete')"
                >
                  <X :size="14" />
                </button>
              </div>
            </td>

            <!-- Usage bars -->
            <td class="px-4 py-3">
              <template v-if="accountStore.getAccountRealUsage(account.id)?.success">
                <div class="space-y-1.5 min-w-[160px]">
                  <div v-if="accountStore.getAccountRealUsage(account.id)!.session_percent != null">
                    <div class="flex items-center justify-between mb-0.5">
                      <span class="text-[10px] text-gray-500">Session</span>
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
                      <span class="text-[10px] text-gray-500">Weekly</span>
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
                @click="accountStore.fetchAccountUsage(account.id)"
                class="text-[10px] text-primary-500 hover:text-primary-600"
              >
                Sync
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
                    account.status === 'expired' ? 'Expired' :
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
                  @click="accountStore.fetchAccountUsage(account.id)"
                  :disabled="accountStore.isFetchingUsage(account.id)"
                  class="cursor-pointer p-1.5 rounded-lg text-gray-400 hover:text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors disabled:opacity-50"
                  title="Sync usage"
                >
                  <RefreshCw :size="15" :class="{ 'animate-spin': accountStore.isFetchingUsage(account.id) }" />
                </button>
                <button
                  @click="checkHealth(account.id)"
                  :disabled="accountStore.isHealthChecking(account.id)"
                  class="cursor-pointer p-1.5 rounded-lg text-gray-400 hover:text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 transition-colors disabled:opacity-50"
                  title="Check token"
                >
                  <ShieldCheck :size="15" :class="{ 'animate-pulse': accountStore.isHealthChecking(account.id) }" />
                </button>
                <button
                  @click="emit('switchVscode', account.id)"
                  :disabled="!account.projects.length"
                  :class="[
                    'p-1.5 rounded-lg transition-colors',
                    account.projects.length
                      ? 'text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20'
                      : 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
                  ]"
                  :title="account.projects.length ? t('accounts.switchAndVscode') : t('accounts.addProjectFirst')"
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
