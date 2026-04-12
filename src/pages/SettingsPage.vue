<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/configStore'
import { useUiStore } from '@/stores/uiStore'
import { Settings, User, Code, Info, Sun, Moon, Monitor, Search as SearchIcon } from 'lucide-vue-next'

const { t, locale } = useI18n()
const configStore = useConfigStore()
const uiStore = useUiStore()

const activeTab = ref('general')
const detecting = ref(false)

const tabs = computed(() => [
  { key: 'general', label: t('settings.tabs.general'), icon: Settings },
  { key: 'accounts', label: t('settings.tabs.accounts'), icon: User },
  { key: 'vscode', label: t('settings.tabs.vscode'), icon: Code },
  { key: 'about', label: t('settings.tabs.about'), icon: Info },
])

async function detectVscode() {
  detecting.value = true
  try {
    const path = await configStore.detectVscode()
    if (path) {
      uiStore.showToast('success', `${t('settings.vscode.detected')}: ${path}`)
    } else {
      uiStore.showToast('warning', t('settings.vscode.notFound'))
    }
  } catch (e) {
    uiStore.showToast('error', String(e))
  } finally {
    detecting.value = false
  }
}

function setLanguage(lang: 'vi' | 'en') {
  locale.value = lang
  configStore.updateConfig({ language: lang })
}

