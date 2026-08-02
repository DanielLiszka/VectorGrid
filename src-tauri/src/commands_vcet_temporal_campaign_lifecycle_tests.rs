use super::super::vcet_temporal_durable_ledger::{
    admit_durable_final_authorization, admit_durable_pre_final_winner_lock, DurableFinalAdmission,
};
use super::*;
use crate::commands::commands_campaign::vcet_test_support::{
    build_sealed_cpu_vcet_campaign_source, SealedCpuVcetCampaignSourceFixture,
};
use crate::state::{AppState, BackgroundTaskState};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use ta_app_core::vcet_temporal::{
    VcetTemporalFoldModeV1, VcetTemporalMinimumSupportV1, VcetTemporalRequestedBoundsV1,
    VcetTemporalUtcBoundsV1, VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
};
use tauri::Manager;

const BAR_COUNT: usize = 280;
static MOCK_APP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct LifecycleFixture {
    app: tauri::App<tauri::test::MockRuntime>,
    base: PathBuf,
    source: SealedCpuVcetCampaignSourceFixture,
}

impl LifecycleFixture {
    fn new(label: &str) -> Self {
        let app = build_mock_app(label);
        let base = app
            .handle()
            .path()
            .app_local_data_dir()
            .expect("resolve isolated mock app-data root");
        fs::create_dir_all(&base).expect("create isolated mock app-data root");
        let inputs = base.join("lifecycle-inputs");
        fs::create_dir_all(&inputs).expect("create lifecycle input directory");
        let first = inputs.join("first.csv");
        let second = inputs.join("second.csv");
        write_wave_csv(&first, 0.025);
        write_wave_csv(&second, 0.035);
        let source = build_sealed_cpu_vcet_campaign_source(
            app.handle(),
            [first.as_path(), second.as_path()],
        )
        .expect("build sealed CPU VCET source campaign");
        Self { app, base, source }
    }

    fn prepare(&self) -> VcetTemporalCampaignStatusV1 {
        prepare_vcet_temporal_campaign_blocking(
            PrepareVcetTemporalCampaignRequestV1 {
                source_campaign_summary_path: self.source.summary_json_path.clone(),
                plan: temporal_plan(),
                report_label: Some("CPU lifecycle integration".to_string()),
            },
            self.app.handle().clone(),
            &self.base,
        )
        .expect("prepare product campaign")
    }
}

impl Drop for LifecycleFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn build_mock_app(label: &str) -> tauri::App<tauri::test::MockRuntime> {
    let counter = MOCK_APP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let token = format!(
        "vectorgrid-{label}-{}-{nanos}-{counter}",
        std::process::id()
    );
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().product_name = Some(token.clone());
    context.config_mut().identifier = format!("com.vectorgrid.tests.{}", token.replace('-', ""));
    tauri::test::mock_builder()
        .manage(AppState::default())
        .manage(BackgroundTaskState::default())
        .build(context)
        .expect("build isolated Tauri mock app")
}

fn write_wave_csv(path: &Path, slope: f64) {
    let mut csv = String::from("timestamp,open,high,low,close,volume\n");
    for index in 0..BAR_COUNT {
        let wave = (index as f64 * 0.35).sin() * 4.0;
        let price = 100.0 + index as f64 * slope + wave;
        csv.push_str(&format!(
            "{},{:.8},{:.8},{:.8},{:.8},{:.2}\n",
            index * 60_000,
            price - 0.05,
            price + 0.50,
            price - 0.50,
            price,
            1_000.0 + index as f64,
        ));
    }
    fs::write(path, csv).expect("write deterministic VCET lifecycle CSV");
}

fn temporal_plan() -> VcetTemporalValidationPlanV1 {
    VcetTemporalValidationPlanV1 {
        schema_version: VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
        requested_bounds: VcetTemporalRequestedBoundsV1 {
            utc: VcetTemporalUtcBoundsV1 {
                start_utc_ms_inclusive: 0,
                end_utc_ms_inclusive: (BAR_COUNT as i64 - 1) * 60_000,
            },
            session: None,
        },
        fold_mode: VcetTemporalFoldModeV1::Anchored,
        train_bars: 64,
        validation_bars: 24,
        final_test_bars: 32,
        step_bars: 32,
        purge_bars: 2,
        embargo_bars: 2,
        warmup_bars: 64,
        position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1::ResetFlatDiscardOpen,
        minimum_support: VcetTemporalMinimumSupportV1 {
            min_folds: 2,
            min_train_bars: 64,
            min_validation_bars: 24,
            min_final_test_bars: 32,
            min_train_trades_per_fold: 1,
            min_validation_trades_per_fold: 1,
            min_final_test_trades: 1,
        },
    }
}

fn status_request(status: &VcetTemporalCampaignStatusV1) -> GetVcetTemporalCampaignStatusRequestV1 {
    GetVcetTemporalCampaignStatusRequestV1 {
        preparation_artifact_path: status.preparation_artifact_path.clone(),
        expected_preparation_artifact_sha256: status.preparation_artifact_sha256.clone(),
    }
}

