<template>
  <Teleport to="body">
    <Transition name="drawer">
      <div v-if="visible" class="drawer-overlay" @click.self="close">
        <div class="drawer-panel">
          <!-- Header -->
          <div class="drawer-header">
            <span class="drawer-title">
              <Settings2 :size="16" />
              ตั้งค่าการเชื่อมต่อฐานข้อมูล
            </span>
            <button class="btn-icon" @click="close"><X :size="16" /></button>
          </div>

          <!-- Tab bar -->
          <div class="tab-bar">
            <button
              :class="['tab-btn', { active: dbStore.activeTab === 'hosxp' }]"
              @click="dbStore.activeTab = 'hosxp'"
            >
              <Database :size="14" />
              HOSxP (MySQL)
            </button>
            <button
              :class="['tab-btn', { active: dbStore.activeTab === 'invs' }]"
              @click="dbStore.activeTab = 'invs'"
            >
              <Database :size="14" />
              INVS (SQL Server)
            </button>
          </div>

          <!-- HOSxP Form -->
          <div v-if="dbStore.activeTab === 'hosxp'" class="form-section">
            <div class="status-row">
              <span :class="['badge', dbStore.hosxpConnected ? 'badge-connected' : 'badge-disconnected']">
                <span :class="['status-dot', dbStore.hosxpConnected ? 'dot-green' : 'dot-red']" />
                {{ dbStore.hosxpConnected ? 'เชื่อมต่อแล้ว' : 'ยังไม่ได้เชื่อมต่อ' }}
              </span>
            </div>

            <div class="form-grid">
              <div class="form-group">
                <label class="form-label">Host / IP</label>
                <input v-model="dbStore.hosxpConfig.host" class="input" placeholder="localhost" autocomplete="off" />
              </div>
              <div class="form-group form-group--half">
                <label class="form-label">Port</label>
                <input v-model.number="dbStore.hosxpConfig.port" class="input" type="number" placeholder="3306" />
              </div>
              <div class="form-group form-group--half">
                <label class="form-label">Database</label>
                <input v-model="dbStore.hosxpConfig.database" class="input" placeholder="hospdb" />
              </div>
              <div class="form-group">
                <label class="form-label">Username</label>
                <input v-model="dbStore.hosxpConfig.user" class="input" placeholder="hosxp_user" autocomplete="username" />
              </div>
              <div class="form-group">
                <label class="form-label">Password</label>
                <div class="password-wrap">
                  <input
                    v-model="dbStore.hosxpConfig.password"
                    :type="showPasswordHosxp ? 'text' : 'password'"
                    class="input"
                    placeholder="••••••••"
                    autocomplete="current-password"
                  />
                  <button class="btn-icon small" @click="showPasswordHosxp = !showPasswordHosxp">
                    <EyeOff v-if="showPasswordHosxp" :size="14" />
                    <Eye v-else :size="14" />
                  </button>
                </div>
              </div>
            </div>

            <div v-if="dbStore.hosxpError" class="error-box">
              <AlertTriangle :size="14" />
              {{ dbStore.hosxpError }}
            </div>

            <div class="drawer-actions">
              <button class="btn btn-ghost" @click="close">ยกเลิก</button>
              <button class="btn btn-primary" :disabled="dbStore.hosxpConnecting" @click="connectHosxp">
                <span v-if="dbStore.hosxpConnecting" class="animate-pulse">กำลังเชื่อมต่อ…</span>
                <template v-else>
                  <PlugZap :size="14" />
                  ทดสอบ & บันทึก
                </template>
              </button>
            </div>
          </div>

          <!-- INVS Form -->
          <div v-if="dbStore.activeTab === 'invs'" class="form-section">
            <div class="status-row">
              <span :class="['badge', dbStore.invsConnected ? 'badge-connected' : 'badge-disconnected']">
                <span :class="['status-dot', dbStore.invsConnected ? 'dot-green' : 'dot-red']" />
                {{ dbStore.invsConnected ? 'เชื่อมต่อแล้ว' : 'ยังไม่ได้เชื่อมต่อ' }}
              </span>
            </div>

            <div class="form-grid">
              <div class="form-group">
                <label class="form-label">Server / Host IP</label>
                <input v-model="dbStore.invsConfig.host" class="input" placeholder="192.168.1.10" autocomplete="off" />
              </div>
              <div class="form-group form-group--half">
                <label class="form-label">Port</label>
                <input v-model.number="dbStore.invsConfig.port" class="input" type="number" placeholder="1433" />
              </div>
              <div class="form-group form-group--half">
                <label class="form-label">Named Instance</label>
                <input v-model="dbStore.invsConfig.instance" class="input" placeholder="(เว้นว่างถ้าไม่มี)" autocomplete="off" />
              </div>
              <div class="form-group">
                <label class="form-label">Database</label>
                <input v-model="dbStore.invsConfig.database" class="input" placeholder="INVS" />
              </div>
              <div class="form-group">
                <label class="form-label">Username</label>
                <input v-model="dbStore.invsConfig.user" class="input" placeholder="sa" autocomplete="username" />
              </div>
              <div class="form-group">
                <label class="form-label">Password</label>
                <div class="password-wrap">
                  <input
                    v-model="dbStore.invsConfig.password"
                    :type="showPasswordInvs ? 'text' : 'password'"
                    class="input"
                    placeholder="••••••••"
                    autocomplete="current-password"
                  />
                  <button class="btn-icon small" @click="showPasswordInvs = !showPasswordInvs">
                    <EyeOff v-if="showPasswordInvs" :size="14" />
                    <Eye v-else :size="14" />
                  </button>
                </div>
              </div>
            </div>

            <div v-if="dbStore.invsError" class="error-box">
              <AlertTriangle :size="14" />
              {{ dbStore.invsError }}
            </div>

            <div class="drawer-actions">
              <button class="btn btn-ghost" @click="close">ยกเลิก</button>
              <button class="btn btn-primary" :disabled="dbStore.invsConnecting" @click="connectInvs">
                <span v-if="dbStore.invsConnecting" class="animate-pulse">กำลังเชื่อมต่อ…</span>
                <template v-else>
                  <PlugZap :size="14" />
                  ทดสอบ & บันทึก
                </template>
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Settings2, X, Eye, EyeOff, AlertTriangle, PlugZap, Database } from 'lucide-vue-next'
import { useDbConfigStore } from '../stores/dbConfig'

defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

const dbStore = useDbConfigStore()
const showPasswordHosxp = ref(false)
const showPasswordInvs = ref(false)

function close() { emit('close') }

async function connectHosxp() {
  const ok = await dbStore.connectHosxp()
  if (ok) close()
}

async function connectInvs() {
  const ok = await dbStore.connectInvs()
  if (ok) close()
}
</script>

<style scoped>
.drawer-overlay {
  position: fixed;
  inset: 0;
  background: rgba(16, 17, 20, 0.5);
  display: flex;
  align-items: stretch;
  justify-content: flex-end;
  z-index: 1000;
}

.drawer-panel {
  width: 400px;
  background: var(--bg-base);
  height: 100%;
  display: flex;
  flex-direction: column;
  box-shadow: -6px 0 32px rgba(0, 0, 0, 0.15);
  padding: 20px;
  gap: 14px;
  overflow-y: auto;
}

.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.drawer-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.btn-icon {
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px 6px;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast), color var(--transition-fast);
}

.btn-icon:hover {
  background: var(--bg-elevated);
  color: var(--text-primary);
}

.btn-icon.small {
  position: absolute;
  right: 6px;
  padding: 0 6px;
}

/* Tabs */
.tab-bar {
  display: flex;
  gap: 4px;
  background: var(--bg-elevated);
  border-radius: var(--radius-lg);
  padding: 3px;
}

.tab-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  border: none;
  border-radius: var(--radius-md);
  font-family: var(--font-body);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  color: var(--text-secondary);
  background: transparent;
  transition: all var(--transition-fast);
}

.tab-btn.active {
  background: var(--bg-base);
  color: var(--kraken-purple);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
  font-weight: 600;
}

/* Form */
.form-section {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.status-row {
  display: flex;
  align-items: center;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  display: inline-block;
  flex-shrink: 0;
}

.dot-green { background: var(--green); }
.dot-red { background: var(--red); }

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.form-group {
  grid-column: span 2;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-group--half {
  grid-column: span 1;
}

.form-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.password-wrap {
  position: relative;
  display: flex;
  align-items: center;
}

.password-wrap .input {
  padding-right: 36px;
}

.error-box {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  background: var(--red-subtle);
  border: 1px solid rgba(224, 62, 62, 0.25);
  border-radius: var(--radius-sm);
  color: var(--red);
  font-size: 13px;
  padding: 10px 14px;
  line-height: 1.5;
}

.drawer-actions {
  margin-top: auto;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding-top: 12px;
  border-top: 1px solid var(--border-subtle);
}

/* Transitions */
.drawer-enter-active,
.drawer-leave-active {
  transition: opacity var(--transition-med);
}

.drawer-enter-active .drawer-panel,
.drawer-leave-active .drawer-panel {
  transition: transform var(--transition-med);
}

.drawer-enter-from,
.drawer-leave-to {
  opacity: 0;
}

.drawer-enter-from .drawer-panel,
.drawer-leave-to .drawer-panel {
  transform: translateX(100%);
}
</style>
