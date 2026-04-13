import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppConfig } from '@/types'

export const useConfigStore = defineStore('config', () => {
  const config = ref<AppConfig>({
    language: 'en',
    theme: 'system',
    vscode_path: '/usr/local/bin/code',
    quota_warning_threshold: 20,
    auto_switch_on_empty: false,
    launch_at_login: false,
    claude_config_path: '~/.claude/.claude.json',
    claude_cli_path: '',
    backup_dir: '~/.claude-switcher/',
    usage_refresh_interval: 300,
  })
  const loaded = ref(false)

  async function loadConfig() {
    try {
      config.value = await invoke<AppConfig>('get_app_config')
    } catch (e) {
      console.warn('Using default config:', e)
    }
    loaded.value = true
  }

  async function saveConfig() {
    await invoke('save_app_config', { config: config.value })
  }

  async function updateConfig(partial: Partial<AppConfig>) {
    Object.assign(config.value, partial)
    await saveConfig()
  }

  async function detectVscode() {
    const path = await invoke<string | null>('find_vscode')
    if (path) {
      config.value.vscode_path = path
      await saveConfig()
    }
    return path
  }

  async function detectCli() {
    const path = await invoke<string | null>('find_claude_cli')
    if (path) {
      config.value.claude_cli_path = path
      await saveConfig()
    }
    return path
  }

  function applyTheme(theme: string) {
    const root = document.documentElement
    if (theme === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
      root.classList.toggle('dark', prefersDark)
    } else {
      root.classList.toggle('dark', theme === 'dark')
    }
  }

  return {
    config,
    loaded,
    loadConfig,
    saveConfig,
    updateConfig,
    detectVscode,
    detectCli,
    applyTheme,
  }
})
