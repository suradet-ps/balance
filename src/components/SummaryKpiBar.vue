<template>
  <div class="kpi-bar">
    <!-- HOSxP: Total Qty -->
    <div class="kpi-card animate-fade-up" style="animation-delay: 0ms">
      <div class="kpi-icon kpi-icon--hosxp">
        <Pill :size="22" />
      </div>
      <div class="kpi-body">
        <div class="kpi-label">HOSxP สถานะ</div>
        <div v-if="dbStore.hosxpConnected" class="kpi-value value-hosxp">
          เชื่อมต่อแล้ว
        </div>
        <div v-else class="kpi-value kpi-na">—</div>
      </div>
    </div>

    <!-- Divider -->
    <div class="kpi-divider" />

    <!-- INVS: Total Value -->
    <div class="kpi-card animate-fade-up" style="animation-delay: 120ms">
      <div class="kpi-icon kpi-icon--invs">
        <Banknote :size="22" />
      </div>
      <div class="kpi-body">
        <div class="kpi-label">INVS สั่งซื้อรวม</div>
        <div v-if="dbStore.invsConnected && dashStore.invsYearSummary" class="kpi-value value-invs">
          {{ formatBaht(dashStore.invsYearSummary.total_value) }}
        </div>
        <div v-else class="kpi-value kpi-na">—</div>
      </div>
    </div>

    <!-- INVS: Drug Count -->
    <div class="kpi-card animate-fade-up" style="animation-delay: 180ms">
      <div class="kpi-icon kpi-icon--invs">
        <Package :size="22" />
      </div>
      <div class="kpi-body">
        <div class="kpi-label">INVS รายการยา</div>
        <div v-if="dbStore.invsConnected && dashStore.invsYearSummary" class="kpi-value value-invs">
          {{ dashStore.invsYearSummary.unique_drug_count.toLocaleString('th-TH') }}
          <span class="kpi-unit">รายการ</span>
        </div>
        <div v-else class="kpi-value kpi-na">—</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Pill, Banknote, Package } from 'lucide-vue-next'
import { useDbConfigStore } from '../stores/dbConfig'
import { useDashboardStore } from '../stores/dashboard'
import { formatBaht } from '../utils/dateUtils'

const dbStore = useDbConfigStore()
const dashStore = useDashboardStore()
</script>

<style scoped>
.kpi-bar {
  display: flex;
  gap: 10px;
  padding: 10px 16px 12px;
  flex-shrink: 0;
  border-top: 1px solid var(--border-subtle);
  background: var(--bg-surface);
}

.kpi-card {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--bg-base);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-subtle);
  padding: 10px 14px;
  transition: box-shadow var(--transition-med);
}

.kpi-card:hover {
  box-shadow: var(--shadow-hover);
}

.kpi-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-md);
  flex-shrink: 0;
}

.kpi-icon--hosxp {
  background: var(--kraken-purple-subtle);
  color: var(--kraken-purple);
}

.kpi-icon--invs {
  background: var(--green-subtle);
  color: var(--green);
}

.kpi-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.kpi-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.kpi-value {
  font-size: 16px;
  font-weight: 700;
  display: flex;
  align-items: baseline;
  gap: 4px;
  white-space: nowrap;
}

.value-hosxp { color: var(--kraken-purple); }
.value-invs { color: var(--green); }
.kpi-na { color: var(--text-muted); }

.kpi-unit {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
}

.kpi-divider {
  width: 1px;
  align-self: stretch;
  margin: 8px 0;
  background: var(--border-subtle);
  flex-shrink: 0;
}
</style>