fn selection_request(
    status: &VcetTemporalCampaignStatusV1,
) -> RunVcetTemporalCampaignSelectionRequestV1 {
    RunVcetTemporalCampaignSelectionRequestV1 {
        preparation_artifact_path: status.preparation_artifact_path.clone(),
        expected_preparation_artifact_sha256: status.preparation_artifact_sha256.clone(),
    }
}

fn final_request(
    status: &VcetTemporalCampaignStatusV1,
    expected_winner_lock_sha256: String,
    confirm_final_test_access: bool,
) -> OpenVcetTemporalCampaignFinalTestRequestV1 {
    OpenVcetTemporalCampaignFinalTestRequestV1 {
        preparation_artifact_path: status.preparation_artifact_path.clone(),
        expected_preparation_artifact_sha256: status.preparation_artifact_sha256.clone(),
        expected_winner_lock_sha256,
        confirm_final_test_access,
    }
}

fn load_status(status: &VcetTemporalCampaignStatusV1, base: &Path) -> VcetTemporalCampaignStatusV1 {
    get_vcet_temporal_campaign_status_blocking(status_request(status), base)
        .expect("load product campaign status")
}

fn assert_xlsx(path: Option<&String>, label: &str) {
    let path = Path::new(path.unwrap_or_else(|| panic!("{label} path")));
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {label}: {error}"));
    assert!(bytes.len() > 4, "{label} must not be empty");
    assert_eq!(&bytes[..2], b"PK", "{label} must be an XLSX ZIP package");
}

fn snapshot_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, current: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(current)
            .expect("read campaign snapshot directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect campaign snapshot entries");
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = entry.metadata().expect("read campaign snapshot metadata");
            if metadata.is_dir() {
                visit(root, &path, out);
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .expect("campaign snapshot path is below root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(
                    relative,
                    fs::read(path).expect("read campaign snapshot file"),
                );
            }
        }
    }

    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

fn remove_file(path: impl AsRef<Path>, label: &str) {
    fs::remove_file(path.as_ref())
        .unwrap_or_else(|error| panic!("remove {label} {}: {error}", path.as_ref().display()));
}

fn hide_csv(path: &str) -> PathBuf {
    let source = PathBuf::from(path);
    let hidden = PathBuf::from(format!("{}.offline", source.to_string_lossy()));
    fs::rename(&source, &hidden).unwrap_or_else(|error| {
        panic!(
            "hide mutable CSV {} as {}: {error}",
            source.display(),
            hidden.display()
        )
    });
    hidden
}

