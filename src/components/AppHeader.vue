<template>
  <header class="app-header">
    <div class="header-brand">
      <img class="brand-icon" :src="logoUrl" alt="Balance Logo" />
      <div class="brand-text">
        <span class="brand-title">Balance</span>
        <span class="brand-sub">โรงพยาบาลสระโบสถ์</span>
      </div>
    </div>

    <div class="header-controls">
      <!-- Year selector -->
      <div class="year-selector">
        <label>ปีงบประมาณ</label>
        <select :value="dashStore.selectedYear" @change="onYearChange">
          <option v-for="y in mergedYears" :key="y" :value="y">{{ y }}</option>
          <option v-if="mergedYears.length === 0" :value="dashStore.selectedYear">
            {{ dashStore.selectedYear }}
          </option>
        </select>
      </div>

      <!-- HOSxP badge -->
      <span :class="['badge', dbStore.hosxpConnected ? 'badge-connected' : 'badge-disconnected']">
        <span :class="['status-dot', dbStore.hosxpConnected ? 'dot-green' : 'dot-red']" />
        MySQL
      </span>

      <!-- INVS badge -->
      <span :class="['badge', dbStore.invsConnected ? 'badge-connected' : 'badge-disconnected']">
        <span :class="['status-dot', dbStore.invsConnected ? 'dot-green' : 'dot-red']" />
        MSSQL
      </span>

      <!-- Settings button -->
      <button class="btn btn-ghost settings-btn" @click="$emit('openSettings')">
        <Settings :size="14" />
        ตั้งค่า
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Settings } from 'lucide-vue-next'
import { useDbConfigStore } from '../stores/dbConfig'
import { useDashboardStore } from '../stores/dashboard'
import logoUrl from '../assets/logo.svg'

defineEmits<{ openSettings: [] }>()

const dbStore = useDbConfigStore()
const dashStore = useDashboardStore()

const mergedYears = computed(() => {
  const all = new Set([...dbStore.hosxpConnected ? [] : [], ...dashStore.hosxpYears, ...dashStore.invsYears])
  const arr = Array.from(all).sort((a, b) => b - a)
  return arr.length > 0 ? arr : [dashStore.selectedYear]
})

function onYearChange(e: Event) {
  const year = parseInt((e.target as HTMLSelectElement).value, 10)
  dashStore.setYear(year)
}
</script>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  height: 52px;
  background: var(--near-black);
  color: var(--text-on-primary);
  flex-shrink: 0;
  gap: 16px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  z-index: 20;
}

.header-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.brand-icon {
  width: 32px;
  height: 32px;
  object-fit: contain;
  flex-shrink: 0;
}

.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.brand-title {
  font-family: var(--font-display);
  font-size: 16px;
  font-weight: 700;
  color: #ffffff;
  letter-spacing: -0.5px;
}

.brand-sub {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.55);
}

.header-controls {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Year selector */
.year-selector {
  display: flex;
  align-items: center;
  gap: 8px;
}

.year-selector label {
  font-size: 12px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.65);
  white-space: nowrap;
}

.year-selector select {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: var(--radius-lg);
  padding: 6px 28px 6px 10px;
  color: #ffffff;
  font-family: var(--font-body);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath d='M1 1l5 5 5-5' stroke='white' stroke-width='1.5' fill='none' stroke-linecap='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  transition: border-color var(--transition-fast);
}

.year-selector select:focus {
  outline: none;
  border-color: var(--kraken-purple);
}

/* Badges */
.badge {
  font-size: 11px;
  padding: 4px 10px;
}

.badge-connected {
  background: rgba(20, 158, 97, 0.15);
  color: #4ade80;
}

.badge-disconnected {
  background: rgba(224, 62, 62, 0.12);
  color: #f87171;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  display: inline-block;
}

.dot-green { background: #4ade80; }
.dot-red { background: #f87171; }

/* Settings button */
.settings-btn {
  color: rgba(255, 255, 255, 0.75);
  border-color: rgba(255, 255, 255, 0.2);
  font-size: 12px;
  padding: 5px 10px;
}

.settings-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.35);
  color: #ffffff;
}
</style>
