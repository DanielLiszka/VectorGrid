import type { FixtureFamilyModule } from './types'
import {
  optionsValidationInputFailureScenario,
  optionsValidationPreviewFailureScenario,
  optionsValidationReadyScenario,
} from './scenarios'

export {
  optionsValidationInputFailureScenario,
  optionsValidationPreviewFailureScenario,
  optionsValidationReadyScenario,
}

export const optionsFixtureFamily: FixtureFamilyModule = {
  family: 'options',
  description: 'Options-expression artifact, feasibility, and validation follow-up browser QA scenarios.',
  scenarios: [
    optionsValidationReadyScenario,
    optionsValidationInputFailureScenario,
    optionsValidationPreviewFailureScenario,
  ],
}
