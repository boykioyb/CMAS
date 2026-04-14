import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Account, AccountUpdate, QuotaSummary, SwitchResult, UsageInfo, TokenHealthResult, RealUsageData, TokenSyncResult } from '@/types'

export const useAccountStore = defineStore('accounts', () => {
  const accounts = ref<Account[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const activeAccount = computed(() => accounts.value.find(a => a.is_active))

  const sortedAccounts = computed(() => {
    return [...accounts.value].sort((a, b) => {
      if (a.is_active) return -1
      if (b.is_active) return 1
      return new Date(b.last_used_at || b.added_at).getTime() - new Date(a.last_used_at || a.added_at).getTime()
    })
  })

  const bestAccount = computed(() => {
    return accounts.value
      .filter(a => !a.is_active && a.status === 'ok' && !a.usage.is_rate_limited)
      .sort((a, b) => {
        return a.usage.messages_today - b.usage.messages_today
      })[0] || null
  })

  async function fetchAccounts() {
    loading.value = true
    error.value = null
    try {
      // Fast load: just accounts without scanning JSONL
      accounts.value = await invoke<Account[]>('list_accounts')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
    // Then refresh usage in background (non-blocking)
    refreshAllUsage()
  }

  async function refreshAllUsage() {
    try {
      accounts.value = await invoke<Account[]>('refresh_all_usage')
    } catch {
      // Silently fail — usage will just show stale/zero data
    }
  }

  async function addCurrentAccount(label?: string) {
    const account = await invoke<Account>('add_current_account', { label: label || null })
    accounts.value.push(account)
    return account
  }

  async function updateAccount(id: string, update: AccountUpdate) {
    const updated = await invoke<Account>('update_account', { id, update })
    const idx = accounts.value.findIndex(a => a.id === id)
    if (idx >= 0) accounts.value[idx] = updated
    return updated
  }

  async function removeAccount(id: string) {
    await invoke('remove_account', { id })
    accounts.value = accounts.value.filter(a => a.id !== id)
  }

  async function switchAccount(targetId: string): Promise<SwitchResult> {
    const result = await invoke<SwitchResult>('switch_account', { targetId })
    if (result.success) {
      await fetchAccounts()
    }
    return result
  }

  async function switchAndOpenVscode(targetId: string, folderPath?: string, vscodePath?: string): Promise<SwitchResult> {
    const result = await invoke<SwitchResult>('switch_and_open_vscode', {
      targetId,
      vscodePath: vscodePath || null,
      folderPath: folderPath || null,
    })
    if (result.success) {
      await fetchAccounts()
    }
    return result
  }

  async function switchToBest(): Promise<SwitchResult> {
    const result = await invoke<SwitchResult>('switch_to_best_account')
    if (result.success) {
      await fetchAccounts()
    }
    return result
  }

  async function refreshUsage(id: string): Promise<UsageInfo> {
    const usage = await invoke<UsageInfo>('get_usage_info', { id })
    const idx = accounts.value.findIndex(a => a.id === id)
    if (idx >= 0) accounts.value[idx].usage = usage
    return usage
  }

  async function addProject(accountId: string, path: string): Promise<Account> {
    const updated = await invoke<Account>('add_project_to_account', { accountId, path })
    const idx = accounts.value.findIndex(a => a.id === accountId)
    if (idx >= 0) accounts.value[idx] = updated
    return updated
  }

  async function removeProject(accountId: string, projectIndex: number): Promise<Account> {
    const updated = await invoke<Account>('remove_project_from_account', { accountId, projectIndex })
    const idx = accounts.value.findIndex(a => a.id === accountId)
    if (idx >= 0) accounts.value[idx] = updated
    return updated
  }

  async function setSelectedProject(accountId: string, projectIndex: number | null) {
    await invoke('set_selected_project', { accountId, projectIndex })
    const idx = accounts.value.findIndex(a => a.id === accountId)
    if (idx >= 0) {
      accounts.value[idx].selected_project = projectIndex ?? undefined
    }
  }

  async function getQuotaSummary(): Promise<QuotaSummary> {
    return invoke<QuotaSummary>('get_quota_summary')
  }

  const healthChecking = ref<Set<string>>(new Set())

  async function checkTokenHealth(accountId: string): Promise<TokenHealthResult> {
    healthChecking.value.add(accountId)
    try {
      const result = await invoke<TokenHealthResult>('check_account_token', { accountId })
      // Update local account status to match API result
      const idx = accounts.value.findIndex(a => a.id === accountId)
      if (idx >= 0) {
        accounts.value[idx].status = result.valid ? 'ok' : (result.status === 'expired' ? 'expired' : 'error')
      }
      return result
    } finally {
      healthChecking.value.delete(accountId)
    }
  }

  function isHealthChecking(accountId: string): boolean {
    return healthChecking.value.has(accountId)
  }

  async function checkAllTokenHealth(): Promise<void> {
    await Promise.allSettled(accounts.value.map(a => checkTokenHealth(a.id)))
  }

  const realUsage = ref<RealUsageData | null>(null)
  const scrapingUsage = ref(false)
  // Per-account real usage data (keyed by account ID)
  const accountRealUsage = ref<Record<string, RealUsageData>>({})
  const fetchingUsageIds = ref<Set<string>>(new Set())
  let lastUsageFetchTime = 0

  async function scrapeClaudeUsage(): Promise<RealUsageData> {
    scrapingUsage.value = true
    try {
      const data = await invoke<RealUsageData>('scrape_claude_usage')
      realUsage.value = data
      // Also store for active account
      const active = activeAccount.value
      if (active && data.success) {
        accountRealUsage.value[active.id] = data
      }
      return data
    } finally {
      scrapingUsage.value = false
    }
  }

  async function fetchAccountUsage(accountId: string): Promise<RealUsageData> {
    fetchingUsageIds.value.add(accountId)
    try {
      const data = await invoke<RealUsageData>('fetch_account_usage', { accountId })
      accountRealUsage.value[accountId] = data
      return data
    } finally {
      fetchingUsageIds.value.delete(accountId)
    }
  }

  function isFetchingUsage(accountId: string): boolean {
    return fetchingUsageIds.value.has(accountId)
  }

  function getAccountRealUsage(accountId: string): RealUsageData | null {
    return accountRealUsage.value[accountId] ?? null
  }

  async function fetchAllAccountUsage(): Promise<void> {
    lastUsageFetchTime = Date.now()
    await Promise.allSettled(
      accounts.value.map(a => fetchAccountUsage(a.id))
    )
  }

  /** Only fetch if no data yet or data is older than 30s */
  async function fetchAllAccountUsageIfStale(): Promise<void> {
    const hasAnyData = Object.keys(accountRealUsage.value).length > 0
    const isStale = Date.now() - lastUsageFetchTime > 30_000
    if (!hasAnyData || isStale) {
      await fetchAllAccountUsage()
    }
  }

  async function openClaudeLogin(): Promise<void> {
    await invoke('open_claude_login')
  }

  /** Sync active credentials + check all tokens + auto-refresh expired ones */
  async function syncAndCheckAllTokens(): Promise<TokenSyncResult[]> {
    try {
      const results = await invoke<TokenSyncResult[]>('sync_and_check_all_tokens')
      // Update local account statuses
      for (const result of results) {
        const idx = accounts.value.findIndex(a => a.id === result.account_id)
        if (idx >= 0) {
          accounts.value[idx].status = result.status === 'ok' ? 'ok' : (result.status === 'expired' ? 'expired' : 'error')
        }
      }
      return results
    } catch (e) {
      console.error('syncAndCheckAllTokens failed:', e)
      return []
    }
  }

  /** Manually refresh a specific account's token */
  async function refreshAccountToken(accountId: string): Promise<TokenSyncResult> {
    healthChecking.value.add(accountId)
    try {
      const result = await invoke<TokenSyncResult>('refresh_account_token', { accountId })
      const idx = accounts.value.findIndex(a => a.id === accountId)
      if (idx >= 0) {
        accounts.value[idx].status = result.status === 'ok' ? 'ok' : (result.status === 'expired' ? 'expired' : 'error')
      }
      return result
    } finally {
      healthChecking.value.delete(accountId)
    }
  }

  return {
    accounts,
    loading,
    error,
    activeAccount,
    sortedAccounts,
    bestAccount,
    fetchAccounts,
    refreshAllUsage,
    addCurrentAccount,
    updateAccount,
    removeAccount,
    switchAccount,
    switchAndOpenVscode,
    switchToBest,
    refreshUsage,
    addProject,
    removeProject,
    setSelectedProject,
    getQuotaSummary,
    healthChecking,
    checkTokenHealth,
    isHealthChecking,
    checkAllTokenHealth,
    realUsage,
    scrapingUsage,
    scrapeClaudeUsage,
    accountRealUsage,
    fetchingUsageIds,
    fetchAccountUsage,
    isFetchingUsage,
    getAccountRealUsage,
    fetchAllAccountUsage,
    fetchAllAccountUsageIfStale,
    openClaudeLogin,
    syncAndCheckAllTokens,
    refreshAccountToken,
  }
})
