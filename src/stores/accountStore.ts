import { defineStore } from 'pinia'
import { ref, computed, shallowRef, triggerRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'

/** Run async tasks sequentially with a delay between each to avoid rate limiting */
async function sequential<T>(items: T[], fn: (item: T) => Promise<void>, delayMs = 1000): Promise<void> {
  for (const item of items) {
    try {
      await fn(item)
    } catch {
      // Continue with remaining items even if one fails
    }
    if (delayMs > 0 && items.indexOf(item) < items.length - 1) {
      await new Promise(r => setTimeout(r, delayMs))
    }
  }
}
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

  /** Silent load: no loading state, no skeleton — for initial app startup */
  async function silentLoadAccounts() {
    try {
      accounts.value = await invoke<Account[]>('list_accounts')
    } catch (e) {
      error.value = String(e)
    }
  }

  /** With loading state — for user-triggered manual refresh */
  async function fetchAccounts() {
    loading.value = true
    error.value = null
    try {
      accounts.value = await invoke<Account[]>('list_accounts')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
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
      const account = accounts.value[idx]
      if (projectIndex == null) {
        account.selected_project_id = undefined
      } else {
        account.selected_project_id = account.project_ids[projectIndex]
      }
    }
  }

  async function linkProject(accountId: string, projectId: string): Promise<Account> {
    const updated = await invoke<Account>('link_project_to_account', { accountId, projectId })
    const idx = accounts.value.findIndex(a => a.id === accountId)
    if (idx >= 0) accounts.value[idx] = updated
    return updated
  }

  async function unlinkProject(accountId: string, projectId: string): Promise<Account> {
    const updated = await invoke<Account>('unlink_project_from_account', { accountId, projectId })
    const idx = accounts.value.findIndex(a => a.id === accountId)
    if (idx >= 0) accounts.value[idx] = updated
    return updated
  }

  async function setSelectedProjectId(accountId: string, projectId: string | null) {
    await invoke('set_selected_project_id', { accountId, projectId })
    const idx = accounts.value.findIndex(a => a.id === accountId)
    if (idx >= 0) accounts.value[idx].selected_project_id = projectId ?? undefined
  }

  async function getQuotaSummary(): Promise<QuotaSummary> {
    return invoke<QuotaSummary>('get_quota_summary')
  }

  const healthChecking = ref<Set<string>>(new Set())

  async function checkTokenHealth(accountId: string): Promise<TokenHealthResult> {
    healthChecking.value.add(accountId)
    try {
      const result = await invoke<TokenHealthResult>('check_account_token', { accountId })
      // Update local account status to match API result. transient_error means
      // the API/network blipped — don't change the displayed status, the next
      // poll will resolve it.
      const idx = accounts.value.findIndex(a => a.id === accountId)
      if (idx >= 0 && result.status !== 'transient_error') {
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
    // Run one-by-one with 1.5s delay to avoid rate limiting
    await sequential(accounts.value, a => checkTokenHealth(a.id).then(() => {}), 1500)
  }

  const realUsage = ref<RealUsageData | null>(null)
  const scrapingUsage = ref(false)
  // Per-account real usage data (keyed by account ID)
  // shallowRef: only trigger reactivity when we explicitly call triggerRef,
  // avoids re-rendering the entire account list on every single usage fetch
  const accountRealUsage = shallowRef<Record<string, RealUsageData>>({})
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
      triggerRef(accountRealUsage) // Manually trigger reactivity for shallowRef
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

  async function fetchAllAccountUsage(): Promise<{ ok: number; failed: number; rateLimited: number }> {
    lastUsageFetchTime = Date.now()
    let ok = 0, failed = 0, rateLimited = 0
    await sequential(accounts.value, async (a) => {
      const data = await fetchAccountUsage(a.id)
      if (data.success) {
        ok++
      } else if (data.error_message?.includes('Rate limited')) {
        rateLimited++
      } else {
        failed++
      }
    }, 1500)
    return { ok, failed, rateLimited }
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

  // Deduplication: prevent concurrent sync/refresh calls
  let pendingSyncPromise: Promise<TokenSyncResult[]> | null = null
  const pendingRefreshes = new Map<string, Promise<TokenSyncResult>>()

  /** Sync active credentials + check all tokens + auto-refresh expired ones */
  async function syncAndCheckAllTokens(): Promise<TokenSyncResult[]> {
    // Dedup: reuse in-flight request
    if (pendingSyncPromise) return pendingSyncPromise

    pendingSyncPromise = (async () => {
      try {
        const results = await invoke<TokenSyncResult[]>('sync_and_check_all_tokens')
        // Update local account statuses. transient_error preserves whatever
        // status the account had before — it's a "we don't know" signal.
        for (const result of results) {
          const idx = accounts.value.findIndex(a => a.id === result.account_id)
          if (idx >= 0 && result.status !== 'transient_error') {
            accounts.value[idx].status = result.status === 'ok' ? 'ok' : (result.status === 'expired' ? 'expired' : 'error')
          }
        }
        return results
      } catch (e) {
        console.error('syncAndCheckAllTokens failed:', e)
        return []
      } finally {
        pendingSyncPromise = null
      }
    })()

    return pendingSyncPromise
  }

  /** Manually refresh a specific account's token */
  async function refreshAccountToken(accountId: string): Promise<TokenSyncResult> {
    // Dedup: reuse in-flight refresh for same account
    const pending = pendingRefreshes.get(accountId)
    if (pending) return pending

    healthChecking.value.add(accountId)

    const refreshPromise = (async () => {
      try {
        const result = await invoke<TokenSyncResult>('refresh_account_token', { accountId })
        const idx = accounts.value.findIndex(a => a.id === accountId)
        if (idx >= 0 && result.status !== 'transient_error') {
          accounts.value[idx].status = result.status === 'ok' ? 'ok' : (result.status === 'expired' ? 'expired' : 'error')
        }
        return result
      } finally {
        healthChecking.value.delete(accountId)
        pendingRefreshes.delete(accountId)
      }
    })()

    pendingRefreshes.set(accountId, refreshPromise)
    return refreshPromise
  }

  return {
    accounts,
    loading,
    error,
    activeAccount,
    sortedAccounts,
    bestAccount,
    fetchAccounts,
    silentLoadAccounts,
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
    linkProject,
    unlinkProject,
    setSelectedProjectId,
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
