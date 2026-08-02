use self::super::{apply_parameters, ParameterBundle};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::{env, fs};
use vswhom::VsFindResult;
use winreg;
use winreg::enums::*;

const RESOURCE_COMPILER_OVERRIDE_ENV: &str = "EMBED_RESOURCE_RC_PATH";
const RESOURCE_COMPILER_ATTESTATION_NONCE_ENV: &str = "EMBED_RESOURCE_RC_ATTESTATION_NONCE";
const RESOURCE_COMPILER_ATTESTATION_RUSTC_ENV: &str = "VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceCompiler;

impl ResourceCompiler {
    #[inline(always)]
    pub fn new() -> ResourceCompiler {
        ResourceCompiler
    }

    #[inline(always)]
    pub fn is_supported(&mut self) -> Option<Cow<'static, str>> {
        None
    }

    pub fn compile_resource<Ms: AsRef<OsStr>, Mi: IntoIterator<Item = Ms>, Is: AsRef<OsStr>, Ii: IntoIterator<Item = Is>>(
        &self,
        out_dir: &str,
        prefix: &str,
        resource: &str,
        parameters: ParameterBundle<Ms, Mi, Is, Ii>,
    ) -> Result<String, Cow<'static, str>> {
        let out_file = format!("{}{}{}.lib", out_dir, MAIN_SEPARATOR, prefix);
        let (resource_compiler, attestation_nonce) = resource_compiler_path()?;
        // `.res`es are linkable under MSVC as well as normal libraries.
        if !apply_parameters(
            Command::new(&resource_compiler).args(&["/fo", &out_file, "/I", out_dir]),
            "/D",
            "/I",
            parameters,
        )
        .arg(resource)
        .status()
        .map_err(|_| Cow::from("Are you sure you have RC.EXE in your $PATH?"))?
        .success()
        {
            return Err("RC.EXE failed to compile specified resource file".into());
        }
        if let Some(nonce) = attestation_nonce {
            let value = resource_compiler_attestation_value(&nonce, &resource_compiler)?;
            println!("cargo:rustc-env={RESOURCE_COMPILER_ATTESTATION_RUSTC_ENV}={value}");
        }
        Ok(out_file)
    }
}

fn resource_compiler_path() -> Result<(PathBuf, Option<String>), Cow<'static, str>> {
    println!("cargo:rerun-if-env-changed={RESOURCE_COMPILER_OVERRIDE_ENV}");
    println!("cargo:rerun-if-env-changed={RESOURCE_COMPILER_ATTESTATION_NONCE_ENV}");
    let attestation_nonce = env::var_os(RESOURCE_COMPILER_ATTESTATION_NONCE_ENV)
        .map(validate_resource_compiler_attestation_nonce)
        .transpose()?;
    if let Some(path) = env::var_os(RESOURCE_COMPILER_OVERRIDE_ENV) {
        return Ok((validate_resource_compiler_override(PathBuf::from(path))?, attestation_nonce));
    }
    if attestation_nonce.is_some() {
        return Err(format!("{RESOURCE_COMPILER_ATTESTATION_NONCE_ENV} requires {RESOURCE_COMPILER_OVERRIDE_ENV}").into());
    }
    Ok((find_windows_sdk_tool_impl("rc.exe").unwrap_or_else(|| PathBuf::from("rc.exe")), None))
}

fn validate_resource_compiler_attestation_nonce(nonce: std::ffi::OsString) -> Result<String, Cow<'static, str>> {
    let nonce = nonce
        .into_string()
        .map_err(|_| Cow::Borrowed("EMBED_RESOURCE_RC_ATTESTATION_NONCE must be a 32-character lowercase hexadecimal nonce"))?;
    if nonce.len() != 32 || !nonce.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(format!("{RESOURCE_COMPILER_ATTESTATION_NONCE_ENV} must be a 32-character lowercase hexadecimal nonce").into());
    }
    Ok(nonce)
}

