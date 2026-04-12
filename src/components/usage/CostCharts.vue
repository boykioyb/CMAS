<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDark } from '@vueuse/core'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  ArcElement,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from 'chart.js'
import { Bar, Doughnut, Line } from 'vue-chartjs'
import type { CostUsageRecord } from '@/types'

ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  ArcElement,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
)

const props = defineProps<{
  records: CostUsageRecord[]
}>()

const { t } = useI18n()
const isDark = useDark()

// --- Color palette ---
const modelColors: Record<string, { bg: string; border: string }> = {
  opus: { bg: 'rgba(147, 51, 234, 0.7)', border: 'rgb(147, 51, 234)' },
  sonnet: { bg: 'rgba(59, 130, 246, 0.7)', border: 'rgb(59, 130, 246)' },
  haiku: { bg: 'rgba(16, 185, 129, 0.7)', border: 'rgb(16, 185, 129)' },
  other: { bg: 'rgba(156, 163, 175, 0.7)', border: 'rgb(156, 163, 175)' },
}

const accountPalette = [
  { bg: 'rgba(59, 130, 246, 0.7)', border: 'rgb(59, 130, 246)' },
  { bg: 'rgba(147, 51, 234, 0.7)', border: 'rgb(147, 51, 234)' },
  { bg: 'rgba(16, 185, 129, 0.7)', border: 'rgb(16, 185, 129)' },
  { bg: 'rgba(245, 158, 11, 0.7)', border: 'rgb(245, 158, 11)' },
  { bg: 'rgba(239, 68, 68, 0.7)', border: 'rgb(239, 68, 68)' },
  { bg: 'rgba(236, 72, 153, 0.7)', border: 'rgb(236, 72, 153)' },
  { bg: 'rgba(20, 184, 166, 0.7)', border: 'rgb(20, 184, 166)' },
  { bg: 'rgba(99, 102, 241, 0.7)', border: 'rgb(99, 102, 241)' },
]

function getModelKey(model: string): string {
  const m = model.toLowerCase()
  if (m.includes('opus')) return 'opus'
  if (m.includes('sonnet')) return 'sonnet'
  if (m.includes('haiku')) return 'haiku'
  return 'other'
}

function gridColor() {
  return isDark.value ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)'
}

function textColor() {
  return isDark.value ? 'rgba(255,255,255,0.6)' : 'rgba(0,0,0,0.5)'
}

// --- 1. Daily Cost Trend (Bar) ---
const dailyCostData = computed(() => {
  const map = new Map<string, number>()
  for (const r of props.records) {
    const day = r.timestamp.slice(0, 10)
    map.set(day, (map.get(day) ?? 0) + r.estimated_cost_usd)
  }
  const sorted = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  const labels = sorted.map(([d]) => {
    const date = new Date(d + 'T00:00:00')
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  })
  const data = sorted.map(([, v]) => +v.toFixed(4))

  return {
    labels,
    datasets: [{
      label: t('usage.cost'),
      data,
      backgroundColor: 'rgba(59, 130, 246, 0.6)',
      borderColor: 'rgb(59, 130, 246)',
      borderWidth: 1,
      borderRadius: 4,
      maxBarThickness: 40,
    }],
  }
})

const dailyCostOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      callbacks: {
        label: (ctx: any) => `$${ctx.raw.toFixed(4)}`,
      },
    },
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: { color: textColor(), font: { size: 10 } },
    },
    y: {
      grid: { color: gridColor() },
      ticks: {
        color: textColor(),
        font: { size: 10 },
        callback: (v: any) => `$${v}`,
      },
    },
  },
}))

