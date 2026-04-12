import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CostUsageRecord } from '@/types'

export const useCostUsageStore = defineStore('costUsage', () => {
  const records = ref<CostUsageRecord[]>([])
  const syncing = ref(false)
  const lastSyncAt = ref<number>(0)

  /** Sync cost usage data from backend (incremental cache). */
  async function sync(days: number = 90) {
    if (syncing.value) return
    syncing.value = true
    try {
      records.value = await invoke<CostUsageRecord[]>('get_cost_usage_history', { days })
      lastSyncAt.value = Date.now()
    } catch (e) {
      console.error('cost-usage sync failed:', e)
    } finally {
      syncing.value = false
    }
  }

  /** Sync only if stale (no data or older than 60s). */
  async function syncIfStale() {
    const isStale = Date.now() - lastSyncAt.value > 60_000
    if (records.value.length === 0 || isStale) {
      await sync()
    }
  }

  /** Get records filtered by days. */
  function getRecords(days: number): CostUsageRecord[] {
    const cutoff = new Date(Date.now() - days * 86_400_000).toISOString()
    return records.value.filter(r => r.timestamp > cutoff)
  }

  return {
    records,
    syncing,
    lastSyncAt,
    sync,
    syncIfStale,
    getRecords,
  }
})
