use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::{DockerError, Result};

pub type Sha256Digest = [u8; 32];

/// Calculates a SHA256 digest from the provided data.
pub fn sha256_digest<T: AsRef<[u8]>>(src: T) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(src.as_ref());
    hasher.finalize().into()
}

/// Creates a hex string from the provided [Sha256Digest].
pub fn encode_sha256_digest(digest: Sha256Digest) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

/// Returns the home directory of the current user.
pub fn get_home_dir() -> Result<PathBuf> {
    homedir::my_home()?.ok_or(DockerError::Other(
        "unnable to resolve the current user's home directory".into(),
    ))
}
