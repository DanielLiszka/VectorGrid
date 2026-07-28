import { $, browser, expect } from '@wdio/globals'
import fs from 'node:fs'
import path from 'node:path'
import {
  RELEASE_PROOF_OPTIONS_DATASET_ID,
  sha256File,
  writeOptionsDatasetFixture,
} from '../support/nativeFeatureFixtures.js'
import {
  isPathInside,
  requireReleaseProofEnvironment,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { invokeNative } from '../support/nativeIpc.js'
import { nativeWorkflowSelectors } from '../support/selectors.js'
import { openWorkspace, waitForShellReady } from '../support/waits.js'

type RunArtifactSummary = {
  path?: unknown
  has_validation?: unknown
  strategy_kind?: unknown
}

type OptionsDatasetManifest = {
  options_dataset_id?: unknown
  dataset_path?: unknown
  row_count?: unknown
  normalized_rows_sha256?: unknown
}

type ReplayBuildTiming = {
  parent_signal_process_cache_hit?: unknown
  persisted_signal_manifest_cache_hit?: unknown
  options_dataset_process_cache_hit?: unknown
  options_dataset_persisted_cache_hit?: unknown
}

type OptionsFeasibilityResponse = {
  replay_build_timing?: ReplayBuildTiming
  report?: {
    compiled_tuple_count?: unknown
    compiled_candidate_count?: unknown
    candidate_density?: unknown
  }
}

type OptionsRunResponse = {
  artifact_path?: unknown
  result?: {
    backend_used?: unknown
    tuple_count?: unknown
    ranked_results?: unknown[]
  }
}

type OptionsValidationResponse = {
  artifact_path?: unknown
  result?: {
    tuple_results?: unknown[]
    acceptance_summary?: {
      verdict?: unknown
      validated_tuple_count?: unknown
    }
  }
}

function text(value: unknown): string {
  return String(value ?? '').trim()
}

async function setInput(selector: string, value: string): Promise<void> {
  const field = await $(selector)
  await field.waitForDisplayed({ timeout: 10_000 })
  await browser.execute((targetSelector: string, nextValue: string) => {
    const page = globalThis as unknown as {
      document: {
        querySelector: (value: string) => {
          value: string
          dispatchEvent: (event: unknown) => boolean
        } | null
      }
      Event: new (type: string, init: { bubbles: boolean }) => unknown
    }
    const element = page.document.querySelector(targetSelector)
    if (!element) throw new Error(`Expected an input for ${targetSelector}.`)
    element.value = nextValue
    element.dispatchEvent(new page.Event('input', { bubbles: true }))
    element.dispatchEvent(new page.Event('change', { bubbles: true }))
  }, selector, value)
  await browser.waitUntil(
    async () => text(await field.getValue()) === value,
    {
      timeout: 3_000,
      interval: 50,
      timeoutMsg: `${selector} did not retain ${JSON.stringify(value)}.`,
    },
  )
}

function boundedOptionsRequest(
  parentArtifactPath: string,
): Record<string, unknown> {
  return {
    parent_artifact_path: parentArtifactPath,
    options_dataset_id: RELEASE_PROOF_OPTIONS_DATASET_ID,
    structure_family: 'long_single',
    backend_preference: 'cpu_reference',
    objective_config: {
      profile: 'balanced',
    },
    delay_minutes: [0],
    dte_buckets: [1],
    moneyness_buckets: ['atm'],
    hold_minutes: [1],
    target_return_buckets: [0.25],
    stop_return_buckets: [0.25],
    exit_template: 'fixed_hold_or_target_stop',
    fill_model: 'aggregate_research',
    filters: {
      max_entry_lag_minutes: 1,
      min_volume: 1,
      min_open_interest: 1,
    },
    top_k: 3,
    min_valid_trades: 1,
    split_ratio_search: 0.6,
    split_ratio_holdout: 0.2,
    split_assignment_mode: 'event_chronological',
    sealed_finalist_count: 1,
  }
}

function validationRequest(artifactPath: string): Record<string, unknown> {
  return {
    artifact_path: artifactPath,
    options_dataset_id: '',
    dataset_policy: 'require_exact_replay',
    tuple_selection: {
      include_sealed_finalists: false,
      include_robust_leader: false,
      include_holdout_leader: false,
      tuple_indexes: [0],
    },
    validation_plan: {
      mode: 'original_embedded_splits',
    },
    min_valid_trades: 1,
  }
}

async function openParentOptionsArtifact(
  parentArtifactPath: string,
  optionsArtifactPath: string,
): Promise<void> {
  await openWorkspace('library')
  const refresh = await $(nativeWorkflowSelectors.savedRunsRefresh)
  await refresh.waitForEnabled({ timeout: 10_000 })
  await refresh.click()
  await setInput(
    nativeWorkflowSelectors.savedRunsSearch,
    path.basename(parentArtifactPath),
  )
  const parent = await $(nativeWorkflowSelectors.savedRunWithValidation)
  await parent.waitForDisplayed({ timeout: 15_000 })
  await parent.click()

  const panel = await $('[aria-label="Options expression follow-up"]')
  await panel.waitForDisplayed({ timeout: 15_000 })
  const refreshOptions = await $('[data-qa-action="options.artifacts.refresh"]')
  await refreshOptions.waitForEnabled({ timeout: 10_000 })
  await refreshOptions.click()
  const select = await $('#optionsArtifactSelect')
  await select.waitForDisplayed({ timeout: 10_000 })
  await select.selectByAttribute('value', optionsArtifactPath)
  const load = await $('[data-qa-action="options.artifacts.load"]')
  await load.waitForEnabled({ timeout: 10_000 })
  await load.click()
  await browser.waitUntil(
    async () => (await $('.optionsPanel [role="status"]').getText())
      .toLowerCase()
      .includes('loaded saved options expression artifact'),
    {
      timeout: 15_000,
      interval: 100,
      timeoutMsg: 'The cold Options surface did not load its saved artifact.',
    },
  )
  await expect(
    await $('[aria-label="Ranked options expression tuples"]'),
  ).toBeDisplayed()
}

describe('desktop exact-package Options lifecycle proof', () => {
  it('runs, validates, cold-reloads, and reconstructs Options replay caches through native IPC', async () => {
    const environment = requireReleaseProofEnvironment()
    await waitForShellReady()
    const parent = (await invokeNative<RunArtifactSummary[]>('list_run_artifacts'))
      .find((row) => (
        text(row.strategy_kind) === 'DoubleMa'
        && Boolean(row.has_validation)
        && text(row.path)
      ))
    if (!parent) {
      throw new Error(
        'Options release proof requires the validated DoubleMa artifact from the native workflow spec.',
      )
    }
    const parentArtifactPath = text(parent.path)
    const optionsCsvPath = writeOptionsDatasetFixture()
    const imported = await invokeNative<OptionsDatasetManifest>(
      'import_options_dataset',
      {
        req: {
          csv_path: optionsCsvPath,
          options_dataset_id: RELEASE_PROOF_OPTIONS_DATASET_ID,
          root_symbol: 'VG',
          provider_name: 'polygon.io',
          provider_source_surface: 'minute_aggregate',
          quote_resolution: '1m',
          price_semantics: 'minute_aggregate',
          calendar_id: 'market_hours_utc',
          timezone: 'UTC',
          normalization_version: 1,
          quote_semantics_version: 1,
          corporate_action_mapping_version: 1,
          contract_reference_source: 'polygon_contract_reference',
          underlier_reference_source: 'underlying_dataset',
          aggregate_price_field: 'close',
        },
      },
    )
    expect(text(imported.options_dataset_id)).toBe(
      RELEASE_PROOF_OPTIONS_DATASET_ID,
    )
    const storedDatasetPath = text(imported.dataset_path)
    expect(fs.existsSync(storedDatasetPath)).toBe(true)
    expect(isPathInside(storedDatasetPath, environment.isolationRoot)).toBe(true)
    expect(Number(imported.row_count)).toBeGreaterThan(1_000)

    const request = boundedOptionsRequest(parentArtifactPath)
    const feasibility = await invokeNative<OptionsFeasibilityResponse>(
      'analyze_options_expression_feasibility',
      { req: request },
    )
    expect(Number(feasibility.report?.compiled_tuple_count)).toBeGreaterThan(0)
    expect(Number(feasibility.report?.compiled_candidate_count)).toBeGreaterThan(0)
    expect(Number(feasibility.report?.candidate_density)).toBeGreaterThan(0)

    const run = await invokeNative<OptionsRunResponse>(
      'run_options_expression',
      { req: request },
    )
    const artifactPath = text(run.artifact_path)
    if (!artifactPath || !fs.existsSync(artifactPath)) {
      throw new Error(`Options run returned a missing artifact: ${artifactPath}`)
    }
    expect(isPathInside(artifactPath, environment.isolationRoot)).toBe(true)
    expect(text(run.result?.backend_used)).toBe('cpu_reference')
    expect(Number(run.result?.tuple_count)).toBeGreaterThan(0)
    expect(Array.isArray(run.result?.ranked_results)).toBe(true)
    expect(run.result?.ranked_results?.length ?? 0).toBeGreaterThan(0)

    const inspection = await invokeNative<Record<string, unknown>>(
      'inspect_options_expression_tuple',
      {
        req: {
          artifact_path: artifactPath,
          tuple_index: 0,
        },
      },
    )
    expect(Object.keys(inspection).length).toBeGreaterThan(0)

    const validation = await invokeNative<OptionsValidationResponse>(
      'validate_options_expression_artifact',
      { req: validationRequest(artifactPath) },
    )
    const validationArtifactPath = text(validation.artifact_path)
    expect(fs.existsSync(validationArtifactPath)).toBe(true)
    expect(isPathInside(validationArtifactPath, environment.isolationRoot)).toBe(true)
    expect(
      Number(validation.result?.acceptance_summary?.validated_tuple_count),
    ).toBeGreaterThan(0)
    expect(validation.result?.tuple_results?.length ?? 0).toBeGreaterThan(0)

    const artifactHash = sha256File(artifactPath)
    const validationHash = sha256File(validationArtifactPath)
    await browser.reloadSession()
    await waitForShellReady()

    const recoveredDatasets = await invokeNative<OptionsDatasetManifest[]>(
      'list_options_datasets',
    )
    expect(
      recoveredDatasets.some(
        (entry) => text(entry.options_dataset_id) === RELEASE_PROOF_OPTIONS_DATASET_ID,
      ),
    ).toBe(true)
    const recoveredFeasibility = await invokeNative<OptionsFeasibilityResponse>(
      'analyze_options_expression_feasibility',
      { req: request },
    )
    const timing = recoveredFeasibility.replay_build_timing ?? {}
    expect(Boolean(timing.parent_signal_process_cache_hit)).toBe(false)
    expect(Boolean(timing.options_dataset_process_cache_hit)).toBe(false)
    expect(Boolean(timing.persisted_signal_manifest_cache_hit)).toBe(true)
    expect(Boolean(timing.options_dataset_persisted_cache_hit)).toBe(true)
    expect(sha256File(artifactPath)).toBe(artifactHash)
    expect(sha256File(validationArtifactPath)).toBe(validationHash)

    await openParentOptionsArtifact(parentArtifactPath, artifactPath)
    writeReleaseProofEvidence('options-lifecycle-proof.json', {
      contract: 'vectorgrid-exact-package-options-lifecycle-v1',
      schemaVersion: 1,
      exactBinary: environment.desktopBinary,
      parentArtifactPath,
      optionsDataset: {
        id: RELEASE_PROOF_OPTIONS_DATASET_ID,
        fixturePath: optionsCsvPath,
        storedPath: storedDatasetPath,
        rows: Number(imported.row_count),
        normalizedRowsSha256: text(imported.normalized_rows_sha256),
      },
      run: {
        artifactPath,
        artifactSha256: artifactHash,
        backendUsed: text(run.result?.backend_used),
        tupleCount: Number(run.result?.tuple_count),
      },
      validation: {
        artifactPath: validationArtifactPath,
        artifactSha256: validationHash,
        verdict: text(validation.result?.acceptance_summary?.verdict),
        validatedTupleCount: Number(
          validation.result?.acceptance_summary?.validated_tuple_count,
        ),
      },
      coldRecovery: {
        processCachesEmpty: true,
        persistedSignalManifestCacheHit: true,
        persistedOptionsDatasetCacheHit: true,
        visibleArtifactReloaded: true,
      },
      networkUsed: false,
      cleanupOwner: environment.isolationRoot,
    })
  })
})
