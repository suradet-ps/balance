<template>
  <div class="app-shell">
    <!-- Header -->
    <AppHeader @open-settings="showSettings = true" />

    <!-- Error banner -->
    <Transition name="slide-down">
      <div v-if="dashStore.error" class="error-banner">
        <AlertTriangle :size="14" />
        {{ dashStore.error }}
        <button class="btn-dismiss" @click="dashStore.error = null">
          <X :size="12" />
        </button>
      </div>
    </Transition>

    <!-- No connection prompt -->
    <div v-if="!dbStore.anyConnected && !dbStore.hosxpConnecting && !dbStore.invsConnecting" class="no-conn-banner">
      <PlugZap :size="14" />
      ยังไม่ได้เชื่อมต่อฐานข้อมูล —
      <button class="link-btn" @click="showSettings = true">คลิกเพื่อตั้งค่าการเชื่อมต่อ</button>
    </div>

    <!-- Main Layout: Two-panel split -->
    <main class="main-grid">
      <!-- Left: HOSxP Panel (Qty) -->
      <section class="panel panel-hosxp">
        <div class="panel-label">
          <span class="panel-dot dot-purple" />
          HOSxP — ปริมาณการจ่ายยา
        </div>
        <DrugSearchPanel
          side="hosxp"
          placeholder="ค้นหายา HOSxP (รหัส / ชื่อ)..."
          :search-fn="hosxpSearch"
          @select="onHosxpSelect"
        />
        <div class="chart-card card">
          <DrugTrendChart
            side="hosxp"
            :data="dashStore.hosxpChartData"
            :loading="dashStore.loadingChart"
          />
        </div>
      </section>

      <!-- Divider -->
      <div class="panel-divider" />

      <!-- Right: INVS Panel (Value) -->
      <section class="panel panel-invs">
        <div class="panel-label">
          <span class="panel-dot dot-green" />
          INVS — มูลค่าการสั่งซื้อ
        </div>
        <DrugSearchPanel
          side="invs"
          placeholder="ค้นหายา INVS (รหัส / ชื่อ)..."
          :search-fn="invsSearch"
          @select="onInvsSelect"
        />
        <div class="chart-card card">
          <DrugTrendChart
            side="invs"
            :data="dashStore.invsChartData"
            :loading="dashStore.loadingChart"
          />
        </div>
      </section>
    </main>

    <!-- KPI Bar -->
    <SummaryKpiBar />

    <!-- Connection Settings Drawer -->
    <ConnectionSettings :visible="showSettings" @close="showSettings = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { AlertTriangle, PlugZap, X } from 'lucide-vue-next'
import { useDbConfigStore } from './stores/dbConfig'
import { useDashboardStore } from './stores/dashboard'
import { useHosxpData } from './composables/useHosxpData'
import { useInvsData } from './composables/useInvsData'

import AppHeader from './components/AppHeader.vue'
import ConnectionSettings from './components/ConnectionSettings.vue'
import DrugSearchPanel from './components/DrugSearchPanel.vue'
import DrugTrendChart from './components/DrugTrendChart.vue'
import SummaryKpiBar from './components/SummaryKpiBar.vue'

const dbStore = useDbConfigStore()
const dashStore = useDashboardStore()
const hosxp = useHosxpData()
const invs = useInvsData()

const showSettings = ref(false)

// Search wrappers
async function hosxpSearch(q: string) { return hosxp.searchDrugs(q) }
async function invsSearch(q: string) { return invs.searchDrugs(q) }

// Drug selection handlers
async function onHosxpSelect(code: string) {
  dashStore.selectHosxpDrug(code)
  await hosxp.fetchDrugMonthly(dashStore.selectedYear, code)
}

async function onInvsSelect(code: string) {
  dashStore.selectInvsDrug(code)
  await invs.fetchDrugMonthlyValue(dashStore.selectedYear, code)
}

// Refresh all data for current year
async function refreshAll() {
  dashStore.loading = true
  dashStore.error = null
  try {
    const promises: Promise<unknown>[] = []
    if (dbStore.hosxpConnected) promises.push(hosxp.refreshAll(dashStore.selectedYear))
    if (dbStore.invsConnected) promises.push(invs.refreshAll(dashStore.selectedYear))
    await Promise.all(promises)
  } catch (e) {
    dashStore.error = String(e)
  } finally {
    dashStore.loading = false
  }
}

// Watch connection state → fetch years + data
watch(() => dbStore.hosxpConnected, async (connected) => {
  if (connected) {
    const years = await hosxp.fetchAvailableYears()
    if (years.length && !years.includes(dashStore.selectedYear)) {
      dashStore.selectedYear = years[0]
    }
  }
})

watch(() => dbStore.invsConnected, async (connected) => {
  if (connected) {
    const years = await invs.fetchAvailableYears()
    if (years.length && !years.includes(dashStore.selectedYear)) {
      dashStore.selectedYear = years[0]
    }
    await invs.fetchYearSummary(dashStore.selectedYear)
  }
})

// Watch year change → reload all
watch(() => dashStore.selectedYear, () => refreshAll())

onMounted(async () => {
  await dbStore.initFromStorage()
  // Small delay to let connections attempt
  setTimeout(() => refreshAll(), 500)
})
</script>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  background: var(--bg-base);
}

/* ─── Main two-panel grid ──────────────────────────────── */

.main-grid {
  display: grid;
  grid-template-columns: 1fr 1px 1fr;
  gap: 0;
  flex: 1;
  overflow: hidden;
  min-height: 0;
  padding: 12px 16px 0;
}

.panel {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
  gap: 8px;
}

.panel-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 4px 0;
  flex-shrink: 0;
}

.panel-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-purple { background: var(--kraken-purple); }
.dot-green { background: var(--green); }

.panel-divider {
  background: var(--border-subtle);
  margin: 0 8px;
}

.chart-card {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

/* ─── Banners ──────────────────────────────────────────── */

.error-banner {
  background: var(--red-subtle);
  border-bottom: 1px solid rgba(224, 62, 62, 0.2);
  color: var(--red);
  font-size: 13px;
  padding: 8px 20px;
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: space-between;
  flex-shrink: 0;
}

.btn-dismiss {
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--red);
  display: flex;
  align-items: center;
  padding: 2px;
}

.no-conn-banner {
  background: var(--kraken-purple-subtle);
  border-bottom: 1px solid rgba(113, 50, 245, 0.2);
  color: var(--kraken-purple-dark);
  font-size: 13px;
  padding: 7px 20px;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.link-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--kraken-purple);
  font-family: var(--font-body);
  font-size: 13px;
  font-weight: 600;
  text-decoration: underline;
}

/* ─── Banner transitions ─────────────────────────────── */

.slide-down-enter-active,
.slide-down-leave-active {
  transition: all var(--transition-med);
  overflow: hidden;
}

.slide-down-enter-from,
.slide-down-leave-to {
  max-height: 0;
  opacity: 0;
  padding-top: 0;
  padding-bottom: 0;
}

.slide-down-enter-to,
.slide-down-leave-from {
  max-height: 48px;
}
</style>
