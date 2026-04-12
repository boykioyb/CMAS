<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAccountStore } from '@/stores/accountStore'
import { useUiStore } from '@/stores/uiStore'
import AppDialog from '@/components/common/AppDialog.vue'
import { LogIn, Loader2, CheckCircle, UserPlus, ExternalLink, ArrowLeft } from 'lucide-vue-next'

const { t } = useI18n()
const accountStore = useAccountStore()
const uiStore = useUiStore()

defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

// Flow steps: 'choose' -> 'oauth_waiting' -> 'oauth_detected' -> done
// Or:         'choose' -> 'add_current' -> done
type Step = 'choose' | 'oauth_waiting' | 'oauth_detected' | 'add_current'

const step = ref<Step>('choose')
const label = ref('')
const loading = ref(false)
const detectedEmail = ref('')
const detectedOrg = ref('')
const detectedSubscription = ref('')
const pollTimer = ref<ReturnType<typeof setInterval> | null>(null)
const backedUpEmail = ref<string | null>(null)

// Reset state when dialog opens
watch(() => step.value, () => {
  if (step.value === 'choose') {
    label.value = ''
    detectedEmail.value = ''
    detectedOrg.value = ''
    detectedSubscription.value = ''
    backedUpEmail.value = null
  }
})

function resetAndClose() {
  stopPolling()
  step.value = 'choose'
  label.value = ''
  emit('close')
}

// ===== Method 1: OAuth Login (new account) =====

async function startOAuthFlow() {
  loading.value = true
  try {
    // Step 1: Backup current credentials
    const currentEmail = await invoke<string | null>('auth_backup_current')
    backedUpEmail.value = currentEmail

    // Step 2: Logout and open OAuth in Terminal
    await invoke('auth_start_login')

    step.value = 'oauth_waiting'

    // Step 3: Start polling for login completion
    startPolling()
  } catch (e) {
    uiStore.showToast('error', String(e))
  } finally {
    loading.value = false
  }
}

function startPolling() {
  stopPolling()
  pollTimer.value = setInterval(async () => {
    try {
      const result = await invoke<{
        logged_in: boolean
        email?: string
        account_uuid?: string
        org_name?: string
        subscription_type?: string
      }>('auth_check_login_status')

      if (result.logged_in && result.email) {
        detectedEmail.value = result.email
        detectedOrg.value = result.org_name || ''
        detectedSubscription.value = result.subscription_type || ''
        step.value = 'oauth_detected'
        stopPolling()
      }
    } catch {
      // Keep polling
    }
  }, 2000)
}

function stopPolling() {
  if (pollTimer.value) {
    clearInterval(pollTimer.value)
    pollTimer.value = null
  }
}

async function confirmNewAccount() {
  loading.value = true
  try {
    // Save the new account
    await invoke('auth_confirm_new_account', { label: label.value || null })

    // Restore original account credentials
    await invoke('auth_restore_original')

    // Refresh account list
    await accountStore.fetchAccounts()

    uiStore.showToast('success', t('accounts.addDialog.accountAdded', { email: detectedEmail.value }))
    resetAndClose()
  } catch (e) {
    uiStore.showToast('error', String(e))
  } finally {
    loading.value = false
  }
}

async function cancelOAuth() {
  stopPolling()
  loading.value = true
  try {
    // Restore original account
    await invoke('auth_restore_original')
  } catch {
    // Best effort restore
  }
  loading.value = false
  step.value = 'choose'
}

// ===== Method 2: Add current (already logged in) account =====

async function addCurrentAccount() {
  loading.value = true
  try {
    await accountStore.addCurrentAccount(label.value || undefined)
    uiStore.showToast('success', t('common.success'))
    resetAndClose()
  } catch (e) {
    uiStore.showToast('error', String(e))
  } finally {
    loading.value = false
  }
}

onUnmounted(() => {
  stopPolling()
})
</script>

