<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useProjectStore } from '@/stores/projectStore'
import { useUiStore } from '@/stores/uiStore'
import type { Project } from '@/types'
import { Folder, ChevronDown, Plus, Star, AlertTriangle, Search, Check, X } from 'lucide-vue-next'

const props = defineProps<{
  linkedIds: string[]
  selectedId?: string | null
  accountId: string
}>()

const emit = defineEmits<{
  pick: [project: Project]
  unlink: [projectId: string]
  selectionChange: [projectId: string | null]
}>()

const { t } = useI18n()
const projectStore = useProjectStore()
const uiStore = useUiStore()

const open = ref(false)
const search = ref('')
const triggerRef = ref<HTMLElement | null>(null)
const dropdownRef = ref<HTMLElement | null>(null)
const dropdownStyle = ref<{ top: string; left: string; width: string }>({
  top: '0px', left: '0px', width: '320px',
})

const DROPDOWN_WIDTH = 340
const DROPDOWN_MAX_HEIGHT = 380

function updatePosition() {
  if (!triggerRef.value) return
  const rect = triggerRef.value.getBoundingClientRect()
  const viewportH = window.innerHeight
  const viewportW = window.innerWidth

  // Prefer below; if no room, flip above
  const spaceBelow = viewportH - rect.bottom
  const showAbove = spaceBelow < DROPDOWN_MAX_HEIGHT && rect.top > spaceBelow

  let left = rect.left
  if (left + DROPDOWN_WIDTH > viewportW - 8) {
    left = Math.max(8, viewportW - DROPDOWN_WIDTH - 8)
  }

  dropdownStyle.value = {
    top: showAbove
      ? `${Math.max(8, rect.top - DROPDOWN_MAX_HEIGHT - 4)}px`
      : `${rect.bottom + 4}px`,
    left: `${left}px`,
    width: `${DROPDOWN_WIDTH}px`,
  }
}

onMounted(async () => {
  if (projectStore.projects.length === 0) {
    await projectStore.fetchAll()
  }
  document.addEventListener('mousedown', handleOutside, true)
  window.addEventListener('resize', handleViewportChange)
  window.addEventListener('scroll', handleViewportChange, true)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleOutside, true)
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
})

function handleViewportChange() {
  if (open.value) updatePosition()
}

function handleOutside(e: MouseEvent) {
  const target = e.target as Node
  if (triggerRef.value?.contains(target)) return
  if (dropdownRef.value?.contains(target)) return
  open.value = false
}

const selectedProject = computed(() => {
  if (!props.selectedId) return null
  return projectStore.projectById.get(props.selectedId) || null
})

const isLinked = (id: string) => props.linkedIds.includes(id)

const orderedProjects = computed(() => {
  const all = projectStore.search(search.value, 200)
  return [...all].sort((a, b) => {
    const al = isLinked(a.id) ? 0 : 1
    const bl = isLinked(b.id) ? 0 : 1
    if (al !== bl) return al - bl
    if (a.favorite !== b.favorite) return a.favorite ? -1 : 1
    if (a.missing !== b.missing) return a.missing ? 1 : -1
    return a.name.localeCompare(b.name)
  })
})

watch(open, async (val) => {
  if (val) {
    search.value = ''
    await nextTick()
    updatePosition()
  }
})

function toggle() {
  open.value = !open.value
}

function pickProject(p: Project) {
  if (isLinked(p.id)) {
    emit('selectionChange', p.id)
  } else {
    emit('pick', p)
  }
  open.value = false
}

function unlink(e: Event, projectId: string) {
  e.stopPropagation()
  emit('unlink', projectId)
}

async function quickAddManual() {
  const folder = await openDialog({ directory: true, multiple: false, title: t('projects.selectManual') })
  if (!folder) return
  try {
    const folderPath = folder as string
    const parts = folderPath.split(/[\\/]/).filter(Boolean)
    const name = parts[parts.length - 1] || folderPath
    const project = await projectStore.addManual(name, folderPath)
    emit('pick', project)
    open.value = false
  } catch (e) {
    uiStore.showToast('error', String(e))
  }
}
</script>

