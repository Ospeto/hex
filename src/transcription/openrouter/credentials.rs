//! Blocking OS-vault access. Call from worker threads, never the UI thread.
//! Only absence permits environment fallback; vault failures fail closed.

use std::sync::Mutex;

use keyring::{Entry, Error};

const MAX_KEY_BYTES: usize = 4096;
const VAULT_ERROR: &str =
    "OpenRouter credentials could not be accessed in the system credential store";
const INVALID_KEY: &str =
    "OpenRouter API key must be nonempty, at most 4096 bytes, and contain no control characters";
static VAULT: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Missing,
    Saved,
    Environment,
}

fn service() -> &'static str {
    if crate::app_paths::is_macos_test_build() {
        "anomaly.hex.test.openrouter"
    } else {
        "anomaly.hex.openrouter"
    }
}

fn entry(account: &str) -> Result<Entry, String> {
    Entry::new(service(), account).map_err(|_| VAULT_ERROR.to_owned())
}

fn read(entry: &Entry) -> Result<Option<String>, String> {
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(Error::NoEntry) => Ok(None),
        Err(_) => Err(VAULT_ERROR.to_owned()),
    }
}

fn validate(key: &str) -> Result<String, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_KEY_BYTES || key.chars().any(char::is_control) {
        return Err(INVALID_KEY.to_owned());
    }
    Ok(trimmed.to_owned())
}

// Lazy inputs ensure neither a masked environment nor an unnecessary vault
// entry is read. This is also the offline precedence contract used by tests.
fn resolve(
    saved: Result<Option<String>, String>,
    disabled: impl FnOnce() -> Result<bool, String>,
    environment: impl FnOnce() -> Result<Option<String>, String>,
) -> Result<(Option<String>, Status), String> {
    if let Some(key) = saved? {
        return Ok((Some(validate(&key)?), Status::Saved));
    }
    if disabled()? {
        return Ok((None, Status::Missing));
    }
    match environment()? {
        Some(key) => Ok((Some(validate(&key)?), Status::Environment)),
        None => Ok((None, Status::Missing)),
    }
}

fn lookup() -> Result<(Option<String>, Status), String> {
    resolve(
        read(&entry("api-key")?),
        || Ok(read(&entry("environment-disabled")?)?.is_some()),
        || match std::env::var(super::KEY_ENV) {
            Ok(key) => Ok(Some(key)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(INVALID_KEY.to_owned()),
        },
    )
}

/// A saved key wins; removal masks the environment until another key is saved.
pub(crate) fn load() -> Result<Option<String>, String> {
    let _guard = VAULT.lock().map_err(|_| VAULT_ERROR.to_owned())?;
    lookup().map(|(key, _)| key)
}

/// Stores only in the OS vault. The environment-disable marker is retained.
pub(crate) fn save(key: &str) -> Result<(), String> {
    let key = validate(key)?;
    let _guard = VAULT.lock().map_err(|_| VAULT_ERROR.to_owned())?;
    entry("api-key")?
        .set_password(&key)
        .map_err(|_| VAULT_ERROR.to_owned())
}

/// Disable environment fallback first. If deletion fails, the saved key still wins.
pub(crate) fn remove() -> Result<(), String> {
    let _guard = VAULT.lock().map_err(|_| VAULT_ERROR.to_owned())?;
    entry("environment-disabled")?
        .set_password("1")
        .map_err(|_| VAULT_ERROR.to_owned())?;
    match entry("api-key")?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(_) => Err(VAULT_ERROR.to_owned()),
    }
}

