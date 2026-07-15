// Responsibility: executable-build-identity
use crate::model::{BuildIdentity, ExecutableIdentityDiagnostics, PathExecutableIdentity};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) fn build_identity(include_binary_sha256: bool) -> BuildIdentity {
    let executable = current_executable();
    let binary_sha256 = include_binary_sha256
        .then(|| sha256_file(&executable))
        .flatten();
    BuildIdentity {
        semver: env!("CARGO_PKG_VERSION").to_string(),
        cache_format: cache_format(),
        schema_manifest_version: schema_manifest_version(),
        executable_path: executable.to_string_lossy().to_string(),
        binary_sha256_state: if !include_binary_sha256 {
            "not_requested"
        } else if binary_sha256.is_some() {
            "computed"
        } else {
            "unavailable"
        }
        .to_string(),
        binary_sha256,
        source_commit: option_env!("CODEMAP_SOURCE_COMMIT").map(str::to_string),
        dirty_build: option_env!("CODEMAP_DIRTY_BUILD").and_then(|value| value.parse().ok()),
    }
}

pub(crate) fn identity_diagnostics() -> ExecutableIdentityDiagnostics {
    let build_identity = build_identity(true);
    let path_identity = path_executable_identity();
    let executable_mismatch = executable_mismatch(&build_identity, &path_identity);
    ExecutableIdentityDiagnostics {
        build_identity,
        path_identity,
        executable_mismatch,
    }
}

pub(crate) fn path_executable_identity() -> PathExecutableIdentity {
    let current = current_executable();
    let Some(executable) = resolve_path_executable("codemap") else {
        return PathExecutableIdentity {
            executable_path: None,
            semver: None,
            binary_sha256: None,
            binary_sha256_state: "unavailable".to_string(),
            version_probe: "not_found".to_string(),
        };
    };
    let same_executable = same_file(&current, &executable);
    let semver = if same_executable {
        Some(env!("CARGO_PKG_VERSION").to_string())
    } else {
        probe_semver(&executable)
    };
    let binary_sha256 = sha256_file(&executable);
    PathExecutableIdentity {
        executable_path: Some(executable.to_string_lossy().to_string()),
        binary_sha256_state: if binary_sha256.is_some() {
            "computed"
        } else {
            "unavailable"
        }
        .to_string(),
        binary_sha256,
        version_probe: if same_executable {
            "same_executable".to_string()
        } else if semver.is_some() {
            "ok".to_string()
        } else {
            "unavailable".to_string()
        },
        semver,
    }
}

pub(crate) fn executable_mismatch(
    build_identity: &BuildIdentity,
    path_identity: &PathExecutableIdentity,
) -> Option<bool> {
    let current = Path::new(&build_identity.executable_path);
    path_identity
        .executable_path
        .as_deref()
        .map(Path::new)
        .map(|path| !same_file(current, path))
}

pub(crate) fn current_executable() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| path.canonicalize().ok().or(Some(path)))
        .unwrap_or_else(|| PathBuf::from("codemap"))
}

fn resolve_path_executable(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(executable_name(name)))
        .find(|candidate| is_executable_file(candidate))
        .map(|candidate| candidate.canonicalize().unwrap_or(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn executable_name(name: &str) -> String {
    if env::consts::EXE_EXTENSION.is_empty() {
        name.to_string()
    } else {
        format!("{name}.{}", env::consts::EXE_EXTENSION)
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn probe_semver(executable: &Path) -> Option<String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let probe_dir =
        env::temp_dir().join(format!("codemap-path-probe-{}-{nonce}", std::process::id()));
    fs::create_dir(&probe_dir).ok()?;
    let result = (|| {
        let mut child = Command::new(executable)
            .arg("--version")
            // A PATH installation is untrusted relative to the target project.
            // Even a badly behaved version probe must not be able to write there.
            .current_dir(&probe_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(status) = child.try_wait().ok()? {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            thread::sleep(Duration::from_millis(10));
        };
        let mut stdout = String::new();
        let mut stderr = String::new();
        child.stdout.take()?.read_to_string(&mut stdout).ok()?;
        child.stderr.take()?.read_to_string(&mut stderr).ok()?;
        Some((status, stdout, stderr))
    })();
    let _ = fs::remove_dir_all(&probe_dir);
    let (status, stdout, stderr) = result?;
    let output = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };
    if !status.success() {
        return None;
    }
    output
        .lines()
        .next()?
        .split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|char| char.is_ascii_digit())
        })
        .map(|part| part.trim().to_string())
}

fn sha256_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
}

fn cache_format() -> String {
    let surface = crate::repo::VERSION
        .split_once('+')
        .map(|(_, format)| format)
        .unwrap_or("unknown");
    format!(
        "{surface};fingerprints-v{};lens-artifacts-v{}",
        crate::cache::fingerprint_format_version(),
        crate::cache::lens_artifact_format_version()
    )
}

fn schema_manifest_version() -> u64 {
    serde_json::from_str::<serde_json::Value>(include_str!("../../schemas/manifest.json"))
        .ok()
        .and_then(|manifest| manifest.get("version").and_then(serde_json::Value::as_u64))
        .unwrap_or(0)
}
