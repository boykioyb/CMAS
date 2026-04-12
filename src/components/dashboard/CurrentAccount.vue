<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { AlertTriangle, ExternalLink as LinkIcon, RefreshCw, CheckCircle, XCircle } from 'lucide-vue-next'
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { open as shellOpen } from '@tauri-apps/plugin-shell'

const { t } = useI18n()
const accountStore = useAccountStore()

const account = computed(() => accountStore.activeAccount)
const usage = computed(() => account.value?.usage)
const real = computed(() => accountStore.realUsage)

// Rate limit countdown
const retryCountdown = ref(0)
let countdownTimer: ReturnType<typeof setInterval> | null = null

function startRetryCountdown(seconds: number) {
  retryCountdown.value = seconds
  if (countdownTimer) clearInterval(countdownTimer)
  countdownTimer = setInterval(() => {
    retryCountdown.value--
    if (retryCountdown.value <= 0) {
      if (countdownTimer) clearInterval(countdownTimer)
      countdownTimer = null
      // Auto-retry when countdown reaches 0
      accountStore.scrapeClaudeUsage().then(() => {
        // If still rate limited, restart countdown
        if (accountStore.realUsage?.retry_after) {
          startRetryCountdown(accountStore.realUsage.retry_after)
        }
      })
    }
  }, 1000)
}