<template>
  <AppDialog :open="open" :title="t('accounts.addDialog.title')" @close="resetAndClose">

    <!-- Step: Choose method -->
    <div v-if="step === 'choose'" class="space-y-3">
      <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
        {{ t('accounts.addDialog.chooseMethod') }}
      </p>

      <!-- Option 1: OAuth Login -->
      <button
        @click="startOAuthFlow"
        :disabled="loading"
        class="w-full flex items-center gap-4 p-4 rounded-xl border-2 border-gray-200 dark:border-gray-600 hover:border-primary-400 dark:hover:border-primary-500 transition-all group text-left"
      >
        <div class="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center shrink-0 group-hover:bg-primary-200 dark:group-hover:bg-primary-900/50 transition-colors">
          <LogIn :size="20" class="text-primary-600 dark:text-primary-400" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-semibold text-gray-900 dark:text-white">{{ t('accounts.addDialog.oauthLogin') }}</div>
          <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{{ t('accounts.addDialog.oauthLoginDesc') }}</div>
        </div>
        <ExternalLink :size="16" class="text-gray-400 shrink-0" />
      </button>

      <!-- Option 2: Add current -->
      <button
        @click="step = 'add_current'"
        class="w-full flex items-center gap-4 p-4 rounded-xl border-2 border-gray-200 dark:border-gray-600 hover:border-emerald-400 dark:hover:border-emerald-500 transition-all group text-left"
      >
        <div class="w-10 h-10 rounded-lg bg-emerald-100 dark:bg-emerald-900/30 flex items-center justify-center shrink-0 group-hover:bg-emerald-200 dark:group-hover:bg-emerald-900/50 transition-colors">
          <UserPlus :size="20" class="text-emerald-600 dark:text-emerald-400" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-semibold text-gray-900 dark:text-white">{{ t('accounts.addDialog.saveCurrentTitle') }}</div>
          <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{{ t('accounts.addDialog.saveCurrentDesc') }}</div>
        </div>
      </button>
    </div>

    <!-- Step: Waiting for OAuth -->
    <div v-else-if="step === 'oauth_waiting'" class="text-center py-4">
      <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
        <Loader2 :size="28" class="text-primary-500 animate-spin" />
      </div>
      <h3 class="text-base font-semibold text-gray-900 dark:text-white mb-2">{{ t('accounts.addDialog.waitingLogin') }}</h3>
      <p class="text-sm text-gray-500 dark:text-gray-400 mb-1">
        {{ t('accounts.addDialog.browserOpened') }}
      </p>
      <p class="text-xs text-gray-400 mb-6">
        {{ t('accounts.addDialog.autoDetect') }}
      </p>

      <button
        @click="cancelOAuth"
        :disabled="loading"
        class="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
      >
        <ArrowLeft :size="14" />
        {{ t('accounts.addDialog.cancelAndBack') }}
      </button>
    </div>

    <!-- Step: OAuth detected -->
    <div v-else-if="step === 'oauth_detected'" class="space-y-4">
      <div class="flex items-center gap-3 p-4 rounded-xl bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800">
        <CheckCircle :size="20" class="text-emerald-500 shrink-0" />
        <div>
          <div class="text-sm font-semibold text-emerald-800 dark:text-emerald-200">{{ t('accounts.addDialog.newAccountDetected') }}</div>
          <div class="text-sm text-emerald-700 dark:text-emerald-300">{{ detectedEmail }}</div>
          <div v-if="detectedOrg" class="text-xs text-emerald-600 dark:text-emerald-400">{{ detectedOrg }} · {{ detectedSubscription }}</div>
        </div>
      </div>

      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{{ t('accounts.label') }}</label>
        <input
          v-model="label"
          type="text"
          :placeholder="t('accounts.addDialog.labelPlaceholder')"
          class="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none transition"
        />
      </div>

      <div class="flex gap-2 pt-2">
        <button
          @click="cancelOAuth"
          class="flex items-center justify-center px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors shrink-0"
        >
          {{ t('common.cancel') }}
        </button>
        <button
          @click="confirmNewAccount"
          :disabled="loading"
          class="flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 disabled:opacity-50 transition-colors whitespace-nowrap"
        >
          <Loader2 v-if="loading" :size="16" class="animate-spin" />
          <CheckCircle v-else :size="16" />
          {{ t('accounts.addDialog.confirmAdd') }}
        </button>
      </div>
    </div>

    <!-- Step: Add current account -->
    <div v-else-if="step === 'add_current'" class="space-y-4">
      <p class="text-sm text-gray-500 dark:text-gray-400">
        {{ t('accounts.addDialog.description') }}
      </p>

      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{{ t('accounts.label') }}</label>
        <input
          v-model="label"
          type="text"
          :placeholder="t('accounts.addDialog.labelPlaceholder')"
          class="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none transition"
        />
      </div>

      <div class="flex gap-2 pt-2">
        <button
          @click="step = 'choose'"
          class="flex items-center justify-center gap-1.5 px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors shrink-0"
        >
          <ArrowLeft :size="14" />
          {{ t('common.cancel') }}
        </button>
        <button
          @click="addCurrentAccount"
          :disabled="loading"
          class="flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 disabled:opacity-50 transition-colors whitespace-nowrap"
        >
          <Loader2 v-if="loading" :size="16" class="animate-spin" />
          <UserPlus v-else :size="16" />
          {{ t('accounts.addCurrentAccount') }}
        </button>
      </div>
    </div>

  </AppDialog>
</template>
