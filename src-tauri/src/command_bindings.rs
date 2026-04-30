use std::path::{Path, PathBuf};

use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri_specta::{collect_commands, Builder};

use crate::commands::{GpuSpeedupEntry, GpuStatus, RepoDataCsvList, SampleCsv};

#[tauri::command]
#[specta::specta]
async fn get_gpu_status() -> Result<GpuStatus, String> {
    crate::commands::get_gpu_status().await
}

#[tauri::command]
#[specta::specta]
async fn list_gpu_speedups() -> Result<Vec<GpuSpeedupEntry>, String> {
    crate::commands::list_gpu_speedups().await
}

#[tauri::command]
#[specta::specta]
fn list_sample_csvs() -> Vec<SampleCsv> {
    crate::commands::list_sample_csvs_for_bindings()
}

#[tauri::command]
#[specta::specta]
async fn list_repo_data_csvs(
    max_files: Option<usize>,
    recursive: Option<bool>,
    include_identity: Option<bool>,
) -> Result<RepoDataCsvList, String> {
    crate::commands::list_repo_data_csvs(max_files, recursive, include_identity).await
}

pub fn command_bindings_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new().commands(collect_commands![
        get_gpu_status,
        list_gpu_speedups,
        list_sample_csvs,
        list_repo_data_csvs,
    ])
}

pub fn default_bindings_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("web")
        .join("src")
        .join("tauri")
        .join("generatedBindings.ts")
}

pub fn export_bindings(path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    command_bindings_builder::<tauri::Wry>().export(
        Typescript::default().bigint(BigIntExportBehavior::Number),
        path,
    )?;
    let generated = std::fs::read_to_string(path)?;
    std::fs::write(
        path,
        format!("// @ts-nocheck\n/* eslint-disable */\n{generated}"),
    )?;
    Ok(())
}

pub fn export_bindings_to_default_web_path() -> Result<(), Box<dyn std::error::Error>> {
    export_bindings(default_bindings_path())
}