<template>
  <div class="relative min-w-[200px]">
    <!-- Trigger button -->
    <button
      ref="triggerRef"
      type="button"
      @click="toggle"
      class="w-full flex items-center gap-2 px-2 py-1 text-xs border border-gray-200 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 hover:border-primary-400 dark:hover:border-primary-500 transition-colors"
    >
      <Folder :size="12" class="text-gray-400 shrink-0" />
      <span
        v-if="selectedProject"
        class="flex-1 min-w-0 text-left truncate text-gray-900 dark:text-white"
        :class="{ 'text-red-500': selectedProject.missing }"
      >
        {{ selectedProject.name }}
        <AlertTriangle v-if="selectedProject.missing" :size="10" class="inline text-red-400" />
      </span>
      <span v-else class="flex-1 text-left text-gray-400 italic">{{ t('accounts.noProjects') }}</span>
      <span v-if="linkedIds.length > 1" class="text-[10px] text-gray-400">
        {{ linkedIds.length }}
      </span>
      <ChevronDown :size="12" class="text-gray-400 shrink-0 transition-transform" :class="{ 'rotate-180': open }" />
    </button>

    <!-- Dropdown teleported to body to escape table overflow clipping -->
    <Teleport to="body">
      <div
        v-if="open"
        ref="dropdownRef"
        :style="dropdownStyle"
        class="fixed z-[100] bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-2xl overflow-hidden flex flex-col"
      >
        <div class="p-2 border-b border-gray-100 dark:border-gray-700 shrink-0">
          <div class="relative">
            <Search :size="12" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              v-model="search"
              type="text"
              :placeholder="t('projects.searchPlaceholder')"
              class="w-full pl-7 pr-2 py-1.5 text-xs rounded-md border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-1 focus:ring-primary-500 outline-none"
              autofocus
            />
          </div>
        </div>

        <div class="overflow-y-auto" :style="{ maxHeight: '280px' }">
          <button
            v-for="p in orderedProjects"
            :key="p.id"
            type="button"
            @click="pickProject(p)"
            class="w-full text-left px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors flex items-center gap-2 group"
            :class="{ 'bg-primary-50/50 dark:bg-primary-900/10': p.id === selectedId }"
          >
            <span class="shrink-0 w-4 flex items-center justify-center">
              <Check v-if="p.id === selectedId" :size="14" class="text-primary-500" />
              <Check v-else-if="isLinked(p.id)" :size="14" class="text-emerald-500" />
              <Folder v-else :size="13" class="text-gray-300" />
            </span>

            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-1.5">
                <span class="text-xs font-medium text-gray-900 dark:text-white truncate">{{ p.name }}</span>
                <Star v-if="p.favorite" :size="10" class="text-amber-400 fill-amber-400 shrink-0" />
                <AlertTriangle v-if="p.missing" :size="10" class="text-red-400 shrink-0" />
                <span
                  class="px-1 rounded text-[9px] font-bold"
                  :class="p.source.kind === 'scanned'
                    ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
                    : 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'"
                >
                  {{ p.source.kind === 'scanned' ? t('projects.badgeScanned') : t('projects.badgeManual') }}
                </span>
              </div>
              <div class="text-[10px] text-gray-500 truncate">{{ p.path }}</div>
            </div>

            <button
              v-if="isLinked(p.id) && p.id !== selectedId"
              type="button"
              @click="(e) => unlink(e, p.id)"
              class="shrink-0 p-0.5 rounded text-gray-300 hover:bg-red-50 hover:text-red-500 dark:hover:bg-red-900/20 transition-colors"
              :title="t('projects.unlink')"
            >
              <X :size="12" />
            </button>
          </button>

          <div v-if="orderedProjects.length === 0" class="px-3 py-6 text-center text-xs text-gray-400">
            {{ t('projects.noMatches') }}
          </div>
        </div>

        <div class="border-t border-gray-100 dark:border-gray-700 shrink-0">
          <button
            type="button"
            @click="quickAddManual"
            class="w-full flex items-center gap-2 px-3 py-2 text-xs font-medium text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 transition-colors"
          >
            <Plus :size="12" />
            {{ t('projects.addManual') }}
          </button>
        </div>
      </div>
    </Teleport>
  </div>
</template>
