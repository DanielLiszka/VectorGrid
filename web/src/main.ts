import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/app.css'
import { runStorageMigrations } from './utils/storage'

runStorageMigrations()
window.__VTA_DISABLE_LEGACY_SHORTCUTS__ = true
window.__VTA_VUE_WORKFLOW_OWNER__ = true

const root = document.getElementById('app')
if (root) {
  const app = createApp(App)
  app.use(createPinia())
  app.mount(root)
}
