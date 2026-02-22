import fs from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'

const APP_VUE_PATH = path.resolve(process.cwd(), 'src', 'App.vue')
const COMPONENTS_DIR = path.resolve(process.cwd(), 'src', 'components')

const EXPECTED_WORKFLOW_PANELS = [
  { key: 'data', component: 'DataWorkspacePanel' },
  { key: 'optimize', component: 'OptimizeWorkspacePanel' },
  { key: 'strategy', component: 'StrategyWorkspacePanel' },
  { key: 'advanced', component: 'AdvancedWorkspacePanel' },
  { key: 'distributed', component: 'DistributedWorkspacePanel' },
  { key: 'overview', component: 'OverviewWorkspacePanel' },
  { key: 'top', component: 'TopWorkspacePanel' },
  { key: 'pareto', component: 'ParetoWorkspacePanel' },
  { key: 'validation', component: 'ValidationWorkspacePanel' },
  { key: 'drill', component: 'DrillWorkspacePanel' },
  { key: 'heatmap', component: 'HeatmapWorkspacePanel' },
  { key: 'history', component: 'HistoryWorkspacePanel' },
  { key: 'meta', component: 'MetaWorkspacePanel' },
  { key: 'debug', component: 'DebugWorkspacePanel' },
  { key: 'settings', component: 'SettingsWorkspacePanel' },
  { key: 'execution', component: 'ExecutionWorkspacePanel' },
] as const

describe('workflow coverage parity in Vue shell', () => {
  it('keeps all migrated workflow workspace panels present in the Vue host', () => {
    const appVue = fs.readFileSync(APP_VUE_PATH, 'utf8')

    for (const panel of EXPECTED_WORKFLOW_PANELS) {
      const componentFile = path.resolve(COMPONENTS_DIR, `${panel.component}.vue`)
      expect(fs.existsSync(componentFile), `${panel.component}.vue is missing`).toBe(true)
      expect(appVue).toContain(
        `const ${panel.component} = defineAsyncComponent(() => import('./components/${panel.component}.vue'))`,
      )
      expect(appVue).toContain(`mountWhenActive('${panel.key}'`)
      expect(appVue).toContain(`<${panel.component} v-if="mountedPanels.${panel.key}" />`)
    }
  })
})
