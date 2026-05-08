use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize)]
pub struct SaveSimpleSummaryMarkdownResponse {
    pub path: String,
    pub bytes: usize,
}

fn sanitize_markdown_default_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => out.push(ch),
            _ => out.push('_'),
        }
    }
    let trimmed = out.trim_matches('_').chars().take(96).collect::<String>();
    if trimmed.is_empty() {
        "vectorgrid-simple-summary.md".to_string()
    } else if {
        let lower = trimmed.to_ascii_lowercase();
        lower.ends_with(".md") || lower.ends_with(".markdown")
    } {
        trimmed
    } else {
        format!("{trimmed}.md")
    }
}

fn normalize_save_markdown_path(path: PathBuf) -> Result<PathBuf, String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") => {
            Ok(path)
        }
        Some(_) => Err(format!(
            "save path must use .md or .markdown extension: {}",
            path.to_string_lossy()
        )),
        None => {
            let mut with_ext = path;
            with_ext.set_extension("md");
            Ok(with_ext)
        }
    }
}

fn write_simple_summary_markdown(
    path: &Path,
    markdown: &str,
) -> Result<SaveSimpleSummaryMarkdownResponse, String> {
    let content = markdown.trim_end();
    if content.trim().is_empty() {
        return Err("summary Markdown must not be empty".to_string());
    }
    let mut file = std::fs::File::create(path).map_err(|e| {
        format!(
            "failed to write Simple summary at {}: {}",
            path.to_string_lossy(),
            e
        )
    })?;
    file.write_all(content.as_bytes()).map_err(|e| {
        format!(
            "failed to write Simple summary at {}: {}",
            path.to_string_lossy(),
            e
        )
    })?;
    file.write_all(b"\n").map_err(|e| {
        format!(
            "failed to write Simple summary at {}: {}",
            path.to_string_lossy(),
            e
        )
    })?;
    Ok(SaveSimpleSummaryMarkdownResponse {
        path: path.to_string_lossy().to_string(),
        bytes: content.len().saturating_add(1),
    })
}

#[tauri::command]
pub fn save_simple_summary_markdown(
    markdown: String,
    default_name: Option<String>,
    app: tauri::AppHandle,
) -> Result<Option<SaveSimpleSummaryMarkdownResponse>, String> {
    let suggested_name = sanitize_markdown_default_name(
        default_name
            .as_deref()
            .unwrap_or("vectorgrid-simple-summary.md"),
    );

    let Some(path) = app
        .dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .set_file_name(&suggested_name)
        .blocking_save_file()
    else {
        return Ok(None);
    };

    let path = path
        .into_path()
        .map_err(|e| e.to_string())
        .and_then(normalize_save_markdown_path)?;
    write_simple_summary_markdown(&path, &markdown).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("{prefix}_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn markdown_default_name_is_safe_and_gets_extension() {
        assert_eq!(
            sanitize_markdown_default_name(" VectorGrid Summary 2026/04/29 "),
            "VectorGrid_Summary_2026_04_29.md"
        );
        assert_eq!(
            sanitize_markdown_default_name("summary.markdown"),
            "summary.markdown"
        );
        assert_eq!(sanitize_markdown_default_name("SUMMARY.MD"), "SUMMARY.MD");
        assert_eq!(
            sanitize_markdown_default_name("   "),
            "vectorgrid-simple-summary.md"
        );
    }

    #[test]
    fn normalize_save_markdown_path_accepts_markdown_extensions_only() {
        assert_eq!(
            normalize_save_markdown_path(PathBuf::from("C:/out/summary"))
                .expect("missing extension becomes md"),
            PathBuf::from("C:/out/summary.md")
        );
        assert!(normalize_save_markdown_path(PathBuf::from("C:/out/summary.markdown")).is_ok());
        let err = normalize_save_markdown_path(PathBuf::from("C:/out/summary.txt"))
            .expect_err("txt should be rejected");
        assert!(err.contains(".md or .markdown"));
    }

    #[test]
    fn write_simple_summary_markdown_writes_final_newline_and_reports_bytes() {
        let dir = make_temp_dir("simple_summary_markdown");
        let path = dir.join("summary.md");
        let saved =
            write_simple_summary_markdown(&path, "# Summary\n\nBody").expect("write summary");
        let raw = std::fs::read_to_string(&path).expect("read written summary");
        assert_eq!(raw, "# Summary\n\nBody\n");
        assert_eq!(saved.bytes, raw.len());
        assert_eq!(saved.path, path.to_string_lossy().to_string());
    }

    #[test]
    fn write_simple_summary_markdown_rejects_empty_content() {
        let dir = make_temp_dir("simple_summary_markdown_empty");
        let err = write_simple_summary_markdown(&dir.join("summary.md"), "  \n  ")
            .expect_err("empty summary should fail");
        assert_eq!(err, "summary Markdown must not be empty");
    }
}
