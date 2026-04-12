<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  value: number
  label?: string
  showPercent?: boolean
  size?: 'sm' | 'md'
}>()

const remaining = computed(() => Math.max(0, 100 - props.value))

const barColor = computed(() => {
  if (remaining.value >= 50) return 'bg-emerald-500'
  if (remaining.value >= 20) return 'bg-amber-500'
  return 'bg-red-500'
})

const bgColor = computed(() => {
  if (remaining.value >= 50) return 'bg-emerald-100 dark:bg-emerald-900/30'
  if (remaining.value >= 20) return 'bg-amber-100 dark:bg-amber-900/30'
  return 'bg-red-100 dark:bg-red-900/30'
})
</script>

<template>
  <div class="w-full">
    <div v-if="label || showPercent" class="flex justify-between items-center mb-1">
      <span v-if="label" class="text-xs font-medium text-gray-600 dark:text-gray-400">{{ label }}</span>
      <span v-if="showPercent" :class="['text-xs font-semibold', remaining >= 50 ? 'text-emerald-600 dark:text-emerald-400' : remaining >= 20 ? 'text-amber-600 dark:text-amber-400' : 'text-red-600 dark:text-red-400']">
        {{ Math.round(remaining) }}%
      </span>
    </div>
    <div :class="['w-full rounded-full overflow-hidden', bgColor, size === 'sm' ? 'h-1.5' : 'h-2.5']">
      <div
        :class="['h-full rounded-full transition-all duration-500', barColor]"
        :style="{ width: remaining + '%' }"
      />
    </div>
  </div>
</template>