#[test]
fn cpu_product_lifecycle_requires_explicit_final_confirmation_without_reranking() {
    let fixture = LifecycleFixture::new("vcettemporalfull");
    let prepared = fixture.prepare();
    assert_eq!(prepared.state, VcetTemporalCampaignStateV1::Prepared);
    assert_eq!(prepared.candidate_count, 2);
    assert_eq!(prepared.dataset_count, 2);
    assert_eq!(prepared.selection_dataset_count, 1);
    assert_eq!(prepared.final_test_dataset_count, 1);
    assert_eq!(prepared.selection_shard_count, 1);
    assert_eq!(prepared.final_test_shard_count, 1);
    assert_eq!(prepared.completed_selection_shard_count, 0);
    assert_eq!(prepared.completed_final_test_shard_count, 0);
    assert_eq!(prepared.next_required_action, "run_selection");
    assert!(!prepared.execution_blocked);
    assert!(prepared.winner_candidate_id.is_none());
    assert!(prepared.winner_lock_sha256.is_none());
    assert_eq!(
        serde_json::to_value(load_status(&prepared, &fixture.base)).expect("encode status"),
        serde_json::to_value(&prepared).expect("encode prepared status")
    );

    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run CPU selection stage");
    assert_eq!(
        selected.state,
        VcetTemporalCampaignStateV1::SelectionComplete
    );
    assert_eq!(
        selected.completed_selection_shard_count,
        selected.selection_shard_count
    );
    assert_eq!(selected.completed_final_test_shard_count, 0);
    assert_eq!(selected.next_required_action, "confirm_and_open_final_test");
    assert!(selected.pre_final_reduction_artifact_path.is_some());
    assert!(selected.pre_final_reduction_artifact_sha256.is_some());
    assert!(selected.final_reduction_artifact_path.is_none());
    assert!(selected.final_report_xlsx_path.is_none());
    assert_xlsx(
        selected.selection_report_xlsx_path.as_ref(),
        "selection report",
    );
    let winner_id = selected
        .winner_candidate_id
        .clone()
        .expect("selection winner candidate ID");
    assert!(fixture.source.candidate_ids.contains(&winner_id));
    let winner_lock = selected
        .winner_lock_sha256
        .clone()
        .expect("selection winner lock");
    let selected_value = serde_json::to_value(load_status(&selected, &fixture.base))
        .expect("encode selected status");
    assert_eq!(
        selected_value,
        serde_json::to_value(&selected).expect("encode selection response")
    );

    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let files_before_rejected_access = snapshot_files(&fixture.base);
    let false_confirmation = open_vcet_temporal_campaign_final_test_blocking(
        final_request(&selected, winner_lock.clone(), false),
        &fixture.base,
    )
    .expect_err("false final-test confirmation must be rejected");
    assert!(false_confirmation.contains("confirm_final_test_access=true"));
    assert_eq!(snapshot_files(&fixture.base), files_before_rejected_access);
    assert_eq!(
        serde_json::to_value(load_status(&selected, &fixture.base)).expect("encode status"),
        selected_value
    );

    let mut wrong_winner_lock = format!("sha256:{}", "0".repeat(64));
    if wrong_winner_lock == winner_lock {
        wrong_winner_lock = format!("sha256:{}", "1".repeat(64));
    }
    let wrong_confirmation = open_vcet_temporal_campaign_final_test_blocking(
        final_request(&selected, wrong_winner_lock, true),
        &fixture.base,
    )
    .expect_err("foreign winner lock must be rejected");
    assert!(wrong_confirmation.contains("winner-lock confirmation mismatch"));
    assert_eq!(snapshot_files(&fixture.base), files_before_rejected_access);
    assert_eq!(
        serde_json::to_value(load_status(&selected, &fixture.base)).expect("encode status"),
        selected_value
    );

    let completed = open_vcet_temporal_campaign_final_test_blocking(
        final_request(&selected, winner_lock.clone(), true),
        &fixture.base,
    )
    .expect("open and complete explicit final-test stage");
    assert_eq!(
        completed.state,
        VcetTemporalCampaignStateV1::FinalTestComplete
    );
    assert_eq!(
        completed.completed_final_test_shard_count,
        completed.final_test_shard_count
    );
    assert_eq!(completed.next_required_action, "none");
    assert_eq!(
        completed.winner_candidate_id.as_deref(),
        Some(winner_id.as_str())
    );
    assert_eq!(
        completed.winner_lock_sha256.as_deref(),
        Some(winner_lock.as_str())
    );
    assert_eq!(
        completed.pre_final_reduction_artifact_sha256,
        selected.pre_final_reduction_artifact_sha256
    );
    assert!(completed.final_reduction_artifact_path.is_some());
    assert!(completed.final_reduction_artifact_sha256.is_some());
    assert_xlsx(completed.final_report_xlsx_path.as_ref(), "final report");
    let preparation = load_preparation(
        Path::new(&completed.preparation_artifact_path),
        &completed.preparation_artifact_sha256,
    )
    .expect("load completed preparation");
    let pre_final =
        load_pre_final_reduction_artifact(&pre_final_reduction_path(campaign_dir), &preparation)
            .expect("load locked pre-final winner reduction");
    let final_reduction = load_final_reduction_artifact(
        &final_reduction_path(campaign_dir),
        &preparation,
        &pre_final,
    )
    .expect("load exact final reduction");
    assert_eq!(final_reduction.payload.final_test_candidate_count, 1);
    assert_eq!(
        final_reduction
            .payload
            .final_test_candidate_window_trial_count,
        final_reduction.payload.evaluated_final_test_dataset_count
    );
    assert!(!final_reduction.payload.selection_used_final_test);
    assert!(!final_reduction.payload.winner_reranked_after_final_test);
    assert_eq!(
        final_reduction.payload.winner_lock.candidate_id.as_str(),
        winner_id.as_str()
    );
    assert_eq!(
        final_reduction
            .payload
            .winner_lock
            .winner_lock_sha256
            .as_str(),
        winner_lock.as_str()
    );
    assert!(final_reduction
        .payload
        .datasets
        .iter()
        .all(|dataset| dataset.candidate_id == final_reduction.payload.winner_lock.candidate_id));
    assert_eq!(
        serde_json::to_value(load_status(&completed, &fixture.base)).expect("encode status"),
        serde_json::to_value(&completed).expect("encode completed status")
    );
}

