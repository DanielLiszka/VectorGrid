fn main() {
    tauri_build::build();
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(target_os = "windows")]
    {
        // Unit test harness executables can miss the Common Controls v6 context,
        // which makes comctl32 imports (e.g. TaskDialogIndirect) fail at load.
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
