<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useProjectStore } from '@/stores/projectStore'
import { useUiStore } from '@/stores/uiStore'
import {
  FolderTree, FolderPlus, FolderSearch, RefreshCw, Trash2, Star, StarOff,
  Search, Plus, Folder, AlertTriangle,
} from 'lucide-vue-next'
import AppDialog from '@/components/common/AppDialog.vue'

const { t } = useI18n()
const projectStore = useProjectStore()
const uiStore = useUiStore()

type FilterKey = 'all' | 'scanned' | 'manual' | 'favorites' | 'missing'
const activeFilter = ref<FilterKey>('all')

const showAddManualDialog = ref(false)
const manualName = ref('')
const manualPath = ref('')

onMounted(async () => {
  await projectStore.fetchAll()
})

const visibleProjects = computed(() => {
  let list = projectStore.sortedProjects
  if (activeFilter.value === 'scanned') list = list.filter(p => p.source.kind === 'scanned')
  else if (activeFilter.value === 'manual') list = list.filter(p => p.source.kind === 'manual')
  else if (activeFilter.value === 'favorites') list = list.filter(p => p.favorite)
  else if (activeFilter.value === 'missing') list = list.filter(p => p.missing)
  return list
})

const counts = computed(() => ({
  all: projectStore.projects.length,
  scanned: projectStore.projects.filter(p => p.source.kind === 'scanned').length,
  manual: projectStore.projects.filter(p => p.source.kind === 'manual').length,
  favorites: projectStore.projects.filter(p => p.favorite).length,
  missing: projectStore.projects.filter(p => p.missing).length,
}))

const filters = computed(() => [
  { key: 'all' as const, label: t('common.all'), count: counts.value.all },
  { key: 'scanned' as const, label: t('projects.filterScanned'), count: counts.value.scanned },
  { key: 'manual' as const, label: t('projects.filterManual'), count: counts.value.manual },
  { key: 'favorites' as const, label: t('projects.filterFavorites'), count: counts.value.favorites },
  { key: 'missing' as const, label: t('projects.filterMissing'), count: counts.value.missing },
])

