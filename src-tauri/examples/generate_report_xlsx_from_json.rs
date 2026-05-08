use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use ta_desktop_demo_app::commands::build_report_xlsx_bytes_headless;

fn usage() -> String {
    "usage: cargo run -p ta_desktop_demo_app --example generate_report_xlsx_from_json -- <report.json> [out.xlsx]".to_string()
}

fn default_output_path(input: &Path) -> PathBuf {
    let mut out = input.to_path_buf();
    out.set_extension("xlsx");
    out
}

fn reject_same_input_output(input: &Path, output: &Path) -> Result<(), String> {
    if input == output {
        return Err("output path must differ from input report path".to_string());
    }
    if output.exists() {
        let input = input
            .canonicalize()
            .map_err(|err| format!("failed to resolve {}: {err}", input.display()))?;
        let output = output
            .canonicalize()
            .map_err(|err| format!("failed to resolve {}: {err}", output.display()))?;
        if input == output {
            return Err("output path resolves to the input report path".to_string());
        }
    }
    Ok(())
}

fn run() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    let input = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&input));
    if args.next().is_some() {
        return Err(usage());
    }
    reject_same_input_output(&input, &output)?;

    let text = fs::read_to_string(&input)
        .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
    let report: Value = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse {}: {err}", input.display()))?;
    let bytes = build_report_xlsx_bytes_headless(&report, Some(25))
        .map_err(|err| format!("failed to build xlsx from {}: {err}", input.display()))?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(&output, bytes)
        .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
    Ok(output)
}

fn main() {
    match run() {
        Ok(output) => println!("{}", output.display()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
