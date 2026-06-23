import { invoke } from '@tauri-apps/api/core'
import { useDashboardStore } from '../stores/dashboard'
import type {
  InvsDrugValueSummary,
  InvsDrugMonthlyValue,
  InvsYearSummary,
  InvsDrugItem,
} from '../stores/dashboard'

export function useInvsData() {
  const store = useDashboardStore()

  async function fetchAvailableYears(): Promise<number[]> {
    try {
      const years = await invoke<number[]>('invs_get_available_years')
      store.invsYears = years
      return years
    } catch (err) {
      console.error('INVS fetchAvailableYears:', err)
      return []
    }
  }

  async function fetchTopDrugs(year: number, limit = 10): Promise<InvsDrugValueSummary[]> {
    try {
      const drugs = await invoke<InvsDrugValueSummary[]>('invs_get_top_drugs_by_value', { year, limit })
      store.invsTopDrugs = drugs
      return drugs
    } catch (err) {
      console.error('INVS fetchTopDrugs:', err)
      return []
    }
  }

  async function fetchDrugMonthlyValue(year: number, workingCode: string): Promise<InvsDrugMonthlyValue | null> {
    store.loadingChart = true
    try {
      const data = await invoke<InvsDrugMonthlyValue>('invs_get_drug_monthly_value', { year, workingCode })
      store.invsChartData = data
      return data
    } catch (err) {
      console.error('INVS fetchDrugMonthlyValue:', err)
      return null
    } finally {
      store.loadingChart = false
    }
  }

  async function fetchYearSummary(year: number): Promise<InvsYearSummary | null> {
    try {
      const summary = await invoke<InvsYearSummary>('invs_get_year_summary', { year })
      store.invsYearSummary = summary
      return summary
    } catch (err) {
      console.error('INVS fetchYearSummary:', err)
      return null
    }
  }

  async function searchDrugs(query: string): Promise<InvsDrugItem[]> {
    if (!query.trim()) return []
    try {
      return await invoke<InvsDrugItem[]>('invs_get_drug_list', { search: query })
    } catch (err) {
      console.error('INVS searchDrugs:', err)
      return []
    }
  }

  async function refreshAll(year: number) {
    await Promise.all([fetchTopDrugs(year), fetchYearSummary(year)])
    if (store.invsSelectedCode) {
      await fetchDrugMonthlyValue(year, store.invsSelectedCode)
    }
  }

  return { fetchAvailableYears, fetchTopDrugs, fetchDrugMonthlyValue, fetchYearSummary, searchDrugs, refreshAll }
}
