param(
  [string]$OutDir = "",
  [string]$CargoExecutable = "cargo",
  [switch]$ContractSimulation
)

$ErrorActionPreference = "Stop"

function Write-JsonUtf8 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    $Value,
    [int]$Depth = 16
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  $Value | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}

function Get-Sha256Hex {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $stream = [System.IO.File]::OpenRead($Path)
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  try {
    $bytes = $sha256.ComputeHash($stream)
    return ([System.BitConverter]::ToString($bytes)).Replace("-", "").ToLowerInvariant()
  } finally {
    $sha256.Dispose()
    $stream.Dispose()
  }
}

function Get-SourceContractHashes {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Root,
    [Parameter(Mandatory = $true)]
    [System.Collections.IDictionary]$Paths
  )

  $rows = @()
  foreach ($sourceId in $Paths.Keys) {
    $relativePath = [string]$Paths[$sourceId]
    $absolutePath = Join-Path $Root $relativePath
    if (-not (Test-Path -LiteralPath $absolutePath -PathType Leaf)) {
      throw "Live safety source contract path is missing: $relativePath"
    }
    $rows += [ordered]@{
      id = [string]$sourceId
      path = $relativePath.Replace("\", "/")
      sha256 = Get-Sha256Hex -Path $absolutePath
    }
  }
  return $rows
}

function Resolve-ExecutablePath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Value
  )

  $commands = @(
    Get-Command -Name $Value -ErrorAction Stop |
      Where-Object { $_.CommandType -in @("Application", "ExternalScript") }
  )
  if ($commands.Count -ne 1) {
    throw "Expected exactly one executable resolution for '$Value', observed $($commands.Count)"
  }
  $path = if (-not [string]::IsNullOrWhiteSpace([string]$commands[0].Source)) {
    [string]$commands[0].Source
  } else {
    [string]$commands[0].Path
  }
  if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Resolved Cargo executable is not a regular file: $path"
  }
  return [System.IO.Path]::GetFullPath($path)
}

function Assert-CpuFeatureContract {
  param(
    [Parameter(Mandatory = $true)]
    [string]$TauriManifestPath,
    [Parameter(Mandatory = $true)]
    [string]$AppCoreManifestPath
  )

  $tauriManifest = Get-Content -LiteralPath $TauriManifestPath -Raw
  $appCoreManifest = Get-Content -LiteralPath $AppCoreManifestPath -Raw
  $requirements = @(
    [ordered]@{
      label = "desktop default feature set"
      text = $tauriManifest
      pattern = '(?m)^default\s*=\s*\[\s*"custom-protocol"\s*,\s*"desktop-ui"\s*\]\s*$'
    },
    [ordered]@{
      label = "desktop-ui empty feature"
      text = $tauriManifest
      pattern = '(?m)^desktop-ui\s*=\s*\[\s*\]\s*$'
    },
    [ordered]@{
      label = "custom-protocol Tauri-only forwarding"
      text = $tauriManifest
      pattern = '(?m)^custom-protocol\s*=\s*\[\s*"tauri/custom-protocol"\s*\]\s*$'
    },
    [ordered]@{
      label = "desktop CUDA feature remains distinct"
      text = $tauriManifest
      pattern = '(?m)^cuda\s*=\s*\[\s*"ta_app_core/cuda"\s*,\s*"vector-ta/cuda"\s*\]\s*$'
    },
    [ordered]@{
      label = "app-core default features remain empty"
      text = $appCoreManifest
      pattern = '(?m)^default\s*=\s*\[\s*\]\s*$'
    },
    [ordered]@{
      label = "app-core CUDA feature remains distinct"
      text = $appCoreManifest
      pattern = '(?m)^cuda\s*=\s*\[\s*"vector-ta/cuda"\s*,\s*"dep:cust"\s*\]\s*$'
    }
  )
  foreach ($requirement in $requirements) {
    if (
      ([string]$requirement.text) -notmatch ([string]$requirement.pattern)
    ) {
      throw "CPU-only feature contract drifted: $($requirement.label)"
    }
  }
}

function Resolve-OutDir {
  param([string]$Value)

  if (-not [string]::IsNullOrWhiteSpace($Value)) {
    return [System.IO.Path]::GetFullPath($Value)
  }
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssfffZ")
  return Join-Path (Join-Path $repoRoot "output\release_candidates") "${stamp}-${PID}-live-safety"
}