fn resource_compiler_attestation_value(nonce: &str, path: &Path) -> Result<String, Cow<'static, str>> {
    let path = path
        .to_str()
        .ok_or_else(|| Cow::Borrowed("attested resource compiler path must be valid Unicode"))?;
    if path.bytes().any(|byte| matches!(byte, b'|' | b'\r' | b'\n')) {
        return Err("attested resource compiler path is not canonically encodable".into());
    }
    Ok(format!("{nonce}|{path}"))
}

fn validate_resource_compiler_override(path: PathBuf) -> Result<PathBuf, Cow<'static, str>> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(format!("{RESOURCE_COMPILER_OVERRIDE_ENV} must identify an absolute rc.exe path").into());
    }
    if !path.file_name().and_then(OsStr::to_str).is_some_and(|name| name.eq_ignore_ascii_case("rc.exe")) {
        return Err(format!("{RESOURCE_COMPILER_OVERRIDE_ENV} must identify a file named rc.exe").into());
    }
    if path_has_reparse_ancestor(&path).map_err(|error| Cow::Owned(format!("{RESOURCE_COMPILER_OVERRIDE_ENV} reparse-point validation failed: {error}")))? {
        return Err(format!("{RESOURCE_COMPILER_OVERRIDE_ENV} must not use a Windows reparse-point file or ancestor").into());
    }
    let metadata =
        fs::symlink_metadata(&path).map_err(|error| Cow::Owned(format!("{RESOURCE_COMPILER_OVERRIDE_ENV} does not identify an accessible file: {error}")))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("{RESOURCE_COMPILER_OVERRIDE_ENV} must identify an ordinary rc.exe file").into());
    }
    Ok(path)
}

fn path_has_reparse_ancestor(path: &Path) -> std::io::Result<bool> {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

    let mut cursor = Some(path);
    while let Some(candidate) = cursor {
        let metadata = fs::symlink_metadata(candidate)?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Ok(true);
        }
        cursor = candidate.parent();
    }
    Ok(false)
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Arch {
    X86,
    X64,
    AArch64,
}

pub fn find_windows_sdk_tool_impl(tool: &str) -> Option<PathBuf> {
    let arch = match env::var("HOST").expect("No HOST env var").as_bytes() {
        [b'x', b'8', b'6', b'_', b'6', b'4', ..] => Arch::X64,           // "x86_64"
        [b'a', b'a', b'r', b'c', b'h', b'6', b'4', ..] => Arch::AArch64, // "aarch64"
        _ => Arch::X86,
    };

    find_windows_10_kits_tool("KitsRoot10", arch, tool)
        .or_else(|| find_windows_kits_tool("KitsRoot10", arch, tool))
        .or_else(|| find_windows_kits_tool("KitsRoot81", arch, tool))
        .or_else(|| find_windows_kits_tool("KitsRoot", arch, tool))
        .or_else(|| find_latest_windows_sdk_tool(arch, tool))
        .or_else(|| find_with_vswhom(arch, tool))
}

fn find_with_vswhom(arch: Arch, tool: &str) -> Option<PathBuf> {
    let res = VsFindResult::search();
    res.as_ref()
        .and_then(|res| res.windows_sdk_root.as_ref())
        .map(PathBuf::from)
        .and_then(|mut root| {
            let ver = root.file_name().expect("malformed vswhom-returned SDK root").to_os_string();
            root.pop();
            root.pop();
            root.push("bin");
            root.push(ver);
            try_bin_dir(root, "x86", "x64", "arm64", arch)
        })
        .and_then(|pb| try_tool(pb, tool))
        .or_else(move || {
            res.and_then(|res| res.windows_sdk_root)
                .map(PathBuf::from)
                .and_then(|mut root| {
                    root.pop();
                    root.pop();
                    try_bin_dir(root, "bin/x86", "bin/x64", "bin/arm64", arch)
                })
                .and_then(|pb| try_tool(pb, tool))
        })
}

