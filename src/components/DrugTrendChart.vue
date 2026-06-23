<template>
  <div class="chart-container">
    <!-- Header -->
    <div class="chart-header">
      <div class="chart-title-group">
        <span v-if="data" class="chart-drug-code font-mono">{{ getCode(data) }}</span>
        <span class="chart-title">
          {{ data ? getName(data) : 'เลือกรายการยาเพื่อดูแนวโน้ม' }}
        </span>
      </div>
      <div v-if="data" class="chart-total">
        {{ side === 'hosxp' ? 'รวมทั้งปี:' : 'มูลค่ารวม:' }}
        <span class="chart-total-value">
          {{ side === 'hosxp' ? formatQty(getTotal(data)) : formatBaht(getTotal(data)) }}
        </span>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="chart-loading">
      <div class="skeleton" style="width: 100%; height: 100%; border-radius: 8px;" />
    </div>

    <!-- Empty -->
    <div v-else-if="!data" class="chart-empty">
      <BarChart2 :size="40" class="chart-empty-icon" />
      <p>คลิกชื่อยาในรายการทางซ้าย<br />หรือค้นหายาเพื่อดูกราฟแนวโน้ม</p>
    </div>

    <!-- ECharts -->
    <v-chart
      v-else
      class="echarts-canvas"
      :option="chartOption"
      :autoresize="true"
      :update-options="{ notMerge: true }"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { BarChart, LineChart } from 'echarts/charts'
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
} from 'echarts/components'
import VChart from 'vue-echarts'
import { BarChart2 } from 'lucide-vue-next'
import { formatBaht, formatQty, FISCAL_MONTHS_SHORT, THAI_MONTHS_SHORT } from '../utils/dateUtils'

use([CanvasRenderer, BarChart, LineChart, GridComponent, TooltipComponent, LegendComponent])

type ChartData = {
  icode?: string; working_code?: string
  drug_name?: string; name?: string
  monthly_qty?: number[]; monthly_value?: number[]
  total_qty?: number; total_value?: number
}

const props = defineProps<{
  side: 'hosxp' | 'invs'
  data: ChartData | null
  loading: boolean
}>()

function getCode(d: ChartData): string { return d.icode ?? d.working_code ?? '' }
function getName(d: ChartData): string { return d.drug_name ?? d.name ?? '—' }
function getTotal(d: ChartData): number { return d.total_qty ?? d.total_value ?? 0 }

const barColor = computed(() => props.side === 'hosxp' ? '#7132f5' : '#149e61')
const barColorLight = computed(() => props.side === 'hosxp' ? 'rgba(113,50,245,0.3)' : 'rgba(20,158,97,0.3)')
const lineColor = computed(() => props.side === 'hosxp' ? '#5741d8' : '#026b3f')

const chartOption = computed(() => {
  if (!props.data) return {}
  const vals = props.side === 'hosxp'
    ? (props.data.monthly_qty ?? Array(12).fill(0))
    : (props.data.monthly_value ?? Array(12).fill(0))
  const total = getTotal(props.data) || 1
  const months = props.side === 'hosxp' ? THAI_MONTHS_SHORT : FISCAL_MONTHS_SHORT

  // 3-month moving average
  const movingAvg = vals.map((_, i) => {
    const start = Math.max(0, i - 2)
    const slice = vals.slice(start, i + 1)
    return slice.reduce((a, b) => a + b, 0) / slice.length
  })

  return {
    backgroundColor: 'transparent',
    grid: { left: 16, right: 16, top: 24, bottom: 36, containLabel: true },
    tooltip: {
      trigger: 'axis',
      backgroundColor: props.side === 'hosxp' ? '#1a1040' : '#1a2e1a',
      borderColor: barColor.value,
      borderWidth: 1,
      textStyle: { color: '#ffffff', fontFamily: 'IBM Plex Sans, sans-serif', fontSize: 12 },
      formatter(params: { name: string; seriesName: string; value: number }[]) {
        const bar = params.find(p => p.seriesName === (props.side === 'hosxp' ? 'จำนวนจ่าย' : 'มูลค่ารายเดือน'))
        const val = bar?.value ?? 0
        const pct = total > 0 ? ((val / total) * 100).toFixed(1) : '0.0'
        const formatted = props.side === 'hosxp' ? formatQty(val) : formatBaht(val)
        return `<b>${params[0]?.name}</b><br/>${props.side === 'hosxp' ? 'จำนวน' : 'มูลค่า'}: <b style="color:${barColor.value}">${formatted}</b> (${pct}%)`
      },
    },
    legend: {
      bottom: 0, right: 'center',
      textStyle: { color: '#686b82', fontFamily: 'IBM Plex Sans, sans-serif', fontSize: 11 },
    },
    xAxis: {
      type: 'category',
      data: months,
      axisLine: { lineStyle: { color: 'rgba(104,107,130,0.15)' } },
      axisLabel: { color: '#686b82', fontFamily: 'IBM Plex Sans, sans-serif', fontSize: 11 },
      axisTick: { show: false },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: 'rgba(104,107,130,0.08)', type: 'dashed' } },
      axisLabel: {
        color: '#9497a9',
        fontFamily: 'IBM Plex Sans, sans-serif',
        fontSize: 11,
        formatter: (v: number) => {
          if (props.side === 'hosxp') {
            return v >= 1000 ? (v / 1000).toFixed(1) + 'K' : v.toString()
          }
          return v >= 1_000_000 ? '฿' + (v / 1_000_000).toFixed(1) + 'M'
            : v >= 1_000 ? '฿' + (v / 1_000).toFixed(0) + 'K'
            : '฿' + v
        },
      },
    },
    series: [
      {
        name: props.side === 'hosxp' ? 'จำนวนจ่าย' : 'มูลค่ารายเดือน',
        type: 'bar',
        data: vals.map(v => ({
          value: v,
          itemStyle: {
            color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: barColor.value },
                { offset: 1, color: barColorLight.value },
              ],
            },
            borderRadius: [4, 4, 0, 0],
          },
        })),
        barMaxWidth: 36,
        animationEasing: 'cubicOut',
        animationDuration: 700,
      },
      {
        name: 'แนวโน้ม',
        type: 'line',
        data: movingAvg,
        smooth: true,
        symbol: 'circle',
        symbolSize: 5,
        lineStyle: { color: lineColor.value, width: 2 },
        itemStyle: { color: lineColor.value },
        animationEasing: 'cubicOut',
        animationDuration: 900,
      },
    ],
  }
})
</script>

<style scoped>
.chart-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 12px 16px 8px;
  gap: 8px;
}

.chart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  gap: 12px;
}

.chart-title-group {
  display: flex;
  align-items: baseline;
  gap: 8px;
  overflow: hidden;
  min-width: 0;
}

.chart-drug-code {
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  padding: 2px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}

.chart-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chart-total {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
  flex-shrink: 0;
}

.chart-total-value {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
  margin-left: 4px;
}

.echarts-canvas {
  flex: 1;
  min-height: 0;
}

.chart-loading {
  flex: 1;
  padding: 12px;
}

.chart-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-muted);
  text-align: center;
  font-size: 13px;
  line-height: 1.6;
}

.chart-empty-icon { opacity: 0.3; }
</style>
