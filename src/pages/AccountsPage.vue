<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { useConfigStore } from '@/stores/configStore'
import { useUiStore } from '@/stores/uiStore'
import { Plus, RefreshCw, List, LayoutGrid, Search, Users } from 'lucide-vue-next'
import AccountTable from '@/components/accounts/AccountTable.vue'
import AccountGrid from '@/components/accounts/AccountGrid.vue'
import AddAccountDialog from '@/components/accounts/AddAccountDialog.vue'
import EditAccountDialog from '@/components/accounts/EditAccountDialog.vue'
import AppDialog from '@/components/common/AppDialog.vue'
import type { Account } from '@/types'

const { t } = useI18n()
const accountStore = useAccountStore()
const configStore = useConfigStore()
const uiStore = useUiStore()

const showAddDialog = ref(false)
const showEditDialog = ref(false)
const showDeleteDialog = ref(false)
const editingAccount = ref<Account | null>(null)
const deletingId = ref<string | null>(null)
const refreshing = ref(false)


const filteredAccounts = computed(() => {
  let list = accountStore.sortedAccounts

  // Search
  if (uiStore.searchQuery) {
    const q = uiStore.searchQuery.toLowerCase()
    list = list.filter(a => a.email.toLowerCase().includes(q) || (a.label && a.label.toLowerCase().includes(q)))
  }

  // Filter
  if (uiStore.accountFilter !== 'all') {
    list = list.filter(a => {
      if (uiStore.accountFilter === 'good') return !a.usage.is_rate_limited && a.usage.messages_today < 100
      if (uiStore.accountFilter === 'low') return !a.usage.is_rate_limited && a.usage.messages_today >= 100
      if (uiStore.accountFilter === 'empty') return a.usage.is_rate_limited
      return true
    })
  }

  return list
})