async function pickRoot() {
  const folder = await openDialog({ directory: true, multiple: false, title: t('projects.selectRoot') })
  if (!folder) return
  try {
    await projectStore.addRoot(folder as string)
    uiStore.showToast('success', t('projects.rootAdded'))
    await scanAll()
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function pickManualFolder() {
  const folder = await openDialog({ directory: true, multiple: false, title: t('projects.selectManual') })
  if (!folder) return
  manualPath.value = folder as string
  if (!manualName.value) {
    const parts = (folder as string).split(/[\\/]/).filter(Boolean)
    manualName.value = parts[parts.length - 1] || ''
  }
}

async function submitManual() {
  if (!manualPath.value.trim()) return
  try {
    const name = manualName.value.trim() || manualPath.value.split(/[\\/]/).filter(Boolean).pop() || ''
    await projectStore.addManual(name, manualPath.value.trim())
    uiStore.showToast('success', t('common.success'))
    showAddManualDialog.value = false
    manualName.value = ''
    manualPath.value = ''
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function scanAll() {
  try {
    const summaries = await projectStore.scanAll()
    const totalAdded = summaries.reduce((s, x) => s + x.added, 0)
    const totalScanned = summaries.reduce((s, x) => s + x.scanned, 0)
    uiStore.showToast(
      'success',
      t('projects.scanResult', { found: totalScanned, added: totalAdded }),
    )
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function removeRoot(rootId: string) {
  try {
    await projectStore.removeRoot(rootId)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function removeProject(id: string) {
  try {
    await projectStore.removeProject(id)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

async function toggleFav(id: string) {
  try {
    await projectStore.toggleFavorite(id)
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}

function formatRelative(iso?: string): string {
  if (!iso) return t('projects.never')
  const date = new Date(iso)
  const diff = Date.now() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return t('projects.justNow')
  if (minutes < 60) return t('projects.minutesAgo', { n: minutes })
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return t('projects.hoursAgo', { n: hours })
  const days = Math.floor(hours / 24)
  return t('projects.daysAgo', { n: days })
}
</script>

<template>
  <div class="space-y-6 animate-fade-in-up">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
          <FolderTree :size="20" class="text-primary-600 dark:text-primary-400" />
        </div>
        <div>
          <h1 class="text-xl font-bold text-gray-900 dark:text-white">{{ t('projects.title') }}</h1>
          <p class="text-xs text-gray-500 dark:text-gray-400">{{ t('projects.subtitle') }}</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          @click="showAddManualDialog = true"
          class="cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
        >
          <Plus :size="14" />
          {{ t('projects.addManual') }}
        </button>
        <button
          @click="pickRoot"
          class="cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
        >
          <FolderPlus :size="14" />
          {{ t('projects.addRoot') }}
        </button>
        <button
          @click="scanAll"
          :disabled="projectStore.scanning || projectStore.roots.length === 0"
          class="cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          <RefreshCw :size="14" :class="{ 'animate-spin': projectStore.scanning }" />
          {{ t('projects.scanNow') }}
        </button>
      </div>
    </div>

    <!-- Scan roots -->
    <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm overflow-hidden">
      <div class="px-4 py-3 border-b border-gray-100 dark:border-gray-700 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <FolderSearch :size="16" class="text-gray-500" />
          <span class="text-sm font-semibold text-gray-700 dark:text-gray-200">{{ t('projects.scanRoots') }}</span>
        </div>
        <span class="text-xs text-gray-400">{{ projectStore.roots.length }}</span>
      </div>
      <div v-if="projectStore.roots.length === 0" class="px-4 py-8 text-center text-sm text-gray-400">
        {{ t('projects.noRoots') }}
      </div>
      <ul v-else class="divide-y divide-gray-50 dark:divide-gray-700/50">
        <li v-for="root in projectStore.roots" :key="root.id" class="px-4 py-3 flex items-center gap-3">
          <Folder :size="16" class="text-gray-400 shrink-0" />
          <div class="flex-1 min-w-0">
            <div class="text-sm text-gray-900 dark:text-white truncate">{{ root.path }}</div>
            <div class="text-[11px] text-gray-400 mt-0.5">
              {{ t('projects.depth') }}: {{ root.max_depth }} ·
              {{ t('projects.lastScanned') }}: {{ formatRelative(root.last_scanned_at) }}
            </div>
          </div>
          <button
            @click="projectStore.scanRoot(root.id)"
            :disabled="projectStore.scanning"
            class="p-1.5 rounded-lg text-gray-400 hover:text-primary-500 hover:bg-primary-50 dark:hover:bg-primary-900/20 transition-colors"
            :title="t('projects.scanThis')"
          >
            <RefreshCw :size="14" :class="{ 'animate-spin': projectStore.scanning }" />
          </button>
          <button
            @click="removeRoot(root.id)"
            class="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            :title="t('common.delete')"
          >
            <Trash2 :size="14" />
          </button>
        </li>
      </ul>
    </section>

    <!-- Search + filters -->
    <div class="flex items-center gap-3">
      <div class="relative flex-1">
        <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          v-model="projectStore.query"
          type="text"
          :placeholder="t('projects.searchPlaceholder')"
          class="w-full pl-9 pr-3 py-2 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
        />
      </div>
      <div class="flex items-center bg-gray-100 dark:bg-gray-800 rounded-full p-1">
        <button
          v-for="f in filters"
          :key="f.key"
          @click="activeFilter = f.key"
          :class="[
            'px-3 py-1 rounded-full text-xs font-medium transition-colors',
            activeFilter === f.key
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200',
          ]"
        >
          {{ f.label }}
          <span class="ml-1 text-gray-400">{{ f.count }}</span>
        </button>
      </div>
    </div>

    <!-- Project list -->
    <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-100 dark:border-gray-700 shadow-sm overflow-hidden">
      <div v-if="projectStore.loading" class="px-4 py-8 text-center text-sm text-gray-400">
        {{ t('common.loading') }}
      </div>
      <div v-else-if="visibleProjects.length === 0" class="px-4 py-12 text-center">
        <div class="text-sm text-gray-400 mb-2">{{ t('projects.empty') }}</div>
        <p class="text-xs text-gray-400">{{ t('projects.emptyHint') }}</p>
      </div>
      <ul v-else class="divide-y divide-gray-50 dark:divide-gray-700/50">
        <li
          v-for="proj in visibleProjects"
          :key="proj.id"
          class="px-4 py-3 flex items-center gap-3 hover:bg-gray-50/50 dark:hover:bg-gray-700/30 transition-colors"
        >
          <button
            @click="toggleFav(proj.id)"
            class="shrink-0 p-1 rounded text-gray-300 hover:text-amber-400 transition-colors"
            :title="proj.favorite ? t('projects.unfavorite') : t('projects.favorite')"
          >
            <Star v-if="proj.favorite" :size="16" class="text-amber-400 fill-amber-400" />
            <StarOff v-else :size="16" />
          </button>

          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-gray-900 dark:text-white truncate">{{ proj.name }}</span>
              <span
                v-if="proj.source.kind === 'scanned'"
                class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300"
              >
                {{ t('projects.badgeScanned') }}
              </span>
              <span
                v-else
                class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300"
              >
                {{ t('projects.badgeManual') }}
              </span>
              <span
                v-if="proj.missing"
                class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-bold bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300"
              >
                <AlertTriangle :size="10" />
                {{ t('projects.badgeMissing') }}
              </span>
            </div>
            <div class="text-xs text-gray-500 truncate mt-0.5">{{ proj.path }}</div>
          </div>

          <button
            @click="removeProject(proj.id)"
            class="shrink-0 p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            :title="t('common.delete')"
          >
            <Trash2 :size="14" />
          </button>
        </li>
      </ul>
    </section>

    <!-- Add manual dialog -->
    <AppDialog
      :open="showAddManualDialog"
      :title="t('projects.addManualTitle')"
      @close="showAddManualDialog = false"
    >
      <div class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{{ t('projects.name') }}</label>
          <input
            v-model="manualName"
            type="text"
            :placeholder="t('projects.namePlaceholder')"
            class="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{{ t('projects.path') }}</label>
          <div class="flex gap-2">
            <input
              v-model="manualPath"
              type="text"
              :placeholder="t('projects.pathPlaceholder')"
              class="flex-1 px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
            />
            <button
              @click="pickManualFolder"
              class="px-3 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            >
              <Folder :size="14" />
            </button>
          </div>
        </div>
        <div class="flex gap-2 pt-2">
          <button
            @click="showAddManualDialog = false"
            class="flex-1 px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            {{ t('common.cancel') }}
          </button>
          <button
            @click="submitManual"
            :disabled="!manualPath.trim()"
            class="flex-1 px-4 py-2 rounded-lg bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {{ t('common.add') }}
          </button>
        </div>
      </div>
    </AppDialog>
  </div>
</template>