// Windows 8 - 10
fn find_windows_kits_tool(key: &str, arch: Arch, tool: &str) -> Option<PathBuf> {
    winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows Kits\Installed Roots", KEY_QUERY_VALUE)
        .and_then(|reg_key| reg_key.get_value::<String, _>(key))
        .ok()
        .and_then(|root_dir| try_bin_dir(root_dir, "bin/x86", "bin/x64", "bin/arm64", arch))
        .and_then(|pb| try_tool(pb, tool))
}

// Windows Vista - 7
fn find_latest_windows_sdk_tool(arch: Arch, tool: &str) -> Option<PathBuf> {
    winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Microsoft SDKs\Windows", KEY_QUERY_VALUE)
        .and_then(|reg_key| reg_key.get_value::<String, _>("CurrentInstallFolder"))
        .ok()
        .and_then(|root_dir| try_bin_dir(root_dir, "Bin", "Bin/x64", "Bin/arm64", arch))
        .and_then(|pb| try_tool(pb, tool))
}

// Windows 10 with subdir support
fn find_windows_10_kits_tool(key: &str, arch: Arch, tool: &str) -> Option<PathBuf> {
    let kit_root = (winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows Kits\Installed Roots", KEY_QUERY_VALUE)
        .and_then(|reg_key| reg_key.get_value::<String, _>(key))
        .ok())?;
    include_windows_10_kits(&kit_root);
    let root_dir = kit_root + "/bin";

    for entry in fs::read_dir(&root_dir).ok()?.filter(|d| d.is_ok()).map(Result::unwrap) {
        let fname = entry.file_name().into_string();
        let ftype = entry.file_type();
        if fname.is_err() || ftype.is_err() || ftype.unwrap().is_file() {
            continue;
        }

        let fname = entry.file_name().into_string().unwrap();
        if let Some(rc) = try_bin_dir(
            root_dir.clone(),
            &format!("{}/x86", fname),
            &format!("{}/x64", fname),
            &format!("{}/arm64", fname),
            arch,
        )
        .and_then(|pb| try_tool(pb, tool))
        {
            return Some(rc);
        }
    }

    None
}

/// Update %INCLUDE% to contain all \Include\<version>\ folders before invoking rc.exe
/// (https://github.com/nabijaczleweli/rust-embed-resource/pull/17),
/// fixing "Unable to find windows.h" errors (https://github.com/nabijaczleweli/rust-embed-resource/issues/11)
fn include_windows_10_kits(kit_root: &str) {
    static IS_INCLUDED: AtomicBool = AtomicBool::new(false);

    if !IS_INCLUDED.swap(true, SeqCst) {
        let mut include = env::var("INCLUDE").unwrap_or_default();
        if !include.ends_with(';') {
            include.push(';');
        }

        if let Ok(include_root) = fs::read_dir(kit_root.to_string() + r"\Include\") {
            get_dirs(include_root).filter_map(|dir| fs::read_dir(dir.path()).ok()).for_each(|dir| {
                get_dirs(dir).for_each(|sub_dir| {
                    if let Some(sub_dir) = sub_dir.path().to_str() {
                        if !include.contains(sub_dir) {
                            include.push_str(sub_dir);
                            include.push(';');
                        }
                    }
                })
            });
        }

        if let Some(cl) = cc::windows_registry::find_tool(env::var("TARGET").expect("No TARGET env var").as_str(), "cl.exe") {
            if let Some((_, ipaths)) = cl.env().iter().find(|(k, _)| k == "INCLUDE") {
                include.push_str(ipaths.to_str().expect("%INCLUDE% from cc nonrepresentable"));
                include.push(';');
            }
        }

        env::set_var("INCLUDE", include);
    }
}

