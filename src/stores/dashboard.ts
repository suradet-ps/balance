import { defineStore } from 'pinia'
import { ref } from 'vue'
import { currentFiscalYear } from '../utils/dateUtils'

// ─── HOSxP Types ───────────────────────────────────────────────────────

export interface HosxpDrugSummary {
  icode: string
  drug_name: string
  total_qty: number
  peak_month: number
}

export interface HosxpDrugMonthlyData {
  icode: string
  drug_name: string
  monthly_qty: number[]
  total_qty: number
}

export interface HosxpDrugItem {
  icode: string
  name: string
}

// ─── INVS Types ────────────────────────────────────────────────────────

export interface InvsDrugValueSummary {
  working_code: string
  drug_name: string
  total_value: number
  peak_month: number
  peak_month_value: number
}

export interface InvsDrugMonthlyValue {
  working_code: string
  drug_name: string
  monthly_value: number[]
  total_value: number
  peak_month: number
}

export interface InvsYearSummary {
  total_value: number
  unique_drug_count: number
  peak_month: number
  peak_month_value: number
}

export interface InvsDrugItem {
  working_code: string
  drug_name: string
}

// ─── Store ─────────────────────────────────────────────────────────────

export const useDashboardStore = defineStore('dashboard', () => {
  const selectedYear = ref<number>(currentFiscalYear())

  // HOSxP
  const hosxpYears = ref<number[]>([])
  const hosxpSelectedIcode = ref<string | null>(null)
  const hosxpTopDrugs = ref<HosxpDrugSummary[]>([])
  const hosxpChartData = ref<HosxpDrugMonthlyData | null>(null)

  // INVS
  const invsYears = ref<number[]>([])
  const invsSelectedCode = ref<string | null>(null)
  const invsTopDrugs = ref<InvsDrugValueSummary[]>([])
  const invsChartData = ref<InvsDrugMonthlyValue | null>(null)
  const invsYearSummary = ref<InvsYearSummary | null>(null)

  // Shared
  const loading = ref(false)
  const loadingChart = ref(false)
  const error = ref<string | null>(null)

  function setYear(year: number) {
    selectedYear.value = year
  }

  function selectHosxpDrug(icode: string) {
    hosxpSelectedIcode.value = icode
  }

  function selectInvsDrug(code: string) {
    invsSelectedCode.value = code
  }

  return {
    selectedYear,
    // HOSxP
    hosxpYears, hosxpSelectedIcode, hosxpTopDrugs, hosxpChartData,
    // INVS
    invsYears, invsSelectedCode, invsTopDrugs, invsChartData, invsYearSummary,
    // Shared
    loading, loadingChart, error,
    setYear, selectHosxpDrug, selectInvsDrug,
  }
})