async function handleSwitchVscode(id: string) {
  try {
    // Uses the account's selected project + configured VSCode path
    const result = await accountStore.switchAndOpenVscode(id, undefined, configStore.config.vscode_path || undefined)
    if (result.success) {
      uiStore.showToast('success', t('switch.success'))
    }
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

function handleEdit(account: Account) {
  editingAccount.value = account
  showEditDialog.value = true
}

function handleDeleteConfirm(id: string) {
  deletingId.value = id
  showDeleteDialog.value = true
}

async function handleDelete() {
  if (!deletingId.value) return
  try {
    await accountStore.removeAccount(deletingId.value)
    uiStore.showToast('success', t('common.success'))
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
  showDeleteDialog.value = false
  deletingId.value = null
}

async function handleRefreshAll() {
  refreshing.value = true
  try {
    await Promise.all([
      accountStore.fetchAccounts(),
      accountStore.fetchAllAccountUsage(),
    ])
    uiStore.showToast('success', t('common.success'))
  } finally {
    refreshing.value = false
  }
}

const filters = computed(() => [
  { key: 'all' as const, label: t('common.all'), count: accountStore.accounts.length },
  { key: 'good' as const, label: t('common.good') },
  { key: 'low' as const, label: t('common.low') },
  { key: 'empty' as const, label: t('common.empty') },
])
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between animate-fade-in-up">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">{{ t('accounts.title') }}</h1>
      <div class="flex items-center gap-2">
        <button
          @click="showAddDialog = true"
          class="cursor-pointer flex items-center gap-2 px-3 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 transition-colors"
        >
          <Plus :size="16" />
          {{ t('accounts.addAccount') }}
        </button>
        <button
          @click="handleRefreshAll"
          :disabled="refreshing"
          :title="t('accounts.refreshAll')"
          class="cursor-pointer p-2 rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
        >
          <RefreshCw :size="16" :class="{ 'animate-spin': refreshing }" class="transition-transform" />
        </button>
      </div>
    </div>

    <!-- Toolbar: search, filters, view toggle -->
    <div class="flex items-center gap-3 flex-wrap animate-fade-in-up delay-1">
      <!-- Search -->
      <div class="relative flex-1 min-w-[200px] max-w-sm">
        <Search :size="16" class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          v-model="uiStore.searchQuery"
          :placeholder="t('accounts.searchPlaceholder')"
          class="w-full pl-9 pr-3 py-2 border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none transition"
        />
      </div>

      <!-- Filters -->
      <div class="flex items-center bg-gray-100 dark:bg-gray-800 rounded-lg p-0.5">
        <button
          v-for="f in filters"
          :key="f.key"
          @click="uiStore.accountFilter = f.key"
          :class="[
            'px-3 py-1.5 rounded-md text-xs font-medium transition-colors',
            uiStore.accountFilter === f.key
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
          ]"
        >
          {{ f.label }}
          <span v-if="f.count !== undefined" class="ml-1 text-gray-400">({{ f.count }})</span>
        </button>
      </div>

      <!-- View toggle -->
      <div class="flex items-center bg-gray-100 dark:bg-gray-800 rounded-lg p-0.5">
        <button
          @click="uiStore.viewMode = 'table'"
          :class="[
            'p-1.5 rounded-md transition-colors',
            uiStore.viewMode === 'table'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm'
              : 'text-gray-400 hover:text-gray-600'
          ]"
        >
          <List :size="16" />
        </button>
        <button
          @click="uiStore.viewMode = 'grid'"
          :class="[
            'p-1.5 rounded-md transition-colors',
            uiStore.viewMode === 'grid'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm'
              : 'text-gray-400 hover:text-gray-600'
          ]"
        >
          <LayoutGrid :size="16" />
        </button>
      </div>
    </div>

    <!-- Loading skeleton -->
    <div v-if="accountStore.loading && filteredAccounts.length === 0" class="space-y-3 animate-pulse">
      <div v-for="i in 3" :key="i" class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 flex items-center gap-4">
        <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 shrink-0" />
        <div class="flex-1 space-y-2">
          <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/3" />
          <div class="h-2 bg-gray-100 dark:bg-gray-700/50 rounded w-1/4" />
        </div>
        <div class="w-24 space-y-1.5">
          <div class="h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full" />
          <div class="h-1.5 bg-gray-100 dark:bg-gray-700/50 rounded-full w-3/4" />
        </div>
      </div>
    </div>

    <!-- Content -->
    <template v-else-if="filteredAccounts.length > 0">
      <AccountTable
        v-if="uiStore.viewMode === 'table'"
        :accounts="filteredAccounts"
        @switch-vscode="handleSwitchVscode"
        @refresh="() => {}"
        @edit="handleEdit"
        @delete="handleDeleteConfirm"
      />
      <AccountGrid
        v-else
        :accounts="filteredAccounts"
        @switch-vscode="handleSwitchVscode"
        @edit="handleEdit"
        @delete="handleDeleteConfirm"
      />
    </template>

    <!-- Empty state -->
    <div v-else class="text-center py-16">
      <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center">
        <Users :size="24" class="text-gray-400" />
      </div>
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">{{ t('accounts.empty.title') }}</h3>
      <p class="text-sm text-gray-500 mb-4">{{ t('accounts.empty.description') }}</p>
      <button
        @click="showAddDialog = true"
        class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 transition-colors"
      >
        <Plus :size="16" />
        {{ t('accounts.addAccount') }}
      </button>
    </div>

    <!-- Dialogs -->
    <AddAccountDialog :open="showAddDialog" @close="showAddDialog = false" />
    <EditAccountDialog :open="showEditDialog" :account="editingAccount" @close="showEditDialog = false" />

    <!-- Delete confirmation -->
    <AppDialog :open="showDeleteDialog" :title="t('accounts.confirmDelete')" @close="showDeleteDialog = false">
      <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">{{ t('accounts.confirmDeleteDesc') }}</p>
      <div class="flex gap-2">
        <button
          @click="showDeleteDialog = false"
          class="flex-1 px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        >{{ t('common.cancel') }}</button>
        <button
          @click="handleDelete"
          class="flex-1 px-4 py-2 rounded-lg bg-red-500 text-white text-sm font-medium hover:bg-red-600 transition-colors"
        >{{ t('common.delete') }}</button>
      </div>
    </AppDialog>
  </div>
</template>
