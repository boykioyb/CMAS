<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { useUiStore } from '@/stores/uiStore'
import AppDialog from '@/components/common/AppDialog.vue'
import type { Account } from '@/types'

const { t } = useI18n()
const accountStore = useAccountStore()
const uiStore = useUiStore()

const props = defineProps<{ open: boolean; account: Account | null }>()
const emit = defineEmits<{ close: [] }>()

const label = ref('')
const plan = ref<'pro' | 'free'>('pro')

watch(() => props.account, (acc) => {
  if (acc) {
    label.value = acc.label || ''
    plan.value = acc.plan
  }
})

async function handleSave() {
  if (!props.account) return
  try {
    await accountStore.updateAccount(props.account.id, { label: label.value, plan: plan.value })
    uiStore.showToast('success', t('common.success'))
    emit('close')
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}
</script>

<template>
  <AppDialog :open="open" :title="t('accounts.editDialog.title')" @close="emit('close')">
    <div class="space-y-4">
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{{ t('accounts.label') }}</label>
        <input
          v-model="label"
          type="text"
          :placeholder="t('accounts.editDialog.labelPlaceholder')"
          class="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none transition"
        />
      </div>

      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{{ t('accounts.plan') }}</label>
        <div class="flex gap-2">
          <button
            @click="plan = 'pro'"
            :class="['flex-1 py-2 rounded-lg text-sm font-medium border transition-colors', plan === 'pro' ? 'bg-blue-50 dark:bg-blue-900/30 border-blue-300 dark:border-blue-700 text-blue-700 dark:text-blue-300' : 'border-gray-200 dark:border-gray-600 text-gray-500']"
          >Pro</button>
          <button
            @click="plan = 'free'"
            :class="['flex-1 py-2 rounded-lg text-sm font-medium border transition-colors', plan === 'free' ? 'bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300' : 'border-gray-200 dark:border-gray-600 text-gray-500']"
          >Free</button>
        </div>
      </div>

      <div class="flex gap-2 pt-2">
        <button
          @click="emit('close')"
          class="flex-1 px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        >
          {{ t('common.cancel') }}
        </button>
        <button
          @click="handleSave"
          class="flex-1 px-4 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 transition-colors"
        >
          {{ t('common.save') }}
        </button>
      </div>
    </div>
  </AppDialog>
</template>