#[test]
fn durable_stage_projections_and_reports_recover_without_mutable_csv_access() {
    let fixture = LifecycleFixture::new("vcettemporalrecovery");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run initial CPU selection");
    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let original_pre_final_sha256 = selected
        .pre_final_reduction_artifact_sha256
        .clone()
        .expect("initial pre-final reduction SHA-256");
    let original_winner_lock = selected
        .winner_lock_sha256
        .clone()
        .expect("initial winner lock");
    let preparation = load_preparation(
        Path::new(&selected.preparation_artifact_path),
        &selected.preparation_artifact_sha256,
    )
    .expect("load preparation for selection recovery");
    let selection_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::Selection,
    );
    assert_eq!(selection_shards.len(), 1);
    let selection_shard_projection =
        pre_final_shard_path(campaign_dir, selection_shards[0].shard_ordinal);
    let original_reduction =
        load_pre_final_reduction_artifact(&pre_final_reduction_path(campaign_dir), &preparation)
            .expect("load initial pre-final reduction");
    let expected_selection_shard_sha256 = original_reduction.payload.ordered_shards[0]
        .artifact_sha256
        .clone();
    remove_file(&selection_shard_projection, "selection shard projection");
    let reconstructed_selection =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&selected), &fixture.base)
            .expect("reconstruct selection shard against the durable winner lock");
    assert_eq!(
        reconstructed_selection
            .pre_final_reduction_artifact_sha256
            .as_deref(),
        Some(original_pre_final_sha256.as_str())
    );
    let reconstructed_shard = load_pre_final_shard_artifact(
        &selection_shard_projection,
        &preparation,
        selection_shards[0],
    )
    .expect("load reconstructed selection shard");
    assert_eq!(
        reconstructed_shard.artifact_sha256,
        expected_selection_shard_sha256
    );

    let selection_report = selected
        .selection_report_xlsx_path
        .as_ref()
        .expect("initial selection report");
    let original_selection_report_bytes =
        fs::read(selection_report).expect("read initial selection report bytes");
    let selection_receipt_path = campaign_dir.join(SELECTION_REPORT_RECEIPT_FILE_NAME);
    let original_selection_receipt_bytes =
        fs::read(&selection_receipt_path).expect("read initial selection report receipt");
    remove_file(selection_report, "selection report");
    assert!(selection_receipt_path.exists());
    remove_file(
        pre_final_reduction_path(campaign_dir),
        "pre-final reduction projection",
    );
    let hidden_selection_csv = hide_csv(&fixture.source.selection_csv_path);

    let recovered_selection =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&selected), &fixture.base)
            .expect("recover selection reduction and report from durable winner lock");
    assert!(hidden_selection_csv.exists());
    assert!(!Path::new(&fixture.source.selection_csv_path).exists());
    assert_eq!(
        recovered_selection
            .pre_final_reduction_artifact_sha256
            .as_deref(),
        Some(original_pre_final_sha256.as_str())
    );
    assert_eq!(
        recovered_selection.winner_lock_sha256.as_deref(),
        Some(original_winner_lock.as_str())
    );
    assert_xlsx(
        recovered_selection.selection_report_xlsx_path.as_ref(),
        "recovered selection report",
    );
    assert_eq!(
        fs::read(
            recovered_selection
                .selection_report_xlsx_path
                .as_ref()
                .expect("recovered selection report path")
        )
        .expect("read recovered selection report"),
        original_selection_report_bytes
    );
    assert_eq!(
        fs::read(&selection_receipt_path).expect("read retained selection receipt"),
        original_selection_receipt_bytes
    );

    let completed = open_vcet_temporal_campaign_final_test_blocking(
        final_request(&recovered_selection, original_winner_lock.clone(), true),
        &fixture.base,
    )
    .expect("run initial final-test stage");
    let original_final_sha256 = completed
        .final_reduction_artifact_sha256
        .clone()
        .expect("initial final reduction SHA-256");
    let final_report = completed
        .final_report_xlsx_path
        .as_ref()
        .expect("initial final report");
    let original_final_report_bytes =
        fs::read(final_report).expect("read initial final report bytes");
    let final_receipt_path = campaign_dir.join(FINAL_REPORT_RECEIPT_FILE_NAME);
    let original_final_receipt_bytes =
        fs::read(&final_receipt_path).expect("read initial final report receipt");
    fs::write(final_report, b"corrupt-retained-final-xlsx")
        .expect("corrupt retained final report before reduction recovery");
    assert!(final_receipt_path.exists());
    remove_file(
        final_reduction_path(campaign_dir),
        "final reduction projection",
    );
    let preparation = load_preparation(
        Path::new(&completed.preparation_artifact_path),
        &completed.preparation_artifact_sha256,
    )
    .expect("load preparation for final-shard projection path");
    let final_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    );
    assert_eq!(final_shards.len(), 1);
    remove_file(
        final_shard_path(campaign_dir, final_shards[0].shard_ordinal),
        "final shard projection",
    );
    let hidden_final_csv = hide_csv(&fixture.source.final_test_csv_path);

    let recovered_final = open_vcet_temporal_campaign_final_test_blocking(
        final_request(&completed, original_winner_lock.clone(), true),
        &fixture.base,
    )
    .expect("recover final shard, reduction, and report from durable terminal");
    assert!(hidden_final_csv.exists());
    assert!(!Path::new(&fixture.source.final_test_csv_path).exists());
    assert_eq!(
        recovered_final.final_reduction_artifact_sha256.as_deref(),
        Some(original_final_sha256.as_str())
    );
    assert_eq!(
        recovered_final
            .pre_final_reduction_artifact_sha256
            .as_deref(),
        Some(original_pre_final_sha256.as_str())
    );
    assert_eq!(
        recovered_final.winner_lock_sha256.as_deref(),
        Some(original_winner_lock.as_str())
    );
    assert_eq!(
        recovered_final.state,
        VcetTemporalCampaignStateV1::FinalTestComplete
    );
    assert_eq!(recovered_final.next_required_action, "none");
    assert_xlsx(
        recovered_final.final_report_xlsx_path.as_ref(),
        "recovered final report",
    );
    assert_eq!(
        fs::read(
            recovered_final
                .final_report_xlsx_path
                .as_ref()
                .expect("recovered final report path")
        )
        .expect("read recovered final report"),
        original_final_report_bytes
    );
    assert_eq!(
        fs::read(&final_receipt_path).expect("read retained final receipt"),
        original_final_receipt_bytes
    );
    assert_eq!(
        serde_json::to_value(load_status(&recovered_final, &fixture.base))
            .expect("encode recovered final status"),
        serde_json::to_value(&recovered_final).expect("encode recovery response")
    );
}

