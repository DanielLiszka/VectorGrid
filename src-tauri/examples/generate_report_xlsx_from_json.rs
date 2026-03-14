use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use ta_desktop_demo_app::commands::build_report_xlsx_bytes_headless;

fn usage() -> String {
    "usage: cargo run -p ta_desktop_demo_app --example generate_report_xlsx_from_json -- <report.json> [out.xlsx]".to_string()
}

fn default_output_path(input: &PathBuf) -> PathBuf {
    let mut out = input.clone();
    out.set_extension("xlsx");
    out
}

fn main() {
    let mut args = env::args_os().skip(1);
    let input = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{}", usage()));
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&input));

    let text = fs::read_to_string(&input)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", input.display()));
    let report: Value = serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", input.display()));
    let bytes = build_report_xlsx_bytes_headless(&report, Some(25))
        .unwrap_or_else(|err| panic!("failed to build xlsx from {}: {err}", input.display()));

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("failed to create {}: {err}", parent.display()));
    }
    fs::write(&output, bytes)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output.display()));
    println!("{}", output.display());
}
