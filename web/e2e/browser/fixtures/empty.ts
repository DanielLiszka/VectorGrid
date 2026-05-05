import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { emptyAppScenario } from './scenarios'

export { emptyAppScenario }

export const staleShellStateScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'stale-shell-state',
  family: 'empty',
  description:
    'Persisted workflow tab/step state contains stale tab vocabulary that must repair on launch.',
  localStorage: {
    ...emptyAppScenario.localStorage,
    vta_workspace_tab_v1: 'workflow',
    vta_workflow_tab_v1: 'settings',
    vta_workflow_step_v1: 'settings',
    vectorgrid_controls_tab_v1: 'advanced',
    vectorgrid_results_tab_v1: 'debug',
  },
}

export const helpTooltipReadyScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'help-tooltip-ready',
  family: 'empty',
  description:
    'Advanced shell starts with full contextual help enabled for tooltip role coverage.',
  localStorage: {
    ...emptyAppScenario.localStorage,
    vta_help_mode_v1: 'full',
  },
}

export const emptyFixtureFamily: FixtureFamilyModule = {
  family: 'empty',
  description: 'Fresh shell and blocked-state browser QA scenarios.',
  scenarios: [emptyAppScenario, staleShellStateScenario, helpTooltipReadyScenario],
}
