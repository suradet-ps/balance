import { invoke } from '@tauri-apps/api/core'
import { useDashboardStore } from '../stores/dashboard'
import type {
  HosxpDrugSummary,
  HosxpDrugMonthlyData,
  HosxpDrugItem,
} from '../stores/dashboard'

export function useHosxpData() {
  const store = useDashboardStore()

  async function fetchAvailableYears(): Promise<number[]> {
    try {
      const years = await invoke<number[]>('hosxp_get_available_years')
      store.hosxpYears = years
      return years
    } catch (err) {
      // Don't set global error — HOSxP may not be connected
      console.error('HOSxP fetchAvailableYears:', err)
      return []
    }
  }

  async function fetchTopDrugs(year: number, limit = 10): Promise<HosxpDrugSummary[]> {
    try {
      const drugs = await invoke<HosxpDrugSummary[]>('hosxp_get_top_drugs', { year, limit })
      store.hosxpTopDrugs = drugs
      return drugs
    } catch (err) {
      console.error('HOSxP fetchTopDrugs:', err)
      return []
    }
  }

  async function fetchDrugMonthly(year: number, icode: string): Promise<HosxpDrugMonthlyData | null> {
    store.loadingChart = true
    try {
      const result = await invoke<HosxpDrugMonthlyData[]>('hosxp_get_drug_monthly_qty', { year, icode })
      const data = result?.[0] ?? null
      store.hosxpChartData = data
      return data
    } catch (err) {
      console.error('HOSxP fetchDrugMonthly:', err)
      return null
    } finally {
      store.loadingChart = false
    }
  }

  async function searchDrugs(query: string): Promise<HosxpDrugItem[]> {
    if (!query.trim()) return []
    try {
      return await invoke<HosxpDrugItem[]>('hosxp_get_drug_list', { search: query })
    } catch (err) {
      console.error('HOSxP searchDrugs:', err)
      return []
    }
  }

  async function refreshAll(year: number) {
    await fetchTopDrugs(year)
    if (store.hosxpSelectedIcode) {
      await fetchDrugMonthly(year, store.hosxpSelectedIcode)
    }
  }

  return { fetchAvailableYears, fetchTopDrugs, fetchDrugMonthly, searchDrugs, refreshAll }
}