fn get_dirs(read_dir: fs::ReadDir) -> impl Iterator<Item = fs::DirEntry> {
    read_dir
        .filter_map(|dir| dir.ok())
        .filter(|dir| dir.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
}

fn try_bin_dir<R: Into<PathBuf>>(root_dir: R, x86_bin: &str, x64_bin: &str, aarch64_bin: &str, arch: Arch) -> Option<PathBuf> {
    try_bin_dir_impl(root_dir.into(), x86_bin, x64_bin, aarch64_bin, arch)
}

fn try_bin_dir_impl(mut root_dir: PathBuf, x86_bin: &str, x64_bin: &str, aarch64_bin: &str, arch: Arch) -> Option<PathBuf> {
    match arch {
        Arch::X86 => root_dir.push(x86_bin),
        Arch::X64 => root_dir.push(x64_bin),
        Arch::AArch64 => root_dir.push(aarch64_bin),
    }

    if root_dir.is_dir() {
        Some(root_dir)
    } else {
        None
    }
}

fn try_tool(mut pb: PathBuf, tool: &str) -> Option<PathBuf> {
    pb.push(tool);
    if pb.exists() {
        Some(pb)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        path_has_reparse_ancestor, resource_compiler_attestation_value, validate_resource_compiler_attestation_nonce, validate_resource_compiler_override,
    };
    use std::ffi::OsString;
    use std::{fs, path::PathBuf};

    #[test]
    fn explicit_resource_compiler_must_be_absolute_and_named_rc() {
        assert!(validate_resource_compiler_override(PathBuf::from("rc.exe")).is_err());
        assert!(validate_resource_compiler_override(std::env::current_exe().expect("current test executable")).is_err());
    }

    #[test]
    fn explicit_resource_compiler_accepts_an_absolute_ordinary_rc_file() {
        let root = std::env::temp_dir().join(format!(
            "embed-resource-rc-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create test root");
        let rc = root.join("rc.exe");
        fs::copy(std::env::current_exe().expect("current test executable"), &rc).expect("create ordinary rc fixture");
        assert_eq!(validate_resource_compiler_override(rc.clone()).expect("valid explicit rc"), rc);
        assert!(!path_has_reparse_ancestor(&rc).expect("ordinary path inspection"));
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn explicit_resource_compiler_rejects_a_reparse_ancestor_when_supported() {
        use std::os::windows::fs::symlink_dir;

        let root = std::env::temp_dir().join(format!(
            "embed-resource-rc-reparse-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let real = root.join("real");
        let alias = root.join("alias");
        fs::create_dir_all(&real).expect("create real test root");
        let rc = real.join("rc.exe");
        fs::copy(std::env::current_exe().expect("current test executable"), &rc).expect("create ordinary rc fixture");
        if symlink_dir(&real, &alias).is_err() {
            fs::remove_dir_all(root).expect("remove unsupported reparse test root");
            return;
        }
        let alias_rc = alias.join("rc.exe");
        let error = validate_resource_compiler_override(alias_rc).expect_err("reparse ancestor must be rejected");
        assert!(error.contains("reparse-point"));
        fs::remove_dir(&alias).expect("remove directory symlink");
        fs::remove_dir_all(root).expect("remove reparse test root");
    }

    #[test]
    fn resource_compiler_attestation_nonce_and_value_are_canonical() {
        let nonce = validate_resource_compiler_attestation_nonce(OsString::from("0123456789abcdef0123456789abcdef")).expect("canonical nonce");
        assert_eq!(
            resource_compiler_attestation_value(&nonce, PathBuf::from(r"C:\SDK\rc.exe").as_path()).expect("canonical marker"),
            r"0123456789abcdef0123456789abcdef|C:\SDK\rc.exe"
        );
        assert!(validate_resource_compiler_attestation_nonce(OsString::from("0123456789ABCDEF0123456789ABCDEF")).is_err());
        assert!(resource_compiler_attestation_value(&nonce, PathBuf::from("C:\\bad|path\\rc.exe").as_path()).is_err());
    }
}
