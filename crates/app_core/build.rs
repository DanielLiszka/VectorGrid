use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/kernels/double_crossover.cu");
    println!("cargo:rerun-if-changed=src/kernels/double_crossover.prebuilt.ptx");
    println!("cargo:rerun-if-env-changed=CUDA_ARCH");
    println!("cargo:rerun-if-env-changed=NVCC");
    println!("cargo:rerun-if-env-changed=NVCC_ARGS");
    println!("cargo:rerun-if-env-changed=VECTORBT_USE_PREBUILT_PTX");
    println!("cargo:rerun-if-env-changed=VECTORBT_REQUIRE_NVCC");

    if env::var("CARGO_FEATURE_CUDA_BACKTEST_KERNEL").is_ok() {
        build_double_crossover_artifacts();
    }
}

fn build_double_crossover_artifacts() {
    let arch = resolve_cuda_arch();
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let kernel = Path::new("src/kernels/double_crossover.cu");
    let prebuilt = Path::new("src/kernels/double_crossover.prebuilt.ptx");
    let ptx_out = out_dir.join("double_crossover.ptx");
    let cubin_out = out_dir.join("double_crossover.sm_89.cubin");

    let use_prebuilt = matches!(env::var("VECTORBT_USE_PREBUILT_PTX"), Ok(ref v) if v == "1" || v.eq_ignore_ascii_case("true"));
    let require_nvcc = matches!(env::var("VECTORBT_REQUIRE_NVCC"), Ok(ref v) if v == "1" || v.eq_ignore_ascii_case("true"));

    if use_prebuilt {
        if !prebuilt.exists() {
            panic!(
                "VECTORBT_USE_PREBUILT_PTX requested, but prebuilt PTX is missing: {}",
                prebuilt.display()
            );
        }
        std::fs::copy(prebuilt, &ptx_out)
            .unwrap_or_else(|e| panic!("failed to copy prebuilt PTX to OUT_DIR: {e}"));
        eprintln!(
            "[ta_app_core] Using prebuilt PTX: {} -> {}",
            prebuilt.display(),
            ptx_out.display()
        );
        write_empty_file(&cubin_out);
        return;
    }

    let Some(nvcc) = find_nvcc() else {
        if require_nvcc {
            panic!("NVCC not found (VECTORBT_REQUIRE_NVCC=1)");
        }
        if prebuilt.exists() {
            std::fs::copy(prebuilt, &ptx_out)
                .unwrap_or_else(|e| panic!("failed to copy prebuilt PTX to OUT_DIR: {e}"));
            eprintln!(
                "[ta_app_core] NVCC not found; using prebuilt PTX: {} -> {}",
                prebuilt.display(),
                ptx_out.display()
            );
            write_empty_file(&cubin_out);
            return;
        }
        panic!(
            "NVCC not found and no prebuilt PTX available at {} (set NVCC/CUDA_PATH or provide a prebuilt PTX)",
            prebuilt.display()
        );
    };

    let mut out = build_nvcc_output(&nvcc, kernel, &ptx_out, NvccArtifact::Ptx, arch.as_str());
    if !out.status.success() {
        let out_s = String::from_utf8_lossy(&out.stdout);
        let err_s = String::from_utf8_lossy(&out.stderr);
        let maybe_arch_fail = err_s.contains("unsupported gpu architecture")
            || err_s.contains("Value 'compute_")
            || out_s.contains("unsupported gpu architecture");

        if arch != "compute_80" && maybe_arch_fail {
            eprintln!(
                "Falling back to -arch=compute_80 (nvcc doesn't support {})",
                arch
            );
            out = build_nvcc_output(
                &nvcc,
                kernel,
                &ptx_out,
                NvccArtifact::Ptx,
                "compute_80",
            );
        }
    }

    if !out.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&out.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&out.stderr));
        if !require_nvcc && prebuilt.exists() {
            std::fs::copy(prebuilt, &ptx_out)
                .unwrap_or_else(|e| panic!("failed to copy prebuilt PTX to OUT_DIR: {e}"));
            eprintln!(
                "[ta_app_core] NVCC failed; using prebuilt PTX: {} -> {}",
                prebuilt.display(),
                ptx_out.display()
            );
            write_empty_file(&cubin_out);
            return;
        }
        if cfg!(target_os = "windows")
            && String::from_utf8_lossy(&out.stdout).contains("Cannot find compiler 'cl.exe'")
        {
            eprintln!("\n=== CUDA Build Error: Missing Visual Studio C++ Compiler ===");
            eprintln!("nvcc requires Microsoft Visual C++ (cl.exe).");
            eprintln!(
                "Run from a 'Developer PowerShell for VS' / 'x64 Native Tools Command Prompt'"
            );
            eprintln!("or install VS Build Tools 2022 with the Desktop C++ workload.");
            eprintln!("===========================================================\n");
        }
        panic!("nvcc failed");
    }

    build_sm89_cubin_best_effort(&nvcc, kernel, &cubin_out);
}

