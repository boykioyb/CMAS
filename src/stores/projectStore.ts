import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Project, ScanRoot, ScanSummary } from '@/types'

export const useProjectStore = defineStore('projects', () => {
  const projects = ref<Project[]>([])
  const roots = ref<ScanRoot[]>([])
  const loading = ref(false)
  const scanning = ref(false)
  const query = ref('')

  const projectById = computed(() => {
    const map = new Map<string, Project>()
    for (const p of projects.value) map.set(p.id, p)
    return map
  })

  const filteredProjects = computed(() => {
    const q = query.value.trim().toLowerCase()
    if (!q) return projects.value
    return projects.value.filter(
      p => p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q)
    )
  })

  const sortedProjects = computed(() => {
    return [...filteredProjects.value].sort((a, b) => {
      if (a.favorite !== b.favorite) return a.favorite ? -1 : 1
      if (a.missing !== b.missing) return a.missing ? 1 : -1
      return a.name.localeCompare(b.name)
    })
  })

  async function fetchAll() {
    loading.value = true
    try {
      const [projs, rts] = await Promise.all([
        invoke<Project[]>('list_projects'),
        invoke<ScanRoot[]>('list_scan_roots'),
      ])
      projects.value = projs
      roots.value = rts
    } finally {
      loading.value = false
    }
  }

  async function addRoot(path: string): Promise<ScanRoot> {
    const root = await invoke<ScanRoot>('add_scan_root', { path })
    if (!roots.value.some(r => r.id === root.id)) roots.value.push(root)
    return root
  }

  async function removeRoot(rootId: string): Promise<void> {
    await invoke('remove_scan_root', { rootId })
    roots.value = roots.value.filter(r => r.id !== rootId)
    await fetchAll()
  }

  async function updateRoot(rootId: string, patch: { max_depth?: number; enabled?: boolean }): Promise<ScanRoot> {
    const updated = await invoke<ScanRoot>('update_scan_root', {
      rootId,
      maxDepth: patch.max_depth ?? null,
      enabled: patch.enabled ?? null,
    })
    const idx = roots.value.findIndex(r => r.id === rootId)
    if (idx >= 0) roots.value[idx] = updated
    return updated
  }

  async function scanAll(): Promise<ScanSummary[]> {
    scanning.value = true
    try {
      const summaries = await invoke<ScanSummary[]>('scan_projects')
      await fetchAll()
      return summaries
    } finally {
      scanning.value = false
    }
  }

  async function scanRoot(rootId: string): Promise<ScanSummary> {
    scanning.value = true
    try {
      const summary = await invoke<ScanSummary>('scan_single_root', { rootId })
      await fetchAll()
      return summary
    } finally {
      scanning.value = false
    }
  }

  async function addManual(name: string, path: string): Promise<Project> {
    const project = await invoke<Project>('add_manual_project', { name, path })
    const idx = projects.value.findIndex(p => p.id === project.id)
    if (idx >= 0) projects.value[idx] = project
    else projects.value.push(project)
    return project
  }

  async function removeProject(id: string): Promise<void> {
    await invoke('remove_project', { projectId: id })
    projects.value = projects.value.filter(p => p.id !== id)
  }

  async function toggleFavorite(id: string): Promise<Project> {
    const updated = await invoke<Project>('toggle_project_favorite', { projectId: id })
    const idx = projects.value.findIndex(p => p.id === id)
    if (idx >= 0) projects.value[idx] = updated
    return updated
  }

  function search(q: string, limit = 50): Project[] {
    const needle = q.trim().toLowerCase()
    if (!needle) return projects.value.slice(0, limit)
    return projects.value
      .filter(p => p.name.toLowerCase().includes(needle) || p.path.toLowerCase().includes(needle))
      .slice(0, limit)
  }

  return {
    projects,
    roots,
    loading,
    scanning,
    query,
    projectById,
    filteredProjects,
    sortedProjects,
    fetchAll,
    addRoot,
    removeRoot,
    updateRoot,
    scanAll,
    scanRoot,
    addManual,
    removeProject,
    toggleFavorite,
    search,
  }
})
