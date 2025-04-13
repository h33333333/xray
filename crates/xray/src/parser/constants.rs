pub(super) const BLOB_PATH_PREFIX: &[u8] = b"blobs/sha256/";

pub(super) const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];

pub(super) const TAR_BLOCK_SIZE: usize = 512;
pub(super) const TAR_MAGIC_NUMBER_START_IDX: usize = 257;
pub(super) const TAR_MAGIC_NUMBER: &[u8] = b"ustar";

pub(super) const SHA256_DIGEST_PREFIX: &[u8] = b"sha256:";
pub(super) const SHA256_DIGEST_LENGTH: usize = 32;

pub(super) const IMAGE_MANIFEST_PATH: &[u8] = b"manifest.json";