// --- 2. Cost by Model (Doughnut) ---
const modelCostData = computed(() => {
  const map = new Map<string, number>()
  for (const r of props.records) {
    const key = r.model_display
    map.set(key, (map.get(key) ?? 0) + r.estimated_cost_usd)
  }
  const sorted = [...map.entries()].sort((a, b) => b[1] - a[1])
  const labels = sorted.map(([k]) => k)
  const data = sorted.map(([, v]) => +v.toFixed(4))
  const bgColors = sorted.map(([k]) => modelColors[getModelKey(k)]?.bg ?? modelColors.other.bg)
  const borderColors = sorted.map(([k]) => modelColors[getModelKey(k)]?.border ?? modelColors.other.border)

  return {
    labels,
    datasets: [{
      data,
      backgroundColor: bgColors,
      borderColor: borderColors,
      borderWidth: 2,
    }],
  }
})

const doughnutOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  cutout: '60%',
  plugins: {
    legend: {
      position: 'bottom' as const,
      labels: { color: textColor(), padding: 12, font: { size: 11 }, boxWidth: 12, boxHeight: 12 },
    },
    tooltip: {
      callbacks: {
        label: (ctx: any) => `${ctx.label}: $${ctx.raw.toFixed(4)}`,
      },
    },
  },
}))

// --- 3. Cost by Account (Horizontal Bar) ---
const accountCostData = computed(() => {
  const map = new Map<string, { cost: number; label: string }>()
  for (const r of props.records) {
    const key = r.account_email
    const existing = map.get(key)
    if (existing) {
      existing.cost += r.estimated_cost_usd
    } else {
      let label = r.account_label || r.account_email
      const at = label.indexOf('@')
      if (at > 0 && !r.account_label) label = label.substring(0, at)
      map.set(key, { cost: r.estimated_cost_usd, label })
    }
  }
  const sorted = [...map.values()].sort((a, b) => b.cost - a.cost)
  const labels = sorted.map(v => v.label)
  const data = sorted.map(v => +v.cost.toFixed(4))
  const bgColors = sorted.map((_, i) => accountPalette[i % accountPalette.length].bg)
  const borderColors = sorted.map((_, i) => accountPalette[i % accountPalette.length].border)

  return {
    labels,
    datasets: [{
      label: t('usage.cost'),
      data,
      backgroundColor: bgColors,
      borderColor: borderColors,
      borderWidth: 1,
      borderRadius: 4,
      maxBarThickness: 28,
    }],
  }
})

const accountCostOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  indexAxis: 'y' as const,
  plugins: {
    legend: { display: false },
    tooltip: {
      callbacks: {
        label: (ctx: any) => `$${ctx.raw.toFixed(4)}`,
      },
    },
  },
  scales: {
    x: {
      grid: { color: gridColor() },
      ticks: {
        color: textColor(),
        font: { size: 10 },
        callback: (v: any) => `$${v}`,
      },
    },
    y: {
      grid: { display: false },
      ticks: { color: textColor(), font: { size: 11 } },
    },
  },
}))

// --- 4. Daily Token Usage (Stacked Bar) ---
const dailyTokenData = computed(() => {
  const map = new Map<string, { input: number; output: number; cache: number }>()
  for (const r of props.records) {
    const day = r.timestamp.slice(0, 10)
    const entry = map.get(day) ?? { input: 0, output: 0, cache: 0 }
    entry.input += r.input_tokens
    entry.output += r.output_tokens
    entry.cache += r.cache_read_tokens
    map.set(day, entry)
  }
  const sorted = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  const labels = sorted.map(([d]) => {
    const date = new Date(d + 'T00:00:00')
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  })

  return {
    labels,
    datasets: [
      {
        label: 'Input',
        data: sorted.map(([, v]) => v.input),
        backgroundColor: 'rgba(59, 130, 246, 0.6)',
        borderColor: 'rgb(59, 130, 246)',
        borderWidth: 1,
        borderRadius: 2,
      },
      {
        label: 'Output',
        data: sorted.map(([, v]) => v.output),
        backgroundColor: 'rgba(245, 158, 11, 0.6)',
        borderColor: 'rgb(245, 158, 11)',
        borderWidth: 1,
        borderRadius: 2,
      },
      {
        label: 'Cache',
        data: sorted.map(([, v]) => v.cache),
        backgroundColor: 'rgba(16, 185, 129, 0.6)',
        borderColor: 'rgb(16, 185, 129)',
        borderWidth: 1,
        borderRadius: 2,
      },
    ],
  }
})

