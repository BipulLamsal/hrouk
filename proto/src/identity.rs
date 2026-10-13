//! The endpoint's secret key, kept on disk so its id survives a restart.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use iroh::SecretKey;

/// Reads the key from `path`, generating and saving one if it's not there yet.
pub fn load_or_create(path: &Path) -> Result<SecretKey> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let bytes = parse(raw.trim()).with_context(|| {
                format!("{} is not a 32-byte hex key", path.display())
            })?;
            Ok(SecretKey::from_bytes(&bytes))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let key = SecretKey::generate();
            write(path, &key)?;
            Ok(key)
        }
        Err(err) => Err(err).with_context(|| format!("read {}", path.display())),
    }
}

/// 64 hex characters → 32 bytes.
fn parse(text: &str) -> Result<[u8; 32]> {
    let bytes: Vec<u8> = (0..text.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(text.get(i..i + 2).unwrap_or(""), 16)
                .map_err(|_| anyhow::anyhow!("bad hex at byte {i}"))
        })
        .collect::<Result<_>>()?;

    bytes
        .try_into()
        .map_err(|v: Vec<u8>| anyhow::anyhow!("expected 32 bytes, got {}", v.len()))
}

fn write(path: &Path, key: &SecretKey) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }

    fs::write(path, format!("{}\n", hex_encode(&key.to_bytes())))
        .with_context(|| format!("write {}", path.display()))?;

    // The file *is* the identity — anyone holding it can impersonate the
    // endpoint. Best-effort: don't abort first-run setup if it fails.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