function setTheme(theme: 'light' | 'dark' | 'system') {
  configStore.updateConfig({ theme })
}
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-xl font-bold text-gray-900 dark:text-white animate-fade-in-up">{{ t('settings.title') }}</h1>

    <!-- Tab pills -->
    <div class="flex items-center bg-gray-100 dark:bg-gray-800 rounded-full p-1 w-fit animate-fade-in-up delay-1">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        @click="activeTab = tab.key"
        :class="[
          'flex items-center gap-2 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200',
          activeTab === tab.key
            ? 'bg-gray-900 text-white dark:bg-white dark:text-gray-900 shadow-sm'
            : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
        ]"
      >
        <component :is="tab.icon" :size="15" />
        <span>{{ tab.label }}</span>
      </button>
    </div>

    <!-- Content -->
    <div class="bg-white dark:bg-gray-800 rounded-2xl border border-gray-100 dark:border-gray-700 shadow-sm p-6 animate-fade-in-up delay-2">

      <transition name="tab-content" mode="out-in">
      <!-- General -->
      <div v-if="activeTab === 'general'" key="general" class="space-y-6">
        <!-- Language -->
        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-1">{{ t('settings.general.language') }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">{{ t('settings.general.languageDesc') }}</p>
          <div class="flex gap-2">
            <button
              @click="setLanguage('vi')"
              :class="['px-4 py-2 rounded-lg text-sm font-medium border transition-colors', configStore.config.language === 'vi' ? 'bg-primary-50 dark:bg-primary-900/30 border-primary-300 dark:border-primary-700 text-primary-700 dark:text-primary-300' : 'border-gray-200 dark:border-gray-600 text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-700']"
            >{{ t('settings.general.vietnamese') }}</button>
            <button
              @click="setLanguage('en')"
              :class="['px-4 py-2 rounded-lg text-sm font-medium border transition-colors', configStore.config.language === 'en' ? 'bg-primary-50 dark:bg-primary-900/30 border-primary-300 dark:border-primary-700 text-primary-700 dark:text-primary-300' : 'border-gray-200 dark:border-gray-600 text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-700']"
            >{{ t('settings.general.english') }}</button>
          </div>
        </div>

        <hr class="border-gray-100 dark:border-gray-700" />

        <!-- Theme -->
        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-1">{{ t('settings.general.theme') }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-3">{{ t('settings.general.themeDesc') }}</p>
          <div class="flex gap-2">
            <button
              v-for="th in [
                { key: 'light' as const, label: t('settings.general.light'), icon: Sun },
                { key: 'dark' as const, label: t('settings.general.dark'), icon: Moon },
                { key: 'system' as const, label: t('settings.general.system'), icon: Monitor },
              ]"
              :key="th.key"
              @click="setTheme(th.key)"
              :class="['flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium border transition-colors', configStore.config.theme === th.key ? 'bg-primary-50 dark:bg-primary-900/30 border-primary-300 dark:border-primary-700 text-primary-700 dark:text-primary-300' : 'border-gray-200 dark:border-gray-600 text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-700']"
            >
              <component :is="th.icon" :size="15" />
              {{ th.label }}
            </button>
          </div>
        </div>
      </div>

      <!-- Account settings -->
      <div v-else-if="activeTab === 'accounts'" key="accounts" class="space-y-6">
        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-1">{{ t('settings.accountSettings.claudeConfigPath') }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{{ t('settings.accountSettings.claudeConfigPathDesc') }}</p>
          <input
            v-model="configStore.config.claude_config_path"
            @change="configStore.saveConfig()"
            class="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white font-mono focus:ring-2 focus:ring-primary-500 outline-none"
          />
        </div>

        <hr class="border-gray-100 dark:border-gray-700" />

        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-1">{{ t('settings.accountSettings.quotaThreshold') }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{{ t('settings.accountSettings.quotaThresholdDesc') }}</p>
          <div class="flex items-center gap-4">
            <input
              v-model.number="configStore.config.quota_warning_threshold"
              type="range"
              min="5"
              max="50"
              class="flex-1 accent-primary-500"
              @change="configStore.saveConfig()"
            />
            <span class="text-sm font-semibold text-gray-900 dark:text-white w-12 text-right">
              {{ configStore.config.quota_warning_threshold }}%
            </span>
          </div>
        </div>

        <hr class="border-gray-100 dark:border-gray-700" />

        <!-- Usage refresh interval -->
        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-1">{{ t('settings.accountSettings.usageRefresh') }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{{ t('settings.accountSettings.usageRefreshDesc') }}</p>
          <div class="flex items-center gap-3">
            <select
              v-model.number="configStore.config.usage_refresh_interval"
              @change="configStore.saveConfig()"
              class="px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-sm text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 outline-none"
            >
              <option :value="0">{{ t('settings.accountSettings.off') }}</option>
              <option :value="60">{{ t('settings.accountSettings.nMin', { n: 1 }) }}</option>
              <option :value="120">{{ t('settings.accountSettings.nMin', { n: 2 }) }}</option>
              <option :value="300">{{ t('settings.accountSettings.nMin', { n: 5 }) }}</option>
              <option :value="600">{{ t('settings.accountSettings.nMin', { n: 10 }) }}</option>
              <option :value="900">{{ t('settings.accountSettings.nMin', { n: 15 }) }}</option>
              <option :value="1800">{{ t('settings.accountSettings.nMin', { n: 30 }) }}</option>
            </select>
          </div>
        </div>

        <hr class="border-gray-100 dark:border-gray-700" />

        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-sm font-semibold text-gray-900 dark:text-white">{{ t('settings.accountSettings.autoSwitch') }}</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">{{ t('settings.accountSettings.autoSwitchDesc') }}</p>
          </div>
          <button
            @click="configStore.config.auto_switch_on_empty = !configStore.config.auto_switch_on_empty; configStore.saveConfig()"
            :class="[
              'relative w-11 h-6 rounded-full transition-colors',
              configStore.config.auto_switch_on_empty ? 'bg-primary-500' : 'bg-gray-300 dark:bg-gray-600'
            ]"
          >
            <span
              :class="[
                'absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform',
                configStore.config.auto_switch_on_empty ? 'translate-x-5' : 'translate-x-0'
              ]"
            />
          </button>
        </div>
      </div>

      <!-- VSCode settings -->
      <div v-else-if="activeTab === 'vscode'" key="vscode" class="space-y-6">
        <div>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-1">{{ t('settings.vscode.path') }}</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">{{ t('settings.vscode.pathDesc') }}</p>
          <div class="flex gap-2">
            <input
              v-model="configStore.config.vscode_path"
              @change="configStore.saveConfig()"
              class="flex-1 px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm text-gray-900 dark:text-white font-mono focus:ring-2 focus:ring-primary-500 outline-none"
            />
            <button
              @click="detectVscode"
              :disabled="detecting"
              class="px-4 py-2 rounded-lg border border-gray-200 dark:border-gray-600 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            >
              <SearchIcon :size="16" :class="{ 'animate-spin': detecting }" />
            </button>
          </div>
        </div>
      </div>

      <!-- About -->
      <div v-else-if="activeTab === 'about'" key="about" class="space-y-4">
        <div class="flex items-center gap-3 mb-6">
          <img src="@/assets/hero.png" alt="CMAS" class="w-12 h-12 rounded-xl" />
          <div>
            <h2 class="text-lg font-bold text-gray-900 dark:text-white">Claude Multi Account Switcher</h2>
            <p class="text-sm text-gray-500">{{ t('settings.about.version') }}: 1.0.0</p>
          </div>
        </div>

        <p class="text-sm text-gray-600 dark:text-gray-400">
          {{ t('settings.about.description') }}
        </p>

        <div class="pt-4 border-t border-gray-100 dark:border-gray-700">
          <p class="text-xs text-gray-400">{{ t('settings.about.license') }}</p>
          <p class="text-xs text-gray-500 mt-2">{{ t('settings.about.author') }}: <span class="font-medium">Hòa TQ</span></p>
          <div class="flex gap-3 mt-1">
            <a href="https://github.com/boykioyb" target="_blank" class="text-xs text-primary-500 hover:underline">GitHub</a>
            <a href="https://hoatq.dev" target="_blank" class="text-xs text-primary-500 hover:underline">hoatq.dev</a>
          </div>
        </div>
      </div>
      </transition>
    </div>
  </div>
</template>