#[derive(Clone, Copy)]
enum NvccArtifact {
    Ptx,
    Cubin,
}

fn build_nvcc_output(
    nvcc: &Path,
    kernel: &Path,
    out_path: &Path,
    artifact: NvccArtifact,
    arch: &str,
) -> std::process::Output {
    let mut cmd = Command::new(nvcc);
    configure_nvcc_command(&mut cmd);
    cmd.arg(match artifact {
        NvccArtifact::Ptx => "-ptx",
        NvccArtifact::Cubin => "-cubin",
    })
    .arg("-O3")
    .arg(format!("-arch={}", arch))
    .arg("-o")
    .arg(out_path)
    .arg(kernel);
    eprintln!("[ta_app_core] nvcc: {:?}", cmd);
    cmd.output().unwrap_or_else(|e| {
        panic!(
            "failed to run nvcc for {}: {e}",
            match artifact {
                NvccArtifact::Ptx => "ptx",
                NvccArtifact::Cubin => "cubin",
            }
        )
    })
}

fn configure_nvcc_command(cmd: &mut Command) {
    cmd.arg("-std=c++17")
        .arg("--expt-relaxed-constexpr")
        .arg("--extended-lambda");

    if cfg!(target_os = "windows") {
        cmd.arg("-D_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH");
        cmd.arg("-allow-unsupported-compiler");
        if which::which("cl.exe").is_err() {
            if let Ok(vs_path) = find_vs_installation() {
                cmd.arg("-ccbin").arg(vs_path);
            }
        }
    }

    if let Ok(extra) = env::var("NVCC_ARGS") {
        for tok in extra.split_whitespace() {
            if !tok.is_empty() {
                cmd.arg(tok);
            }
        }
    }
}

fn build_sm89_cubin_best_effort(nvcc: &Path, kernel: &Path, cubin_out: &Path) {
    let out = build_nvcc_output(nvcc, kernel, cubin_out, NvccArtifact::Cubin, "sm_89");
    if out.status.success() {
        return;
    }

    eprintln!(
        "[ta_app_core] sm_89 cubin build failed; runtime will fall back to PTX.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    write_empty_file(cubin_out);
}

fn write_empty_file(path: &Path) {
    std::fs::write(path, []).unwrap_or_else(|e| {
        panic!(
            "failed to write placeholder CUDA artifact '{}': {e}",
            path.display()
        )
    });
}

fn resolve_cuda_arch() -> String {
    fn to_compute(tag: &str) -> String {
        let t = tag.trim().to_lowercase();
        if t.starts_with("compute_") {
            return t;
        }
        if t.starts_with("sm_") {
            return format!("compute_{}", &t[3..]);
        }
        let digits = if t.contains('.') {
            t.replace('.', "")
        } else {
            t
        };
        if digits.chars().all(|c| c.is_ascii_digit()) && !digits.is_empty() {
            return format!("compute_{}", digits);
        }
        "compute_80".into()
    }
    to_compute(&env::var("CUDA_ARCH").unwrap_or_else(|_| "compute_89".into()))
}

fn find_nvcc() -> Option<PathBuf> {
    if let Ok(p) = env::var("NVCC") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    if let Ok(cuda) = env::var("CUDA_PATH").or_else(|_| env::var("CUDA_HOME")) {
        let cand = if cfg!(target_os = "windows") {
            Path::new(&cuda).join("bin/nvcc.exe")
        } else {
            Path::new(&cuda).join("bin/nvcc")
        };
        if cand.exists() {
            return Some(cand);
        }
    }
    which::which("nvcc")
        .ok()
        .or_else(|| which::which("nvcc.exe").ok())
}

#[cfg(target_os = "windows")]
fn find_vs_installation() -> Result<String, ()> {
    let vs_paths = [
        "C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2022/Professional/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2022/Enterprise/VC/Tools/MSVC",
        "C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2019/Community/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2019/Professional/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2019/Enterprise/VC/Tools/MSVC",
    ];

    for vs_base in &vs_paths {
        if let Ok(entries) = std::fs::read_dir(vs_base) {
            if let Some(msvc_version) = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| name.starts_with("14."))
                .max()
            {
                let cl_path = format!("{}/{}/bin/Hostx64/x64", vs_base, msvc_version);
                if std::path::Path::new(&format!("{}/cl.exe", cl_path)).exists() {
                    eprintln!("Found cl.exe at: {}", cl_path);
                    return Ok(cl_path);
                }
            }
        }
    }

    Err(())
}

#[cfg(not(target_os = "windows"))]
fn find_vs_installation() -> Result<String, ()> {
    Err(())
}
