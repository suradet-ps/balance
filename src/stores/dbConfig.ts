import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

// ─── HOSxP (MySQL) Config ──────────────────────────────────────────────

export interface HosxpDbConfig {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
}

// ─── INVS (SQL Server) Config ──────────────────────────────────────────

export interface InvsDbConfig {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
  instance: string;
}

// ─── Combined Store ────────────────────────────────────────────────────

export const useDbConfigStore = defineStore('dbConfig', () => {
  // HOSxP state
  const hosxpConfig = ref<HosxpDbConfig>({
    host: 'localhost',
    port: 3306,
    user: '',
    password: '',
    database: 'hospdb',
  });
  const hosxpConnected = ref(false);
  const hosxpConnecting = ref(false);
  const hosxpError = ref<string | null>(null);

  // INVS state
  const invsConfig = ref<InvsDbConfig>({
    host: 'localhost',
    port: 1433,
    user: '',
    password: '',
    database: 'INVS',
    instance: '',
  });
  const invsConnected = ref(false);
  const invsConnecting = ref(false);
  const invsError = ref<string | null>(null);

  const showSettings = ref(false);
  const activeTab = ref<'hosxp' | 'invs'>('hosxp');

  // Save feedback
  const saving = ref(false);
  const saveMessage = ref<string | null>(null);

  // Computed
  const hosxpConfigured = computed(
    () => hosxpConfig.value.host.trim() !== '' && hosxpConfig.value.user.trim() !== '',
  );
  const invsConfigured = computed(
    () => invsConfig.value.host.trim() !== '' && invsConfig.value.user.trim() !== '',
  );
  const bothConnected = computed(() => hosxpConnected.value && invsConnected.value);
  const anyConnected = computed(() => hosxpConnected.value || invsConnected.value);

  // HOSxP methods
  async function connectHosxp(): Promise<boolean> {
    hosxpConnecting.value = true;
    hosxpError.value = null;
    try {
      await invoke('hosxp_connect', { config: hosxpConfig.value });
      hosxpConnected.value = true;
      return true;
    } catch (e) {
      hosxpConnected.value = false;
      hosxpError.value = String(e);
      return false;
    } finally {
      hosxpConnecting.value = false;
    }
  }

  // INVS methods
  async function connectInvs(): Promise<boolean> {
    invsConnecting.value = true;
    invsError.value = null;
    try {
      await invoke('invs_connect', { cfg: invsConfig.value });
      invsConnected.value = true;
      return true;
    } catch (e) {
      invsConnected.value = false;
      invsError.value = String(e);
      return false;
    } finally {
      invsConnecting.value = false;
    }
  }

  // Unified save — persists both configs via encrypted Tauri settings
  async function saveSettings(): Promise<boolean> {
    saving.value = true;
    saveMessage.value = null;
    try {
      await invoke('save_settings', {
        hosxp: hosxpConfig.value,
        invs: invsConfig.value.user ? invsConfig.value : null,
      });
      saveMessage.value = 'บันทึกสำเร็จ';
      setTimeout(() => {
        saveMessage.value = null;
      }, 3000);
      return true;
    } catch (e) {
      saveMessage.value = String(e);
      setTimeout(() => {
        saveMessage.value = null;
      }, 5000);
      return false;
    } finally {
      saving.value = false;
    }
  }

  // Init
  async function initFromStorage() {
    try {
      const settings = await invoke<{ hosxp: HosxpDbConfig; invs: InvsDbConfig | null }>(
        'load_settings',
      );
      hosxpConfig.value = settings.hosxp;
      if (settings.invs) invsConfig.value = settings.invs;
    } catch {
      /* no saved settings yet — use defaults */
    }

    if (hosxpConfig.value.user) {
      connectHosxp().catch(() => {});
    }
    if (invsConfig.value.user) {
      connectInvs().catch(() => {});
    }
  }

  return {
    // HOSxP
    hosxpConfig,
    hosxpConnected,
    hosxpConnecting,
    hosxpError,
    hosxpConfigured,
    connectHosxp,
    // INVS
    invsConfig,
    invsConnected,
    invsConnecting,
    invsError,
    invsConfigured,
    connectInvs,
    // Save
    saving,
    saveMessage,
    saveSettings,
    // Combined
    showSettings,
    activeTab,
    bothConnected,
    anyConnected,
    initFromStorage,
  };
});
