import { onMounted, onUnmounted, type Ref } from 'vue'

export function onClickOutside(target: Ref<HTMLElement | null>, handler: () => void) {
  const listener = (e: MouseEvent) => {
    if (!target.value || target.value.contains(e.target as Node)) return
    handler()
  }
  onMounted(() => document.addEventListener('mousedown', listener))
  onUnmounted(() => document.removeEventListener('mousedown', listener))
}