function formatTokenShort(v: number): string {
  if (v >= 1_000_000) return (v / 1_000_000).toFixed(1) + 'M'
  if (v >= 1_000) return (v / 1_000).toFixed(0) + 'K'
  return v.toString()
}

const dailyTokenOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: 'bottom' as const,
      labels: { color: textColor(), padding: 12, font: { size: 11 }, boxWidth: 12, boxHeight: 12 },
    },
    tooltip: {
      callbacks: {
        label: (ctx: any) => `${ctx.dataset.label}: ${formatTokenShort(ctx.raw)}`,
      },
    },
  },
  scales: {
    x: {
      stacked: true,
      grid: { display: false },
      ticks: { color: textColor(), font: { size: 10 } },
    },
    y: {
      stacked: true,
      grid: { color: gridColor() },
      ticks: {
        color: textColor(),
        font: { size: 10 },
        callback: (v: any) => formatTokenShort(v),
      },
    },
  },
}))

// --- 5. Cost Trend Line (cumulative daily) ---
const costTrendData = computed(() => {
  const map = new Map<string, number>()
  for (const r of props.records) {
    const day = r.timestamp.slice(0, 10)
    map.set(day, (map.get(day) ?? 0) + r.estimated_cost_usd)
  }
  const sorted = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  const labels = sorted.map(([d]) => {
    const date = new Date(d + 'T00:00:00')
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  })

  let cumulative = 0
  const data = sorted.map(([, v]) => {
    cumulative += v
    return +cumulative.toFixed(4)
  })

  return {
    labels,
    datasets: [{
      label: t('charts.cumulativeCost'),
      data,
      borderColor: 'rgb(147, 51, 234)',
      backgroundColor: 'rgba(147, 51, 234, 0.1)',
      borderWidth: 2,
      fill: true,
      tension: 0.3,
      pointRadius: 3,
      pointHoverRadius: 5,
      pointBackgroundColor: 'rgb(147, 51, 234)',
    }],
  }
})

const costTrendOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      callbacks: {
        label: (ctx: any) => `$${ctx.raw.toFixed(4)}`,
      },
    },
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: { color: textColor(), font: { size: 10 } },
    },
    y: {
      grid: { color: gridColor() },
      ticks: {
        color: textColor(),
        font: { size: 10 },
        callback: (v: any) => `$${v}`,
      },
    },
  },
}))
</script>

<template>
  <div class="space-y-4">
    <!-- Row 1: Daily Cost + Cost by Model -->
    <div class="grid grid-cols-3 gap-4">
      <!-- Daily Cost Bar -->
      <div class="col-span-2 bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">{{ t('charts.dailyCost') }}</h3>
        <div class="h-52">
          <Bar :data="dailyCostData" :options="dailyCostOptions" />
        </div>
      </div>

      <!-- Cost by Model Doughnut -->
      <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">{{ t('charts.costByModel') }}</h3>
        <div class="h-52">
          <Doughnut :data="modelCostData" :options="doughnutOptions" />
        </div>
      </div>
    </div>

    <!-- Row 2: Cumulative Trend + Cost by Account -->
    <div class="grid grid-cols-3 gap-4">
      <!-- Cumulative Cost Line -->
      <div class="col-span-2 bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">{{ t('charts.cumulativeCost') }}</h3>
        <div class="h-52">
          <Line :data="costTrendData" :options="costTrendOptions" />
        </div>
      </div>

      <!-- Cost by Account Horizontal Bar -->
      <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">{{ t('charts.costByAccount') }}</h3>
        <div class="h-52">
          <Bar :data="accountCostData" :options="accountCostOptions" />
        </div>
      </div>
    </div>

    <!-- Row 3: Token Usage (full width) -->
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 p-4 shadow-sm">
      <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">{{ t('charts.dailyTokens') }}</h3>
      <div class="h-56">
        <Bar :data="dailyTokenData" :options="dailyTokenOptions" />
      </div>
    </div>
  </div>
</template>
