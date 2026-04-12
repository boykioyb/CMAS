<script setup lang="ts">
import { useUiStore } from '@/stores/uiStore'
import { CheckCircle, XCircle, AlertTriangle, Info, X } from 'lucide-vue-next'

const uiStore = useUiStore()

const icons = {
  success: CheckCircle,
  error: XCircle,
  warning: AlertTriangle,
  info: Info,
}

const colors = {
  success: 'bg-emerald-50 dark:bg-emerald-900/30 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800',
  error: 'bg-red-50 dark:bg-red-900/30 text-red-800 dark:text-red-200 border-red-200 dark:border-red-800',
  warning: 'bg-amber-50 dark:bg-amber-900/30 text-amber-800 dark:text-amber-200 border-amber-200 dark:border-amber-800',
  info: 'bg-blue-50 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200 border-blue-200 dark:border-blue-800',
}
</script>

<template>
  <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
    <TransitionGroup name="toast">
      <div
        v-for="toast in uiStore.toasts"
        :key="toast.id"
        :class="['flex items-center gap-3 px-4 py-3 rounded-xl border shadow-lg', colors[toast.type]]"
      >
        <component :is="icons[toast.type]" :size="18" class="shrink-0" />
        <span class="text-sm flex-1">{{ toast.message }}</span>
        <button @click="uiStore.removeToast(toast.id)" class="shrink-0 opacity-60 hover:opacity-100">
          <X :size="14" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style>
.toast-enter-active { animation: toast-in 0.3s ease; }
.toast-leave-active { animation: toast-out 0.2s ease; }
@keyframes toast-in {
  from { opacity: 0; transform: translateX(100%); }
  to { opacity: 1; transform: translateX(0); }
}
@keyframes toast-out {
  from { opacity: 1; transform: translateX(0); }
  to { opacity: 0; transform: translateX(100%); }
}
</style>
