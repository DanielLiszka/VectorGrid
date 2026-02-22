export function createCoalescedSync(syncNow: () => void): () => void {
  let pending = false

  return () => {
    if (pending) return
    pending = true

    const flush = () => {
      pending = false
      syncNow()
    }

    if (typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
      window.requestAnimationFrame(() => flush())
      return
    }

    Promise.resolve().then(flush)
  }
}
