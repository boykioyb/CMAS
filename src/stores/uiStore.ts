import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface Toast {
  id: string
  type: 'success' | 'error' | 'warning' | 'info'
  message: string
  duration?: number
}

export const useUiStore = defineStore('ui', () => {
  const toasts = ref<Toast[]>([])
  const viewMode = ref<'table' | 'grid'>('table')
  const accountFilter = ref<'all' | 'good' | 'low' | 'empty'>('all')
  const searchQuery = ref('')

  function showToast(type: Toast['type'], message: string, duration = 3000) {
    const id = Date.now().toString()
    toasts.value.push({ id, type, message, duration })
    if (duration > 0) {
      setTimeout(() => {
        toasts.value = toasts.value.filter(t => t.id !== id)
      }, duration)
    }
  }

  function removeToast(id: string) {
    toasts.value = toasts.value.filter(t => t.id !== id)
  }

  return {
    toasts,
    viewMode,
    accountFilter,
    searchQuery,
    showToast,
    removeToast,
  }
})