#[test]
fn retained_final_receipt_without_complete_durable_cohort_blocks_before_data_loading() {
    let fixture = LifecycleFixture::new("vcettemporalretainedfinalreceiptnodurablecohort");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run selection before forging retained final receipt");
    let preparation = load_preparation(
        Path::new(&selected.preparation_artifact_path),
        &selected.preparation_artifact_sha256,
    )
    .expect("load preparation for retained final receipt");
    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let retained_report_bytes = fs::read(
        selected
            .selection_report_xlsx_path
            .as_ref()
            .expect("selection report fixture"),
    )
    .expect("read XLSX bytes for retained final report fixture");
    let retained_report_path = commands_reports::persist_vcet_temporal_report_bytes_for_test(
        "final",
        Some("retained without durable cohort"),
        &preparation.payload.sealed_universe.universe_sha256,
        &fixture.base,
        &retained_report_bytes,
    )
    .expect("persist retained final report fixture");
    let payload = VcetTemporalCampaignReportReceiptPayloadV1 {
        schema_version: VCET_TEMPORAL_PRODUCT_CAMPAIGN_SCHEMA_VERSION,
        campaign_id: preparation.payload.campaign_id.clone(),
        stage: "final".to_string(),
        report_sha256: file_sha256(&retained_report_path, MAX_PRODUCT_ARTIFACT_READ_BYTES)
            .expect("hash retained final report fixture"),
        report_path: retained_report_path.to_string_lossy().to_string(),
        report_bytes: retained_report_bytes.len(),
        source_artifact_sha256: format!("sha256:{}", "e".repeat(64)),
    };
    let receipt = VcetTemporalCampaignReportReceiptV1 {
        artifact_sha256: report_receipt_sha256(&payload).expect("hash retained final receipt"),
        payload,
    };
    let receipt_path = campaign_dir.join(FINAL_REPORT_RECEIPT_FILE_NAME);
    let receipt_bytes = serde_json::to_vec_pretty(&receipt).expect("encode retained final receipt");
    fs::write(&receipt_path, &receipt_bytes).expect("write retained final receipt");

    let final_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    );
    assert_eq!(final_shards.len(), 1);
    assert!(!final_reduction_path(campaign_dir).exists());
    assert!(!final_shard_path(campaign_dir, final_shards[0].shard_ordinal).exists());
    let hidden_final_csv = hide_csv(&fixture.source.final_test_csv_path);

    let blocked = open_vcet_temporal_campaign_final_test_blocking(
        final_request(
            &selected,
            selected
                .winner_lock_sha256
                .clone()
                .expect("selected winner lock"),
            true,
        ),
        &fixture.base,
    )
    .expect("retained final receipt must block before final data loading");
    assert!(hidden_final_csv.exists());
    assert!(!Path::new(&fixture.source.final_test_csv_path).exists());
    assert!(blocked.execution_blocked);
    assert_eq!(blocked.blocked_stage.as_deref(), Some("final_report"));
    assert_eq!(
        blocked.blocked_reason_code.as_deref(),
        Some(REPORT_RECEIPT_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(blocked.next_required_action, BLOCKED_CAMPAIGN_ACTION);
    assert_eq!(
        blocked.state,
        VcetTemporalCampaignStateV1::SelectionComplete
    );
    assert_eq!(blocked.completed_final_test_shard_count, 0);
    assert!(blocked.pre_final_reduction_artifact_path.is_some());
    assert!(blocked.selection_report_xlsx_path.is_some());
    assert!(blocked.final_reduction_artifact_path.is_none());
    assert!(blocked.final_report_xlsx_path.is_none());
    assert!(!final_reduction_path(campaign_dir).exists());
    assert!(!final_shard_path(campaign_dir, final_shards[0].shard_ordinal).exists());
    assert_eq!(
        fs::read(&receipt_path).expect("read unchanged retained final receipt"),
        receipt_bytes
    );
    assert_eq!(
        fs::read(&retained_report_path).expect("read unchanged retained final report"),
        retained_report_bytes
    );
}

#[test]
fn durable_pre_final_with_zero_selection_projections_resumes_selection() {
    let fixture = LifecycleFixture::new("vcettemporalzeroselectionprojection");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run initial CPU selection");
    let preparation = load_preparation(
        Path::new(&selected.preparation_artifact_path),
        &selected.preparation_artifact_sha256,
    )
    .expect("load preparation for zero-projection recovery");
    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let selection_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::Selection,
    );
    assert!(!selection_shards.is_empty());
    for shard in &selection_shards {
        remove_file(
            pre_final_shard_path(campaign_dir, shard.shard_ordinal),
            "selection shard projection",
        );
    }

    let status = load_status(&selected, &fixture.base);
    assert_eq!(status.completed_selection_shard_count, 0);
    assert_eq!(status.selection_shard_count, selection_shards.len());
    assert_eq!(
        status.pre_final_reduction_artifact_sha256,
        selected.pre_final_reduction_artifact_sha256
    );
    assert!(!status.execution_blocked);
    assert_eq!(status.next_required_action, "resume_selection");
}