/// Reports only provenance, never a key or a key fragment.
pub(crate) fn status() -> Result<Status, String> {
    let _guard = VAULT.lock().map_err(|_| VAULT_ERROR.to_owned())?;
    lookup().map(|(_, status)| status)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual native-store check only: never uses Hex's service or environment.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "writes synthetic sentinels to a unique macOS Keychain service"]
    fn native_keychain_synthetic_lifecycle() -> Result<(), &'static str> {
        const CHILD_SERVICE: &str = "HEX_SYNTHETIC_KEYCHAIN_CHECK_SERVICE";
        if let Ok(service) = std::env::var(CHILD_SERVICE) {
            if !service.starts_with("hex-credentials-native-check.") {
                return Err("Synthetic Keychain service scope mismatch");
            }
            let entry = Entry::new(&service, "synthetic-sentinel")
                .map_err(|_| "Synthetic child entry creation failed")?;
            let value = entry
                .get_password()
                .map_err(|_| "Synthetic cross-process retrieval failed")?;
            assert!(
                value == "invented-hex-native-sentinel-a",
                "Synthetic cross-process mismatch"
            );
            return Ok(());
        }
        struct Cleanup(Entry);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                // Also runs on assertion panic and early return. Never log native errors.
                let _ = self.0.delete_credential();
            }
        }

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "Synthetic test clock unavailable")?
            .as_nanos();
        let service = format!(
            "hex-credentials-native-check.{}.{nonce}",
            std::process::id()
        );
        let account = "synthetic-sentinel";
        let fresh = || {
            Entry::new(&service, account).map_err(|_| "Synthetic Keychain entry creation failed")
        };
        let cleanup = Cleanup(fresh()?);
        cleanup
            .0
            .set_password("invented-hex-native-sentinel-a")
            .map_err(|_| "Synthetic Keychain write failed; permissions left unchanged")?;

        let test_module = module_path!()
            .split_once("::")
            .ok_or("Synthetic test module unavailable")?
            .1;
        let test_name = format!("{test_module}::native_keychain_synthetic_lifecycle");
        let child = std::process::Command::new(
            std::env::current_exe().map_err(|_| "Synthetic test executable unavailable")?,
        )
        .args(["--ignored", "--exact", &test_name])
        .env(CHILD_SERVICE, &service)
        .output()
        .map_err(|_| "Synthetic child process failed")?;
        assert!(
            child.status.success() && String::from_utf8_lossy(&child.stdout).contains("1 passed"),
            "Synthetic cross-process check failed or did not run"
        );

        let second = fresh()?;
        let first_read = second
            .get_password()
            .map_err(|_| "Synthetic Keychain reconstruction read failed")?;
        assert!(
            first_read == "invented-hex-native-sentinel-a",
            "Synthetic value mismatch"
        );
        second
            .set_password("invented-hex-native-sentinel-b")
            .map_err(|_| "Synthetic Keychain replacement failed")?;
        let third = fresh()?;
        let replacement = third
            .get_password()
            .map_err(|_| "Synthetic Keychain replacement read failed")?;
        assert!(
            replacement == "invented-hex-native-sentinel-b",
            "Synthetic replacement mismatch"
        );
        third
            .delete_credential()
            .map_err(|_| "Synthetic Keychain deletion failed")?;
        assert!(
            matches!(fresh()?.get_password(), Err(Error::NoEntry)),
            "Synthetic Keychain deletion was not confirmed"
        );
        Ok(())
    }

    #[test]
    fn saved_wins_without_reading_marker_or_environment() {
        let (key, status) = resolve(
            Ok(Some(" synthetic-saved ".into())),
            || panic!("must not read marker"),
            || panic!("must not read environment"),
        )
        .unwrap();
        assert_eq!(key.as_deref(), Some("synthetic-saved"));
        assert!(status == Status::Saved);
    }

    #[test]
    fn removed_masks_environment() {
        let (key, status) =
            resolve(Ok(None), || Ok(true), || panic!("environment masked")).unwrap();
        assert!(key.is_none());
        assert!(status == Status::Missing);
    }

    #[test]
    fn environment_is_only_used_without_saved_key_or_marker() {
        let (key, status) = resolve(
            Ok(None),
            || Ok(false),
            || Ok(Some(" synthetic-env ".into())),
        )
        .unwrap();
        assert_eq!(key.as_deref(), Some("synthetic-env"));
        assert!(status == Status::Environment);
        let (key, status) = resolve(Ok(None), || Ok(false), || Ok(None)).unwrap();
        assert!(key.is_none());
        assert!(status == Status::Missing);
    }

    #[test]
    fn errors_never_fall_back() {
        assert!(
            resolve(
                Err(VAULT_ERROR.into()),
                || panic!("saved read failed"),
                || panic!("saved read failed"),
            )
            .is_err()
        );
        assert!(
            resolve(
                Ok(None),
                || Err(VAULT_ERROR.into()),
                || panic!("marker read failed"),
            )
            .is_err()
        );
        assert!(
            resolve(
                Ok(Some("invalid\nkey".into())),
                || panic!("invalid saved key"),
                || panic!("invalid saved key"),
            )
            .is_err()
        );
        assert!(resolve(Ok(None), || Ok(false), || Err(INVALID_KEY.into())).is_err());
    }

    #[test]
    fn validation_is_bounded_trimmed_and_sanitized() {
        assert_eq!(validate(" synthetic-key ").unwrap(), "synthetic-key");
        assert!(validate(&"x".repeat(MAX_KEY_BYTES)).is_ok());
        for key in [
            "",
            "  ",
            "synthetic\nkey",
            "\tsynthetic",
            "synthetic\0",
            "synthetic\u{7f}",
        ] {
            assert_eq!(validate(key).unwrap_err(), INVALID_KEY);
        }
        assert_eq!(
            validate(&"x".repeat(MAX_KEY_BYTES + 1)).unwrap_err(),
            INVALID_KEY
        );
        assert!(resolve(Ok(None), || Ok(false), || Ok(Some(" ".into()))).is_err());
    }
}
