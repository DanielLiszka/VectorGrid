// Add this function to crates/app_core/build.rs and call it from main().
// It intentionally avoids any new Cargo dependencies.

fn build_rtcore_barrier_if_requested() {
    use std::env;
    use std::path::PathBuf;
    use std::process::Command;

    println!("cargo:rustc-check-cfg=cfg(rtcore_barrier_compiled)");
    println!("cargo:rerun-if-env-changed=OPTIX_HOME");
    println!("cargo:rerun-if-env-changed=VECTORBT_ENABLE_RTCORE_BARRIER");
    println!("cargo:rerun-if-changed=native/rtcore_barrier/CMakeLists.txt");
    println!("cargo:rerun-if-changed=native/rtcore_barrier/rtcore_barrier_api.h");
    println!("cargo:rerun-if-changed=native/rtcore_barrier/rtcore_barrier_device.cu");
    println!("cargo:rerun-if-changed=native/rtcore_barrier/rtcore_barrier_host.cpp");

    let enabled = env::var_os("VECTORBT_ENABLE_RTCORE_BARRIER").is_some()
        || env::var_os("OPTIX_HOME").is_some();
    if !enabled {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let optix_home = env::var_os("OPTIX_HOME").map(PathBuf::from).unwrap_or_else(|| {
        panic!("RT-core barrier build requested but OPTIX_HOME is not set");
    });
    let build_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("rtcore_barrier_build");
    std::fs::create_dir_all(&build_dir).expect("create rtcore build dir");

    let source_dir = manifest_dir.join("native/rtcore_barrier");

    let configure_status = Command::new("cmake")
        .arg("-S")
        .arg(&source_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DOPTIX_HOME={}", optix_home.display()))
        .status()
        .expect("failed to spawn cmake configure for rtcore_barrier");
    if !configure_status.success() {
        panic!("cmake configure failed for rtcore_barrier");
    }

    let build_status = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .arg("--config")
        .arg("Release")
        .status()
        .expect("failed to spawn cmake build for rtcore_barrier");
    if !build_status.success() {
        panic!("cmake build failed for rtcore_barrier");
    }

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=dylib=rtcore_barrier");
    println!("cargo:rustc-cfg=rtcore_barrier_compiled");
}

// Then invoke:
// fn main() {
//     ...existing build logic...
//     build_rtcore_barrier_if_requested();
// }