#[test]
fn campaign_status_reports_live_pre_final_as_retryable_then_blocks_after_owner_drop() {
    let fixture = LifecycleFixture::new("vcettemporalliveprefinalstatus");
    let prepared = fixture.prepare();
    let preparation = load_preparation(
        Path::new(&prepared.preparation_artifact_path),
        &prepared.preparation_artifact_sha256,
    )
    .expect("load preparation for live pre-final status");
    let request_sha256 = format!("sha256:{}", "a".repeat(64));
    let claim =
        match admit_durable_pre_final_winner_lock::<ReduceVcetTemporalPreFinalShardsResponseV1>(
            &fixture.base,
            &preparation.payload.sealed_universe.universe_sha256,
            &request_sha256,
        )
        .expect("acquire live pre-final claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("fresh pre-final claim cannot replay"),
        };

    let live_error =
        get_vcet_temporal_campaign_status_blocking(status_request(&prepared), &fixture.base)
            .expect_err("status must treat a live pre-final claim as retryable");
    assert!(
        live_error.contains("temporarily unavailable"),
        "{live_error}"
    );
    assert!(live_error.contains("retry"), "{live_error}");
    assert!(!live_error.contains("interrupted"), "{live_error}");

    drop(claim);
    let blocked =
        get_vcet_temporal_campaign_status_blocking(status_request(&prepared), &fixture.base)
            .expect("status after dropped pre-final claim");
    assert!(blocked.execution_blocked);
    assert_eq!(
        blocked.blocked_stage.as_deref(),
        Some("pre_final_reduction")
    );
    assert_eq!(
        blocked.blocked_reason_code.as_deref(),
        Some(DURABLE_STAGE_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(blocked.next_required_action, BLOCKED_CAMPAIGN_ACTION);
}

#[test]
fn campaign_status_reports_live_final_as_retryable_then_blocks_after_owner_drop() {
    let fixture = LifecycleFixture::new("vcettemporallivefinalstatus");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run selection before live final status");
    let preparation = load_preparation(
        Path::new(&selected.preparation_artifact_path),
        &selected.preparation_artifact_sha256,
    )
    .expect("load preparation for live final status");
    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let pre_final =
        load_pre_final_reduction_artifact(&pre_final_reduction_path(campaign_dir), &preparation)
            .expect("load winner lock before live final status");
    let final_shard = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    )
    .into_iter()
    .next()
    .expect("one final shard");
    let authorization = pre_final
        .payload
        .final_shard_authorizations
        .iter()
        .find(|authorization| authorization.shard.shard_id == final_shard.shard_id)
        .expect("final shard authorization");
    let request_sha256 = format!("sha256:{}", "b".repeat(64));
    let claim =
        match admit_durable_final_authorization::<ExecuteVcetTemporalFinalTestShardResponseV1>(
            &fixture.base,
            &authorization.authorization_sha256,
            &request_sha256,
        )
        .expect("acquire live final claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("fresh final claim cannot replay"),
        };

    let live_error =
        get_vcet_temporal_campaign_status_blocking(status_request(&selected), &fixture.base)
            .expect_err("status must treat a live final claim as retryable");
    assert!(
        live_error.contains("temporarily unavailable"),
        "{live_error}"
    );
    assert!(live_error.contains("retry"), "{live_error}");
    assert!(!live_error.contains("interrupted"), "{live_error}");

    drop(claim);
    let blocked =
        get_vcet_temporal_campaign_status_blocking(status_request(&selected), &fixture.base)
            .expect("status after dropped final claim");
    assert!(blocked.execution_blocked);
    assert_eq!(blocked.blocked_stage.as_deref(), Some("final_test_shard"));
    assert_eq!(
        blocked.blocked_shard_ordinal,
        Some(final_shard.shard_ordinal)
    );
    assert_eq!(
        blocked.blocked_reason_code.as_deref(),
        Some(DURABLE_STAGE_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(blocked.next_required_action, BLOCKED_CAMPAIGN_ACTION);
}

#[test]
fn status_probes_later_durable_terminals_before_classifying_projection_corruption() {
    let fixture = LifecycleFixture::new("vcettemporalstatusprojectioncorruption");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run CPU selection");
    let completed = open_vcet_temporal_campaign_final_test_blocking(
        final_request(
            &selected,
            selected
                .winner_lock_sha256
                .clone()
                .expect("selected winner lock"),
            true,
        ),
        &fixture.base,
    )
    .expect("run CPU final test");
    let preparation = load_preparation(
        Path::new(&completed.preparation_artifact_path),
        &completed.preparation_artifact_sha256,
    )
    .expect("load preparation for projection corruption status");
    let campaign_dir = Path::new(&completed.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let pre_final_path = pre_final_reduction_path(campaign_dir);
    let original_pre_final = fs::read(&pre_final_path).expect("read pre-final projection");
    let final_shards = universe_shards_for_role(
        &preparation.payload.sealed_universe.payload,
        VcetTemporalDatasetCohortRoleV1::FinalTest,
    );
    assert_eq!(final_shards.len(), 1);
    let final_path = final_shard_path(campaign_dir, final_shards[0].shard_ordinal);
    let original_final = fs::read(&final_path).expect("read final shard projection");

    fs::write(&pre_final_path, b"{malformed")
        .expect("replace pre-final projection with malformed bytes");
    remove_file(&final_path, "final shard projection");
    let pre_final_blocked = load_status(&completed, &fixture.base);
    assert!(pre_final_blocked.execution_blocked);
    assert_eq!(
        pre_final_blocked.blocked_stage.as_deref(),
        Some("pre_final_reduction")
    );
    assert_eq!(
        pre_final_blocked.blocked_reason_code.as_deref(),
        Some(DURABLE_STAGE_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(
        pre_final_blocked.next_required_action,
        BLOCKED_CAMPAIGN_ACTION
    );
    assert!(
        final_path.exists(),
        "later durable final terminal was reconciled"
    );
    let durable_pre_final =
        match probe_pre_final_reduction_from_durable_terminal(&fixture.base, &preparation)
            .expect("probe durable pre-final authority")
        {
            DurableCompletionProbe::Completed { response, .. } => response,
            other => panic!("expected completed durable pre-final authority, got {other:?}"),
        };
    load_final_shard_artifact(
        &final_path,
        &preparation,
        final_shards[0],
        &durable_pre_final,
    )
    .expect("recovered final projection remains valid");

    fs::write(&pre_final_path, &original_pre_final).expect("restore pre-final projection");
    fs::write(&final_path, b"{malformed").expect("replace final projection with malformed bytes");
    let final_blocked = load_status(&completed, &fixture.base);
    assert!(final_blocked.execution_blocked);
    assert_eq!(
        final_blocked.blocked_stage.as_deref(),
        Some("final_test_shard")
    );
    assert_eq!(
        final_blocked.blocked_shard_ordinal,
        Some(final_shards[0].shard_ordinal)
    );
    assert_eq!(
        fs::read(&final_path).expect("read retained invalid final projection"),
        b"{malformed",
        "status must not overwrite an invalid projection"
    );

    fs::write(&final_path, original_final).expect("restore final shard projection");
}

#[test]
fn malformed_and_foreign_report_receipts_return_path_safe_stable_blocks() {
    let fixture = LifecycleFixture::new("vcettemporalreportreceiptblocks");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run CPU selection");
    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let selection_receipt_path = campaign_dir.join(SELECTION_REPORT_RECEIPT_FILE_NAME);
    let original_selection_receipt =
        fs::read(&selection_receipt_path).expect("read selection receipt");
    fs::write(&selection_receipt_path, b"{malformed").expect("write malformed selection receipt");

    let malformed = load_status(&selected, &fixture.base);
    assert!(malformed.execution_blocked);
    assert_eq!(malformed.blocked_stage.as_deref(), Some("selection_report"));
    assert_eq!(
        malformed.blocked_reason_code.as_deref(),
        Some(REPORT_RECEIPT_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(malformed.next_required_action, BLOCKED_CAMPAIGN_ACTION);
    assert_eq!(malformed.state, VcetTemporalCampaignStateV1::Prepared);
    assert!(malformed.pre_final_reduction_artifact_path.is_some());
    assert!(malformed.selection_report_xlsx_path.is_none());

    fs::write(&selection_receipt_path, original_selection_receipt)
        .expect("restore selection receipt");
    let completed = open_vcet_temporal_campaign_final_test_blocking(
        final_request(
            &selected,
            selected
                .winner_lock_sha256
                .clone()
                .expect("selected winner lock"),
            true,
        ),
        &fixture.base,
    )
    .expect("run final test");
    let final_receipt_path = campaign_dir.join(FINAL_REPORT_RECEIPT_FILE_NAME);
    let mut foreign: VcetTemporalCampaignReportReceiptV1 =
        serde_json::from_slice(&fs::read(&final_receipt_path).expect("read final receipt"))
            .expect("decode final receipt");
    foreign.payload.source_artifact_sha256 = format!("sha256:{}", "f".repeat(64));
    foreign.artifact_sha256 =
        report_receipt_sha256(&foreign.payload).expect("rehash foreign receipt fixture");
    fs::write(
        &final_receipt_path,
        serde_json::to_vec_pretty(&foreign).expect("encode foreign receipt fixture"),
    )
    .expect("write foreign final receipt");

    let blocked = load_status(&completed, &fixture.base);
    assert!(blocked.execution_blocked);
    assert_eq!(blocked.blocked_stage.as_deref(), Some("final_report"));
    assert_eq!(
        blocked.blocked_reason_code.as_deref(),
        Some(REPORT_RECEIPT_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(
        blocked.state,
        VcetTemporalCampaignStateV1::SelectionComplete
    );
    assert!(blocked.selection_report_xlsx_path.is_some());
    assert!(blocked.final_reduction_artifact_path.is_some());
    assert!(blocked.final_report_xlsx_path.is_none());
}

#[test]
fn status_deterministically_repairs_corrupt_and_missing_report_xlsx() {
    let fixture = LifecycleFixture::new("vcettemporalreportrepair");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run CPU selection");
    let report_path = PathBuf::from(
        selected
            .selection_report_xlsx_path
            .as_ref()
            .expect("selection report path"),
    );
    let original = fs::read(&report_path).expect("read original selection report");

    fs::write(&report_path, b"corrupt-xlsx").expect("corrupt selection report");
    let repaired_corrupt = load_status(&selected, &fixture.base);
    assert!(!repaired_corrupt.execution_blocked);
    assert_eq!(
        fs::read(&report_path).expect("read repaired corrupt report"),
        original
    );
    assert_eq!(
        repaired_corrupt.selection_report_xlsx_path.as_deref(),
        selected.selection_report_xlsx_path.as_deref()
    );

    remove_file(&report_path, "repairable selection report");
    let repaired_missing = load_status(&selected, &fixture.base);
    assert!(!repaired_missing.execution_blocked);
    assert_eq!(
        fs::read(&report_path).expect("read repaired missing report"),
        original
    );
    assert!(repaired_missing.selection_report_xlsx_path.is_some());

    let oversized = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&report_path)
        .expect("open selection report for oversized corruption");
    oversized
        .set_len(MAX_PRODUCT_ARTIFACT_READ_BYTES.saturating_add(1))
        .expect("expand selection report beyond the bounded read limit");
    drop(oversized);
    let repaired_oversized = load_status(&selected, &fixture.base);
    assert!(!repaired_oversized.execution_blocked);
    assert_eq!(
        fs::read(&report_path).expect("read repaired oversized report"),
        original
    );

    let completed = open_vcet_temporal_campaign_final_test_blocking(
        final_request(
            &repaired_oversized,
            repaired_oversized
                .winner_lock_sha256
                .clone()
                .expect("selected winner lock"),
            true,
        ),
        &fixture.base,
    )
    .expect("run final test for report status repair");
    let final_report_path = PathBuf::from(
        completed
            .final_report_xlsx_path
            .as_ref()
            .expect("final report path"),
    );
    let original_final = fs::read(&final_report_path).expect("read original final report");

    fs::write(&final_report_path, b"corrupt-final-xlsx").expect("corrupt final report");
    let repaired_final_corrupt = load_status(&completed, &fixture.base);
    assert!(!repaired_final_corrupt.execution_blocked);
    assert_eq!(
        repaired_final_corrupt.state,
        VcetTemporalCampaignStateV1::FinalTestComplete
    );
    assert_eq!(
        fs::read(&final_report_path).expect("read repaired corrupt final report"),
        original_final
    );

    remove_file(&final_report_path, "repairable final report");
    let repaired_final_missing = load_status(&completed, &fixture.base);
    assert!(!repaired_final_missing.execution_blocked);
    assert_eq!(
        repaired_final_missing.state,
        VcetTemporalCampaignStateV1::FinalTestComplete
    );
    assert_eq!(
        fs::read(&final_report_path).expect("read repaired missing final report"),
        original_final
    );
    assert!(repaired_final_missing.final_report_xlsx_path.is_some());
}

#[test]
fn nondeterministic_report_receipt_blocks_instead_of_rewriting_evidence() {
    let fixture = LifecycleFixture::new("vcettemporalreportnondeterministic");
    let prepared = fixture.prepare();
    let selected =
        run_vcet_temporal_campaign_selection_blocking(selection_request(&prepared), &fixture.base)
            .expect("run CPU selection");
    let campaign_dir = Path::new(&selected.preparation_artifact_path)
        .parent()
        .expect("campaign directory");
    let receipt_path = campaign_dir.join(SELECTION_REPORT_RECEIPT_FILE_NAME);
    let mut receipt: VcetTemporalCampaignReportReceiptV1 =
        serde_json::from_slice(&fs::read(&receipt_path).expect("read selection receipt"))
            .expect("decode selection receipt");
    let report_path = PathBuf::from(&receipt.payload.report_path);
    let original_report = fs::read(&report_path).expect("read original selection report");
    receipt.payload.report_bytes = receipt.payload.report_bytes.saturating_add(1);
    receipt.artifact_sha256 =
        report_receipt_sha256(&receipt.payload).expect("rehash altered receipt fixture");
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("encode altered receipt fixture"),
    )
    .expect("write altered receipt fixture");

    let blocked = load_status(&selected, &fixture.base);
    assert!(blocked.execution_blocked);
    assert_eq!(blocked.blocked_stage.as_deref(), Some("selection_report"));
    assert_eq!(
        blocked.blocked_reason_code.as_deref(),
        Some(REPORT_XLSX_NOT_RECOVERABLE_REASON)
    );
    assert_eq!(blocked.next_required_action, BLOCKED_CAMPAIGN_ACTION);
    assert!(blocked.selection_report_xlsx_path.is_none());
    assert_eq!(
        fs::read(report_path).expect("read unchanged selection report"),
        original_report,
        "a receipt that cannot be deterministically regenerated must not rewrite report evidence"
    );
}
