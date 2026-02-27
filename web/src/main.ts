import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/app.css'
import { runStorageMigrations } from './utils/storage'

runStorageMigrations()

const root = document.querySelector('#app') as HTMLElement | null
if (root) {
  const app = createApp(App)
  app.use(createPinia())
  app.mount(root)
}

