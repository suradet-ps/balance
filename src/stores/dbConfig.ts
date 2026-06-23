import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// ─── HOSxP (MySQL) Config ──────────────────────────────────────────────

export interface HosxpDbConfig {
  host: string
  port: number
  user: string
  password: string
  database: string
}

const HOSXP_STORAGE_KEY = 'balance_hosxp_db_config'

// ─── INVS (SQL Server) Config ──────────────────────────────────────────

export interface InvsDbConfig {
  host: string
  port: number
  user: string
  password: string
  database: string
  instance: string
}

const INVS_STORAGE_KEY = 'balance_invs_db_config'

// ─── Combined Store ────────────────────────────────────────────────────

export const useDbConfigStore = defineStore('dbConfig', () => {
  // HOSxP state
  const hosxpConfig = ref<HosxpDbConfig>({
    host: 'localhost',
    port: 3306,
    user: '',
    password: '',
    database: 'hospdb',
  })
  const hosxpConnected = ref(false)
  const hosxpConnecting = ref(false)
  const hosxpError = ref<string | null>(null)

  // INVS state
  const invsConfig = ref<InvsDbConfig>({
    host: 'localhost',
    port: 1433,
    user: '',
    password: '',
    database: 'INVS',
    instance: '',
  })
  const invsConnected = ref(false)
  const invsConnecting = ref(false)
  const invsError = ref<string | null>(null)

  const showSettings = ref(false)
  const activeTab = ref<'hosxp' | 'invs'>('hosxp')

  // Computed
  const hosxpConfigured = computed(
    () => hosxpConfig.value.host.trim() !== '' && hosxpConfig.value.user.trim() !== ''
  )
  const invsConfigured = computed(
    () => invsConfig.value.host.trim() !== '' && invsConfig.value.user.trim() !== ''
  )
  const bothConnected = computed(() => hosxpConnected.value && invsConnected.value)
  const anyConnected = computed(() => hosxpConnected.value || invsConnected.value)

  // HOSxP methods
  function loadHosxpFromStorage() {
    try {
      const raw = localStorage.getItem(HOSXP_STORAGE_KEY)
      if (raw) {
        const saved = JSON.parse(raw) as Partial<HosxpDbConfig>
        if (saved.host) hosxpConfig.value.host = saved.host
        if (saved.port) hosxpConfig.value.port = saved.port
        if (saved.user) hosxpConfig.value.user = saved.user
        if (saved.password) hosxpConfig.value.password = saved.password
        if (saved.database) hosxpConfig.value.database = saved.database
      }
    } catch { /* ignore */ }
  }

  function saveHosxpToStorage() {
    localStorage.setItem(HOSXP_STORAGE_KEY, JSON.stringify(hosxpConfig.value))
  }

  async function connectHosxp(): Promise<boolean> {
    hosxpConnecting.value = true
    hosxpError.value = null
    try {
      await invoke('hosxp_connect', { config: hosxpConfig.value })
      hosxpConnected.value = true
      saveHosxpToStorage()
      return true
    } catch (e) {
      hosxpConnected.value = false
      hosxpError.value = String(e)
      return false
    } finally {
      hosxpConnecting.value = false
    }
  }

  // INVS methods
  function loadInvsFromStorage() {
    try {
      const raw = localStorage.getItem(INVS_STORAGE_KEY)
      if (raw) {
        const saved = JSON.parse(raw) as Partial<InvsDbConfig>
        if (saved.host) invsConfig.value.host = saved.host
        if (saved.port) invsConfig.value.port = saved.port
        if (saved.user) invsConfig.value.user = saved.user
        if (saved.password) invsConfig.value.password = saved.password
        if (saved.database) invsConfig.value.database = saved.database
        if (saved.instance) invsConfig.value.instance = saved.instance
      }
    } catch { /* ignore */ }
  }

  function saveInvsToStorage() {
    localStorage.setItem(INVS_STORAGE_KEY, JSON.stringify(invsConfig.value))
  }

  async function connectInvs(): Promise<boolean> {
    invsConnecting.value = true
    invsError.value = null
    try {
      await invoke('invs_connect', { cfg: invsConfig.value })
      invsConnected.value = true
      saveInvsToStorage()
      return true
    } catch (e) {
      invsConnected.value = false
      invsError.value = String(e)
      return false
    } finally {
      invsConnecting.value = false
    }
  }

  // Init
  async function initFromStorage() {
    loadHosxpFromStorage()
    loadInvsFromStorage()

    if (hosxpConfig.value.user) {
      connectHosxp().catch(() => {})
    }
    if (invsConfig.value.user) {
      connectInvs().catch(() => {})
    }
  }

  return {
    // HOSxP
    hosxpConfig, hosxpConnected, hosxpConnecting, hosxpError, hosxpConfigured,
    connectHosxp,
    // INVS
    invsConfig, invsConnected, invsConnecting, invsError, invsConfigured,
    connectInvs,
    // Combined
    showSettings, activeTab,
    bothConnected, anyConnected,
    initFromStorage,
  }
})
