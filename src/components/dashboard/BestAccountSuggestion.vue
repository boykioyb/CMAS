<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAccountStore } from '@/stores/accountStore'
import { Sparkles, CheckCircle } from 'lucide-vue-next'
import { computed } from 'vue'

const { t } = useI18n()
const accountStore = useAccountStore()

const best = computed(() => accountStore.bestAccount)
const bestReal = computed(() => {
  if (!best.value) return null
  return accountStore.getAccountRealUsage(best.value.id)
})

function progressColor(percent: number): string {
  if (percent < 50) return 'bg-blue-500'
  if (percent < 80) return 'bg-amber-500'
  return 'bg-red-500'
}

</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-100 dark:border-gray-700 shadow-sm card-hover">
    <h3 class="text-sm font-semibold text-gray-900 dark:text-white flex items-center gap-2 mb-4">
      <Sparkles :size="16" class="text-amber-500" />
      {{ t('dashboard.bestSuggestion') }}
    </h3>

    <template v-if="best">
      <div class="bg-gradient-to-r from-emerald-50 to-blue-50 dark:from-emerald-900/20 dark:to-blue-900/20 rounded-lg p-4 mb-4">
        <div class="flex items-center justify-between mb-2">
          <span class="text-sm font-medium text-gray-900 dark:text-white truncate">
            {{ best.label || best.email }}
          </span>
          <span class="flex items-center gap-1 text-xs font-semibold text-emerald-600 dark:text-emerald-400">
            <CheckCircle :size="13" />
            {{ t('dashboard.active') }}
          </span>
        </div>
        <div v-if="best.label" class="text-xs text-gray-500 mb-3 truncate">
          {{ best.email }}
        </div>

        <!-- Usage bars for best account -->
        <div v-if="bestReal?.success" class="space-y-2">
          <div v-if="bestReal.session_percent != null">
            <div class="flex items-center justify-between mb-0.5">
              <span class="text-[10px] text-gray-500">Session</span>
              <span class="text-[10px] font-semibold text-gray-700 dark:text-gray-300">
                {{ bestReal.session_percent }}%
                <span v-if="bestReal.session_reset" class="font-normal text-gray-400">({{ bestReal.session_reset }})</span>
              </span>
            </div>
            <div class="w-full h-1.5 rounded-full bg-white/60 dark:bg-gray-700 overflow-hidden">
              <div
                :class="['h-full rounded-full transition-all duration-500', progressColor(bestReal.session_percent)]"
                :style="{ width: Math.max(2, bestReal.session_percent) + '%' }"
              />
            </div>
          </div>
          <div v-if="bestReal.weekly_all_percent != null">
            <div class="flex items-center justify-between mb-0.5">
              <span class="text-[10px] text-gray-500">Weekly</span>
              <span class="text-[10px] font-semibold text-gray-700 dark:text-gray-300">
                {{ bestReal.weekly_all_percent }}%
                <span v-if="bestReal.weekly_reset" class="font-normal text-gray-400">({{ bestReal.weekly_reset }})</span>
              </span>
            </div>
            <div class="w-full h-1.5 rounded-full bg-white/60 dark:bg-gray-700 overflow-hidden">
              <div
                :class="['h-full rounded-full transition-all duration-500', progressColor(bestReal.weekly_all_percent)]"
                :style="{ width: Math.max(2, bestReal.weekly_all_percent) + '%' }"
              />
            </div>
          </div>
        </div>
        <div v-else class="text-[10px] text-gray-400 py-1">
          Loading usage...
        </div>
      </div>

    </template>

    <template v-else>
      <div class="text-center py-6 text-gray-400">
        <p class="text-sm">{{ t('common.noData') }}</p>
      </div>
    </template>
  </div>
</template>