function Assert-FreshEvidencePaths {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Directory,
    [Parameter(Mandatory = $true)]
    [string[]]$Names
  )

  if (Test-Path -LiteralPath $Directory -PathType Leaf) {
    throw "Live safety output path is a file, not a directory: $Directory"
  }
  foreach ($name in $Names) {
    $path = Join-Path $Directory $name
    if (Test-Path -LiteralPath $path) {
      throw "Live safety evidence path already exists; refusing to reuse stale evidence: $path"
    }
  }
}

function Get-CargoTestResults {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [AllowEmptyString()]
    [string[]]$Lines
  )

  $rows = @()
  foreach ($line in $Lines) {
    $match = [regex]::Match(
      [string]$line,
      "^\s*test\s+(?<name>\S+)\s+\.\.\.\s+(?<status>ok|FAILED|ignored)(?:\s|$)"
    )
    if (-not $match.Success) {
      continue
    }
    $rows += [ordered]@{
      name = $match.Groups["name"].Value
      status = $match.Groups["status"].Value.ToLowerInvariant()
      passed = $match.Groups["status"].Value -eq "ok"
    }
  }
  return $rows
}

function Restore-ProcessEnvironment {
  param(
    [Parameter(Mandatory = $true)]
    [System.Collections.IDictionary]$Snapshot
  )

  foreach ($name in $Snapshot.Keys) {
    $value = $Snapshot[$name]
    if ($null -eq $value) {
      Remove-Item "Env:$name" -ErrorAction SilentlyContinue
    } else {
      [Environment]::SetEnvironmentVariable($name, [string]$value, "Process")
    }
  }
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$resolvedOutDir = Resolve-OutDir -Value $OutDir
Assert-FreshEvidencePaths `
  -Directory $resolvedOutDir `
  -Names @(
    "live-safety-summary.json",
    "live-safety-cargo.stdout.log",
    "live-safety-cargo.stderr.log"
  )
New-Item -ItemType Directory -Force -Path $resolvedOutDir | Out-Null

$summaryPath = Join-Path $resolvedOutDir "live-safety-summary.json"
$stdoutPath = Join-Path $resolvedOutDir "live-safety-cargo.stdout.log"
$stderrPath = Join-Path $resolvedOutDir "live-safety-cargo.stderr.log"
$sourceContractPaths = [ordered]@{
  harness = "scripts/run_live_execution_safety_harness.ps1"
  commands_execution = "src-tauri/src/commands_execution.rs"
  execution_policy = "src-tauri/src/execution_policy.rs"
  execution_mocks = "src-tauri/src/execution_mocks.rs"
  execution_quantity = "src-tauri/src/execution_quantity.rs"
  settings = "src-tauri/src/settings.rs"
  run_artifacts = "src-tauri/src/run_artifacts.rs"
  tauri_manifest = "src-tauri/Cargo.toml"
  app_core_manifest = "crates/app_core/Cargo.toml"
  cargo_lock = "Cargo.lock"
}
$sourceHashesBefore = @(Get-SourceContractHashes -Root $repoRoot -Paths $sourceContractPaths)
$sourceHashesAfter = @()
$sourceContractStable = $false

# Every row below is an exact libtest identity. The harness runs the whole
# execution namespace once, then fails closed unless every authority row was
# observed as `ok` in that one process.
$requiredCommandsExecutionTests = @(
  "commands::commands_execution::tests::alpaca_keyring_service_name_scopes_to_app_identifier",
  "commands::commands_execution::tests::alpaca_credential_storage_status_tracks_presence_bits",
  "commands::commands_execution::tests::store_alpaca_api_credentials_rejects_empty_values_before_secure_storage",
  "commands::commands_execution::tests::normalize_alpaca_typed_override_rejects_partial_pairs",
  "commands::commands_execution::tests::build_live_session_seed_preserves_mode_and_source_run_context",
  "commands::commands_execution::tests::broker_open_order_status_matches_only_open_like_states",
  "commands::commands_execution::tests::broker_cancelable_status_is_stricter_than_action_blocking_status",
  "commands::commands_execution::tests::broker_terminal_status_matches_alpaca_terminal_lifecycle",
  "commands::commands_execution::tests::broker_status_classifier_marks_review_required_states",
  "commands::commands_execution::tests::alpaca_intent_status_tracks_reconciled_order_lifecycle",
  "commands::commands_execution::tests::broker_reconciliation_event_captures_partial_and_terminal_order_state",
  "commands::commands_execution::tests::open_orders_for_symbol_filters_to_open_current_symbol_rows",
  "commands::commands_execution::tests::cancelable_orders_for_symbol_uses_cancelable_status_subset",
  "commands::commands_execution::tests::cancel_order_snapshots_reports_partial_failure_without_hiding_completed_cancels",
  "commands::commands_execution::tests::broker_open_flatten_order_must_exactly_reduce_current_position",
  "commands::commands_execution::tests::owned_alpaca_protective_order_selection_is_target_symbol_and_intent_scoped",
  "commands::commands_execution::tests::guard_open_orders_before_fresh_delta_reconciles_target_already_in_flight",
  "commands::commands_execution::tests::guard_open_orders_before_fresh_delta_blocks_conflicting_open_orders",
  "commands::commands_execution::tests::guard_open_orders_before_fresh_delta_blocks_when_qty_metadata_is_missing",
  "commands::commands_execution::tests::alpaca_orders_url_can_scope_open_queries_to_a_symbol",
  "commands::commands_execution::tests::alpaca_client_order_lookup_url_uses_query_parameter",
  "commands::commands_execution::tests::alpaca_bracket_lookup_requires_nested_order_id_payload",
  "commands::commands_execution::tests::alpaca_client_order_id_is_stable_for_same_execution_intent",
  "commands::commands_execution::tests::alpaca_order_intent_is_persisted_without_credentials_and_can_update_status",
  "commands::commands_execution::tests::alpaca_order_intent_attempt_count_increments_for_retry",
  "commands::commands_execution::tests::alpaca_flatten_intent_retry_reuses_only_unconfirmed_matching_payload_and_terminal_history_gets_new_id",
  "commands::commands_execution::tests::unresolved_alpaca_intents_are_isolated_by_broker_target",
  "commands::commands_execution::tests::alpaca_intent_decode_requires_payload_hash_and_exact_client_order_binding",
  "commands::commands_execution::tests::alpaca_broker_response_validation_binds_full_payload_and_client_order_id",
  "commands::commands_execution::tests::non_retryable_broker_open_intent_cannot_be_reset_for_resubmission",
  "commands::commands_execution::tests::unresolved_alpaca_intent_restart_reconciliation_requires_position_refresh_for_fills",
  "commands::commands_execution::tests::alpaca_daily_order_count_is_durable_target_scoped_and_retry_safe",
  "commands::commands_execution::tests::alpaca_account_daily_pnl_accepts_numeric_strings_and_rejects_missing_data",
  "commands::commands_execution::tests::failed_alpaca_order_intent_replacement_preserves_previous_record",
  "commands::commands_execution::tests::corrupt_alpaca_order_intent_blocks_submission_with_reconciliation_guidance",
  "commands::commands_execution::tests::summarize_snapshot_orders_counts_full_filtered_open_set_before_truncation",
  "commands::commands_execution::tests::manual_execution_audit_failure_becomes_warning_only_after_submission",
  "commands::commands_execution::tests::rollback_runner_start_only_clears_matching_generation",
  "commands::commands_execution::tests::flatten_position_order_payload_handles_long_and_short_positions",
  "commands::commands_execution::tests::normalize_execution_safety_symbol_requires_non_blank_symbol",
  "commands::commands_execution::tests::emergency_halt_status_message_reflects_runner_and_feed_state",
  "commands::commands_execution::tests::ensure_runner_inactive_for_broker_safety_action_rejects_active_runner",
  "commands::commands_execution::tests::position_flips_are_staged_through_flat_before_opposite_entry",
  "commands::commands_execution::tests::ensure_runner_inactive_for_manual_broker_execution_blocks_only_live_broker_runs",
  "commands::commands_execution::tests::broker_safety_action_rechecks_runner_after_waiting_for_shared_lease",
  "commands::commands_execution::tests::runner_activation_requires_audit_slot_before_runner_becomes_visible",
  "commands::commands_execution::tests::credential_mutation_quiesces_active_and_concurrently_rearmed_runner_under_broker_lease",
  "commands::commands_execution::tests::resolve_alpaca_credentials_reads_stored_pair_by_default",
  "commands::commands_execution::tests::resolve_alpaca_credentials_prefers_complete_typed_override",
  "commands::commands_execution::tests::execution_target_normalization_blocks_legacy_broker_routing_and_pins_explicit_modes",
  "commands::commands_execution::tests::manual_execution_orchestration_begins_audit_before_starting_broker_work",
  "commands::commands_execution::tests::manual_broker_execution_audit_is_durable_before_submission_attempt",
  "commands::commands_execution::tests::live_session_exact_path_finalization_cannot_clear_replacement_session",
  "commands::commands_execution::tests::alpaca_live_validation_verification_accepts_exact_approved_evidence",
  "commands::commands_execution::tests::alpaca_live_validation_verification_rejects_cost_drift",
  "commands::commands_execution::tests::alpaca_live_validation_verification_rejects_unapproved_run_verdict",
  "commands::commands_execution::tests::alpaca_live_validation_verification_rejects_inconsistent_approval_counts",
  "commands::commands_execution::tests::alpaca_live_validation_verification_rejects_research_only_promotion",
  "commands::commands_execution::tests::alpaca_live_validation_verification_rejects_dataset_content_drift",
  "commands::commands_execution::tests::local_paper_transaction_costs_follow_explicit_and_legacy_strategy_assumptions",
  "commands::commands_execution::tests::alpaca_account_tradeable_validation_blocks_non_active_or_blocked_accounts",
  "commands::commands_execution::tests::alpaca_market_clock_validation_requires_open_session",
  "commands::commands_execution::tests::live_execution_source_context_rejects_unverified_nonempty_fingerprint",
  "commands::commands_execution::tests::execution_strategy_support_matrix_is_code_backed_and_fail_closed",
  "commands::commands_execution::tests::unresolved_alpaca_intent_blocks_new_client_order_for_same_symbol",
  "commands::commands_execution::tests::unresolved_alpaca_intent_statuses_fail_closed_for_review_required_states",
  "commands::commands_execution::tests::submit_or_reuse_alpaca_order_avoids_duplicate_submit_when_lookup_finds_existing_order",
  "commands::commands_execution::tests::submit_or_reuse_alpaca_order_rechecks_after_submit_error_before_failing",
  "commands::commands_execution::tests::open_position_from_trades_uses_latest_open_trade",
  "commands::commands_execution::tests::risk_override_sanitizes_values",
  "commands::commands_execution::tests::execution_risk_policy_merge_lets_request_overrides_tighten_persisted_policy",
  "commands::commands_execution::tests::execution_risk_policy_merge_prevents_request_overrides_from_loosening_saved_policy",
  "commands::commands_execution::tests::execution_risk_policy_merge_rejects_invalid_request_caps",
  "commands::commands_execution::tests::json_field_as_string_supports_scalar_types",
  "commands::commands_execution::tests::canonical_symbol_key_preserves_meaningful_punctuation",
  "commands::commands_execution::tests::symbol_matching_does_not_collapse_punctuation_variants",
  "commands::commands_execution::tests::normalize_execute_request_rejects_missing_symbol_and_invalid_qty",
  "commands::commands_execution::tests::normalize_execute_request_quantizes_qty_to_1e4_boundaries",
  "commands::commands_execution::tests::delta_qty_hold_threshold_is_strictly_less_than_1e6",
  "commands::commands_execution::tests::opening_bracket_threshold_uses_current_lt_and_target_gt_1e6",
  "commands::commands_execution::tests::local_paper_bracket_spec_requires_both_bounds_and_documents_tie_break",
  "commands::commands_execution::tests::local_paper_bracket_preview_builds_non_mutating_child_legs_for_long_and_short",
  "commands::commands_execution::tests::local_paper_bracket_preview_skips_missing_bounds_and_non_opening_deltas",
  "commands::commands_execution::tests::live_runner_stream_preconditions_require_active_stream",
  "commands::commands_execution::tests::live_runner_stream_preconditions_require_matching_symbol",
  "commands::commands_execution::tests::live_runner_stream_preconditions_accept_canonical_symbol_match",
  "commands::commands_execution::tests::live_runner_stream_preconditions_reject_live_broker_on_delayed_stream",
  "commands::commands_execution::tests::live_runner_stream_preconditions_require_recent_accepted_bar",
  "commands::commands_execution::tests::live_runner_stream_preconditions_reject_implausible_future_receive_time",
  "commands::commands_execution::tests::base_url_maps_paper_and_live_modes",
  "commands::commands_execution::tests::alpaca_http_client_policy_is_https_only_redirect_free_and_endpoints_are_pinned",
  "commands::commands_execution::tests::alpaca_path_segment_urls_percent_encode_symbols_and_order_ids",
  "commands::commands_execution::tests::alpaca_path_segment_urls_reject_empty_segments",
  "commands::commands_execution::tests::alpaca_http_error_message_redacts_credentials_from_broker_body",
  "commands::commands_execution::tests::normalize_order_snapshot_reduces_broker_payload_to_review_fields",
  "commands::commands_execution::tests::normalize_position_snapshot_requires_symbol_and_preserves_position_fields",
  "commands::commands_execution::tests::alpaca_order_payload_attaches_bracket_only_when_both_risk_bounds_exist",
  "commands::commands_execution::tests::alpaca_price_formatting_uses_broker_precision_rules",
  "commands::commands_execution::tests::alpaca_order_payload_rejects_bracket_prices_that_round_to_inversion",
  "commands::commands_execution::tests::alpaca_order_payload_uses_four_decimals_below_one_dollar",
  "commands::commands_execution::tests::runner_status_from_runtime_reports_config_and_counters",
  "commands::commands_execution::tests::runner_status_from_runtime_none_is_inactive_and_zeroed",
  "commands::commands_execution::tests::pending_bar_coalesces_three_inflight_bars_to_latest_only",
  "commands::commands_execution::tests::pending_bar_rejects_stale_or_equal_timestamps",
  "commands::commands_execution::tests::generation_gate_ignores_stale_runtime_updates",
  "commands::commands_execution::tests::runner_abort_handle_sets_cancellation_marker",
  "commands::commands_execution::tests::with_active_runner_generation_ignores_stale_runtime_generations"
)

$requiredExecutionPolicyTests = @(
  "execution_policy::tests::allows_when_no_policy_caps_are_configured",
  "execution_policy::tests::blocks_invalid_market_price_when_an_order_is_needed",
  "execution_policy::tests::blocks_explicit_notional_caps_and_short_policy",
  "execution_policy::tests::no_short_policy_allows_partial_and_full_covers",
  "execution_policy::tests::no_short_policy_blocks_increased_or_new_short_exposure",
  "execution_policy::tests::sign_flip_is_risk_increasing_even_when_target_absolute_exposure_is_smaller",
  "execution_policy::tests::blocks_required_bracket_only_for_new_positions",
  "execution_policy::tests::blocks_daily_order_and_loss_caps_and_fails_closed_without_state",
  "execution_policy::tests::preview_without_broker_state_warns_instead_of_blocking_daily_limits",
  "execution_policy::tests::daily_caps_do_not_block_risk_reducing_orders",
  "execution_policy::tests::notional_caps_do_not_block_risk_reducing_orders"
)

$requiredOtherExecutionTests = @(
  "execution_mocks::tests::mock_broker_models_fills_partials_rejects_timeouts_and_cancel_failures",
  "execution_mocks::tests::mock_feed_classifies_normal_duplicate_out_of_order_close_and_error_events",
  "execution_quantity::tests::normalizes_midpoints_at_one_ten_thousandth_boundaries",
  "execution_quantity::tests::rejects_invalid_and_too_small_quantities",
  "settings::tests::execution_risk_policy_defaults_are_conservative_when_not_configured",
  "settings::tests::set_persists_and_resolves_execution_risk_policy_settings",
  "settings::tests::clearing_execution_risk_policy_settings_restores_default_resolution",
  "settings::tests::set_execution_risk_policy_rejects_invalid_notional_caps",
  "settings::tests::configured_execution_risk_policy_rejects_invalid_persisted_values",
  "settings::tests::set_execution_risk_policy_rejects_invalid_daily_limits"
)

$requiredCoverage = [ordered]@{
  commands_execution_complete = $requiredCommandsExecutionTests
  execution_policy_complete = $requiredExecutionPolicyTests
  execution_mocks_quantity_and_settings = $requiredOtherExecutionTests
}

$requiredTests = New-Object System.Collections.Generic.List[string]
foreach ($category in $requiredCoverage.Keys) {
  foreach ($testName in @($requiredCoverage[$category])) {
    if (-not $requiredTests.Contains([string]$testName)) {
      $requiredTests.Add([string]$testName)
    }
  }
}

$credentialEnvNames = @(
  "ALPACA_API_KEY",
  "ALPACA_API_KEY_ID",
  "ALPACA_API_SECRET",
  "ALPACA_API_SECRET_KEY",
  "ALPACA_KEY",
  "ALPACA_KEY_ID",
  "ALPACA_SECRET",
  "ALPACA_SECRET_KEY",
  "ALPACA_API_BASE_URL",
  "APCA_API_KEY",
  "APCA_API_KEY_ID",
  "APCA_API_SECRET",
  "APCA_API_SECRET_KEY",
  "APCA_API_BASE_URL",
  "POLYGON_KEY",
  "POLYGON_API_KEY",
  "POLYGON_TOKEN",
  "POLYGON_API_BASE_URL"
)
$networkEnvNames = @(
  "HTTP_PROXY",
  "HTTPS_PROXY",
  "ALL_PROXY",
  "NO_PROXY",
  "http_proxy",
  "https_proxy",
  "all_proxy",
  "no_proxy",
  "CARGO_NET_OFFLINE",
  "CARGO_TERM_COLOR",
  "RUST_TEST_THREADS"
)
$wrapperEnvNames = @(
  "RUSTC_WRAPPER",
  "CARGO_BUILD_RUSTC_WRAPPER",
  "SCCACHE_NO_DAEMON"
)
$managedEnvNames = @($credentialEnvNames + $networkEnvNames + $wrapperEnvNames)
$environmentSnapshot = [System.Collections.Generic.Dictionary[string, object]]::new(
  [System.StringComparer]::Ordinal
)
foreach ($name in $managedEnvNames) {
  $environmentSnapshot[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

$cargoArguments = @(
  "test",
  "--locked",
  "--offline",
  "-p",
  "ta_desktop_demo_app",
  "--no-default-features",
  "--features",
  "desktop-ui,custom-protocol",
  "--lib",
  "execution",
  "--",
  "--test-threads=1"
)
$cargoExitCode = $null
$resolvedCargoExecutable = $null
$cargoExecutableSha256 = $null
$parsedTests = @()
$coverageRows = @()
$failureMessage = $null
$invocationCount = 0
$credentialEnvironmentCleared = $false
$cargoOfflineConfigured = $false
$dependencyLockEnforced = $false
$externalHttpProxyTrapConfigured = $false
$rustTestThreadsConfigured = $false
$cpuFeatureContractVerified = $false
$authorityMode = -not [bool]$ContractSimulation
$modeLabel = if ($authorityMode) { "authority" } else { "contract simulation" }

try {
  Push-Location $repoRoot

  if ($authorityMode -and $CargoExecutable -ne "cargo") {
    throw "Custom -CargoExecutable values are allowed only with -ContractSimulation and are never authority-eligible."
  }
  $resolvedCargoExecutable = Resolve-ExecutablePath -Value $CargoExecutable
  $cargoExecutableSha256 = Get-Sha256Hex -Path $resolvedCargoExecutable
  Assert-CpuFeatureContract `
    -TauriManifestPath (Join-Path $repoRoot "src-tauri\Cargo.toml") `
    -AppCoreManifestPath (Join-Path $repoRoot "crates\app_core\Cargo.toml")
  $cpuFeatureContractVerified = $true

  foreach ($name in $credentialEnvNames) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
  }
  $remainingCredentialNames = @(
    $credentialEnvNames | Where-Object {
      $null -ne [Environment]::GetEnvironmentVariable($_, "Process")
    }
  )
  $credentialEnvironmentCleared = $remainingCredentialNames.Count -eq 0
  if ($remainingCredentialNames.Count -gt 0) {
    throw "Credential-free preflight failed to clear: $($remainingCredentialNames -join ', ')"
  }

  # Cargo cannot resolve or download dependencies, and reqwest's standard
  # environment proxy discovery is redirected to the local discard port.
  # Loopback remains available for deterministic mock-server tests.
  $env:CARGO_NET_OFFLINE = "true"
  $env:CARGO_TERM_COLOR = "never"
  $env:RUST_TEST_THREADS = "1"
  $env:HTTP_PROXY = "http://127.0.0.1:9"
  $env:HTTPS_PROXY = "http://127.0.0.1:9"
  $env:ALL_PROXY = "http://127.0.0.1:9"
  $env:NO_PROXY = "localhost,127.0.0.1,::1"
  $env:http_proxy = $env:HTTP_PROXY
  $env:https_proxy = $env:HTTPS_PROXY
  $env:all_proxy = $env:ALL_PROXY
  $env:no_proxy = $env:NO_PROXY
  $env:RUSTC_WRAPPER = ""
  $env:CARGO_BUILD_RUSTC_WRAPPER = ""
  $env:SCCACHE_NO_DAEMON = "1"
  $cargoOfflineConfigured = $env:CARGO_NET_OFFLINE -eq "true" -and $cargoArguments -contains "--offline"
  $dependencyLockEnforced = $cargoArguments -contains "--locked"
  $externalHttpProxyTrapConfigured =
    $env:HTTP_PROXY -eq "http://127.0.0.1:9" -and
    $env:HTTPS_PROXY -eq "http://127.0.0.1:9" -and
    $env:ALL_PROXY -eq "http://127.0.0.1:9" -and
    $env:NO_PROXY -eq "localhost,127.0.0.1,::1"
  $rustTestThreadsConfigured =
    $env:RUST_TEST_THREADS -eq "1" -and $cargoArguments -contains "--test-threads=1"
  if (
    -not $cargoOfflineConfigured -or
    -not $dependencyLockEnforced -or
    -not $externalHttpProxyTrapConfigured -or
    -not $rustTestThreadsConfigured
  ) {
    throw "Live safety isolation preflight could not verify the offline, locked, proxy-trap, and serialized test controls."
  }

  Write-Host "Execution safety ${modeLabel}: one production-credential-cleared, offline, explicit-CPU Cargo invocation."
  Write-Host "==> cargo test execution safety namespace"
  $invocationCount += 1
  $proc = Start-Process -FilePath $resolvedCargoExecutable `
    -ArgumentList $cargoArguments `
    -NoNewWindow `
    -Wait `
    -PassThru `
    -RedirectStandardOutput $stdoutPath `
    -RedirectStandardError $stderrPath
  $cargoExitCode = $proc.ExitCode
  $sourceHashesAfter = @(Get-SourceContractHashes -Root $repoRoot -Paths $sourceContractPaths)
  $changedSources = @()
  foreach ($before in $sourceHashesBefore) {
    $after = @($sourceHashesAfter | Where-Object { $_.id -eq $before.id })
    if ($after.Count -ne 1 -or [string]$after[0].sha256 -ne [string]$before.sha256) {
      $changedSources += [string]$before.path
    }
  }
  $sourceContractStable = $changedSources.Count -eq 0

  $stdoutLines = if (Test-Path -LiteralPath $stdoutPath) {
    @(Get-Content -LiteralPath $stdoutPath)
  } else {
    @()
  }
  $stderrLines = if (Test-Path -LiteralPath $stderrPath) {
    @(Get-Content -LiteralPath $stderrPath)
  } else {
    @()
  }
  foreach ($line in @($stdoutLines + $stderrLines)) {
    Write-Host $line
  }

  $parsedTests = @(Get-CargoTestResults -Lines @($stdoutLines + $stderrLines))
  $observed = @{}
  foreach ($row in $parsedTests) {
    $observed[[string]$row.name] = [string]$row.status
  }
  foreach ($category in $requiredCoverage.Keys) {
    $categoryTests = @()
    $categoryPassed = $true
    foreach ($testName in @($requiredCoverage[$category])) {
      $status = if ($observed.ContainsKey([string]$testName)) {
        [string]$observed[[string]$testName]
      } else {
        "missing"
      }
      $passed = $status -eq "ok"
      if (-not $passed) {
        $categoryPassed = $false
      }
      $categoryTests += [ordered]@{
        name = [string]$testName
        status = $status
        passed = $passed
      }
    }
    $coverageRows += [ordered]@{
      category = [string]$category
      passed = $categoryPassed
      tests = $categoryTests
    }
  }

  if ($cargoExitCode -ne 0) {
    throw "Execution safety Cargo invocation failed with exit code $cargoExitCode"
  }
  if (-not $sourceContractStable) {
    throw "Live safety source contract changed during the Cargo invocation: $($changedSources -join ', ')"
  }
  $missingOrFailed = @(
    $coverageRows |
      ForEach-Object { $_.tests } |
      Where-Object { -not $_.passed }
  )
  if ($missingOrFailed.Count -gt 0) {
    $details = @(
      $missingOrFailed | ForEach-Object { "$($_.name)=$($_.status)" }
    )
    throw "Execution safety authority did not observe every required exact test as passing: $($details -join ', ')"
  }
  if ($invocationCount -ne 1) {
    throw "Execution safety authority expected exactly one Cargo invocation, observed $invocationCount"
  }
} catch {
  $failureMessage = $_.Exception.Message
} finally {
  Pop-Location -ErrorAction SilentlyContinue
  Restore-ProcessEnvironment -Snapshot $environmentSnapshot
}

$passed = [string]::IsNullOrWhiteSpace($failureMessage)
$authorityEligible =
  $passed -and
  $authorityMode -and
  $credentialEnvironmentCleared -and
  $cargoOfflineConfigured -and
  $dependencyLockEnforced -and
  $externalHttpProxyTrapConfigured -and
  $rustTestThreadsConfigured -and
  $cpuFeatureContractVerified -and
  $sourceContractStable -and
  $invocationCount -eq 1
$summary = [ordered]@{
  schema_version = 2
  report_kind = if ($authorityMode) {
    "live-execution-safety-authority"
  } else {
    "live-execution-safety-contract-simulation"
  }
  run_mode = if ($authorityMode) { "authority" } else { "contract_simulation" }
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  status = if ($passed) { "passed" } else { "failed" }
  passed = $passed
  authority_eligible = $authorityEligible
  error = if ($passed) { $null } else { $failureMessage }
  authority_scope = [ordered]@{
    claim = "offline unit-contract authority for the enumerated live-execution safety invariants with the production credential environment cleared"
    excludes = @(
      "end-to-end command execution or broker I/O",
      "broker connectivity or success",
      "order profitability",
      "credentialed provider behavior",
      "manual disaster-recovery evidence"
    )
  }
  isolation = [ordered]@{
    cargo_offline = $cargoOfflineConfigured
    dependency_lock_enforced = $dependencyLockEnforced
    credential_environment_cleared = $credentialEnvironmentCleared
    credential_environment_names = $credentialEnvNames
    credential_test_contract = "required credential rows use the synthetic cfg(test) store; production credential environment variables are removed"
    external_http_proxy_trap_configured = $externalHttpProxyTrapConfigured
    external_http_proxy = if ($externalHttpProxyTrapConfigured) {
      "http://127.0.0.1:9"
    } else {
      $null
    }
    loopback_mock_hosts = @("localhost", "127.0.0.1", "::1")
    rust_test_threads = if ($rustTestThreadsConfigured) { 1 } else { $null }
    operating_system_network_isolation = $false
    proxy_bypassing_clients_excluded = $true
  }
  cargo = [ordered]@{
    invocation_count = $invocationCount
    requested_executable = $CargoExecutable
    resolved_executable_path = $resolvedCargoExecutable
    executable_sha256 = $cargoExecutableSha256
    arguments = $cargoArguments
    feature_contract = if ($cpuFeatureContractVerified) {
      "verified explicit desktop-ui,custom-protocol CPU feature set; default and cuda features disabled"
    } else {
      "unverified"
    }
    feature_contract_verified = $cpuFeatureContractVerified
    exit_code = $cargoExitCode
    stdout_log = [System.IO.Path]::GetFileName($stdoutPath)
    stderr_log = [System.IO.Path]::GetFileName($stderrPath)
  }
  required_test_count = $requiredTests.Count
  observed_test_count = $parsedTests.Count
  source_contract_stable = $sourceContractStable
  source_contract_before = $sourceHashesBefore
  source_contract_after = $sourceHashesAfter
  coverage = $coverageRows
}
Write-JsonUtf8 -Path $summaryPath -Value $summary

if (-not $passed) {
  throw $failureMessage
}

Write-Host "Live execution safety $modeLabel passed."
Write-Host "Summary: $summaryPath"