function formatCountdown(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

// Auto-fetch usage on mount
onMounted(async () => {
  if (account.value && !real.value?.success) {
    await accountStore.scrapeClaudeUsage()
    // Start countdown if rate limited
    if (accountStore.realUsage?.retry_after) {
      startRetryCountdown(accountStore.realUsage.retry_after)
    }
  }
})

onUnmounted(() => {
  if (countdownTimer) clearInterval(countdownTimer)
})

const subscriptionLabel = computed(() => {
  const sub = usage.value?.subscription_type
  if (!sub) return null
  if (sub === 'team') return 'Team'
  if (sub === 'pro') return 'Pro'
  return sub.charAt(0).toUpperCase() + sub.slice(1)
})

const tierLabel = computed(() => {
  const tier = usage.value?.rate_limit_tier
  if (!tier) return null
  // Map internal tier names to user-friendly labels
  if (tier.includes('max_5x') || tier.includes('5x')) return '5x'
  if (tier.includes('max')) return 'Max'
  if (tier.includes('raven')) return 'Standard'
  if (tier.includes('pro')) return 'Pro'
  if (tier.includes('free')) return 'Free'
  if (tier.includes('enterprise')) return 'Enterprise'
  // Strip "default_" or "default_claude_" prefix for cleaner display
  return tier.replace(/^default_(claude_)?/, '').replace(/_/g, ' ')
})

function formatResetTime(dateStr?: string) {
  if (!dateStr) return ''
  const d = new Date(dateStr)
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
}

function progressColor(percent: number): string {
  if (percent < 50) return 'bg-blue-500'
  if (percent < 80) return 'bg-amber-500'
  return 'bg-red-500'
}

function openUsagePage() {
  shellOpen('https://claude.ai/settings/usage')
}

// Token health check
const healthResult = ref<{ valid: boolean; org?: string; error?: string } | null>(null)

async function checkHealth() {
  if (!account.value) return
  try {
    const result = await accountStore.checkTokenHealth(account.value.id)
    healthResult.value = {
      valid: result.valid,
      org: result.organization_name ?? undefined,
      error: result.error_message ?? undefined,
    }
  } catch (e) {
    healthResult.value = { valid: false, error: String(e) }
  }
}

async function refreshUsage() {
  await accountStore.scrapeClaudeUsage()
}
</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-100 dark:border-gray-700 shadow-sm card-hover">
    <h3 class="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2 mb-4">
      <span class="w-2 h-2 rounded-full bg-emerald-500" />
      {{ t('dashboard.currentAccount') }}
    </h3>

    <template v-if="account">
      <!-- Account info -->
      <div class="flex items-center gap-2 mb-4">
        <div class="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
          <span class="text-sm font-bold text-primary-600 dark:text-primary-400">
            {{ account.email[0].toUpperCase() }}
          </span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium text-gray-900 dark:text-white truncate">
            {{ account.label || account.email }}
          </div>
          <div v-if="account.label" class="text-xs text-gray-500 truncate">
            {{ account.email }}
          </div>
        </div>
        <span
          v-if="subscriptionLabel"
          class="px-2 py-0.5 rounded-full text-xs font-semibold bg-violet-100 dark:bg-violet-900/30 text-violet-700 dark:text-violet-300"
        >
          {{ subscriptionLabel }}
        </span>
        <span
          v-if="tierLabel"
          class="px-2 py-0.5 rounded-full text-xs font-semibold bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300"
        >
          {{ tierLabel }}
        </span>
      </div>

      <!-- Token health check -->
      <div class="flex items-center gap-2 mb-4">
        <button
          @click="checkHealth"
          :disabled="accountStore.isHealthChecking(account.id)"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium border border-gray-200 dark:border-gray-600 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
        >
          <RefreshCw :size="12" :class="{ 'animate-spin': accountStore.isHealthChecking(account.id) }" />
          Check token
        </button>
        <div v-if="healthResult" class="flex items-center gap-1 text-xs">
          <template v-if="healthResult.valid">
            <CheckCircle :size="13" class="text-emerald-500" />
            <span class="text-emerald-600 dark:text-emerald-400">OK</span>
            <span v-if="healthResult.org" class="text-gray-400">· {{ healthResult.org }}</span>
          </template>
          <template v-else>
            <XCircle :size="13" class="text-red-500" />
            <span class="text-red-600 dark:text-red-400">{{ healthResult.error || 'Failed' }}</span>
          </template>
        </div>
      </div>

      <!-- Rate limit warning banner -->
      <div
        v-if="usage?.is_rate_limited"
        class="flex items-center gap-2 p-3 mb-4 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800"
      >
        <AlertTriangle :size="16" class="text-red-500 shrink-0" />
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium text-red-700 dark:text-red-300">
            {{ t('dashboard.rateLimitWarning') }}
          </div>
          <div v-if="usage.estimated_reset_at" class="text-xs text-red-500 dark:text-red-400 mt-0.5">
            {{ t('dashboard.resetAt') }}: {{ formatResetTime(usage.estimated_reset_at) }}
          </div>
        </div>
      </div>

      <!-- Usage limits -->
      <div class="mb-4">
        <!-- Header with refresh button -->
        <div class="flex items-center justify-between mb-3">
          <span class="text-xs font-semibold text-gray-900 dark:text-white uppercase tracking-wider">
            {{ t('dashboard.usageLimits') }}
          </span>
          <button
            @click="refreshUsage"
            :disabled="accountStore.scrapingUsage"
            class="flex items-center gap-1 text-[10px] text-gray-400 hover:text-primary-500 transition-colors disabled:opacity-50"
          >
            <RefreshCw :size="10" :class="{ 'animate-spin': accountStore.scrapingUsage }" />
            {{ accountStore.scrapingUsage ? 'Syncing...' : 'Sync claude.ai' }}
          </button>
        </div>

        <!-- Real usage from API -->
        <template v-if="real?.success">
          <div class="space-y-3">
            <!-- Current session -->
            <div v-if="real.session_percent != null">
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-xs font-medium text-gray-700 dark:text-gray-300">Session</span>
                <span class="text-xs font-semibold text-gray-900 dark:text-white">
                  {{ real.session_percent }}%
                  <span v-if="real.session_reset" class="font-normal text-gray-400">({{ real.session_reset }})</span>
                </span>
              </div>
              <div class="w-full h-2 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden">
                <div
                  :class="['h-full rounded-full transition-all duration-500', progressColor(real.session_percent)]"
                  :style="{ width: Math.max(2, real.session_percent) + '%' }"
                />
              </div>
            </div>

            <!-- Weekly: All models -->
            <div v-if="real.weekly_all_percent != null">
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-xs font-medium text-gray-700 dark:text-gray-300">{{ t('dashboard.allModels') }}</span>
                <span class="text-xs font-semibold text-gray-900 dark:text-white">
                  {{ real.weekly_all_percent }}%
                  <span v-if="real.weekly_reset" class="font-normal text-gray-400">({{ real.weekly_reset }})</span>
                </span>
              </div>
              <div class="w-full h-2 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden">
                <div
                  :class="['h-full rounded-full transition-all duration-500', progressColor(real.weekly_all_percent)]"
                  :style="{ width: Math.max(2, real.weekly_all_percent) + '%' }"
                />
              </div>
            </div>

            <!-- Weekly: Sonnet -->
            <div v-if="real.weekly_sonnet_percent != null">
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-xs font-medium text-gray-700 dark:text-gray-300">Sonnet</span>
                <span class="text-xs font-semibold text-gray-900 dark:text-white">{{ real.weekly_sonnet_percent }}%</span>
              </div>
              <div class="w-full h-2 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden">
                <div
                  :class="['h-full rounded-full transition-all duration-500', progressColor(real.weekly_sonnet_percent)]"
                  :style="{ width: Math.max(2, real.weekly_sonnet_percent) + '%' }"
                />
              </div>
            </div>
          </div>

        </template>

        <!-- Rate limited — show countdown -->
        <template v-else-if="real?.retry_after || retryCountdown > 0">
          <div class="text-center py-4">
            <div class="w-10 h-10 mx-auto mb-3 rounded-full bg-amber-50 dark:bg-amber-900/20 flex items-center justify-center">
              <RefreshCw :size="18" class="text-amber-500" :class="{ 'animate-spin': retryCountdown <= 3 && retryCountdown > 0 }" />
            </div>
            <p class="text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">API rate limited</p>
            <p v-if="retryCountdown > 0" class="text-lg font-bold text-amber-600 dark:text-amber-400">
              {{ formatCountdown(retryCountdown) }}
            </p>
            <p class="text-[11px] text-gray-400 mt-1">{{ t('dashboard.autoRetry') }}</p>
          </div>
        </template>

        <!-- Not authenticated -->
        <template v-else-if="real && !real.authenticated">
          <div class="text-center py-4">
            <p class="text-xs text-gray-500 mb-2">{{ t('dashboard.tokenExpired') }}</p>
            <p class="text-xs text-gray-400 mb-3">{{ t('dashboard.reAddAccount') }}</p>
          </div>
        </template>

        <!-- Error message -->
        <template v-else-if="real?.error_message">
          <div class="text-center py-4">
            <p class="text-xs text-red-500 mb-2">{{ real.error_message }}</p>
            <button
              @click="refreshUsage"
              :disabled="accountStore.scrapingUsage"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-primary-500 text-white hover:bg-primary-600 transition-colors disabled:opacity-50"
            >
              <RefreshCw :size="13" :class="{ 'animate-spin': accountStore.scrapingUsage }" />
              {{ t('dashboard.retry') }}
            </button>
          </div>
        </template>

        <!-- Loading -->
        <template v-else>
          <div v-if="accountStore.scrapingUsage" class="flex items-center justify-center gap-2 py-4 text-xs text-gray-400">
            <RefreshCw :size="13" class="animate-spin" />
            {{ t('dashboard.fetchingData') }}
          </div>
          <div v-else class="text-center py-4">
            <p class="text-xs text-gray-400">{{ t('dashboard.clickSyncToFetch') }}</p>
          </div>
        </template>

        <!-- Link to full usage page -->
        <button
          @click="openUsagePage"
          class="w-full flex items-center justify-center gap-1.5 text-xs text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 transition-colors py-2 mt-3"
        >
          {{ t('dashboard.viewUsagePage') }}
          <LinkIcon :size="12" />
        </button>
      </div>

    </template>

    <template v-else>
      <div class="text-center py-6 text-gray-400">
        <p class="text-sm">{{ t('dashboard.noActiveAccount') }}</p>
      </div>
    </template>
  </div>
</template>
