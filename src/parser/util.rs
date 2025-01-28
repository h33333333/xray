use std::io::Read;

use anyhow::Context as _;
use tar::Header;

use super::constants::{GZIP_MAGIC_NUMBER, SHA256_DIGEST_LENGTH, TAR_BLOCK_SIZE};
use super::{BlobType, Sha256Digest, TAR_MAGIC_NUMBER, TAR_MAGIC_NUMBER_START_IDX};

/// Converts a SHA256 digest from hex bytes to [Sha256Digest].
pub(super) fn sha256_digest_from_hex(src: impl AsRef<[u8]>) -> anyhow::Result<Sha256Digest> {
    if src.as_ref().len() != SHA256_DIGEST_LENGTH * 2 {
        anyhow::bail!(
            "Expected a slice of length {}, got {}",
            SHA256_DIGEST_LENGTH * 2,
            src.as_ref().len()
        );
    }

    let mut sha256_hash = [0u8; SHA256_DIGEST_LENGTH];
    for (idx, byte_str) in src
        .as_ref()
        .chunks(2)
        .map(std::str::from_utf8)
        .filter_map(Result::ok)
        .enumerate()
    {
        let byte = u8::from_str_radix(byte_str, 16).context("error while parsing a sha256 from a hex str")?;
        sha256_hash[idx] = byte;
    }

    Ok(sha256_hash)
}

/// Converts a Tar entry's [size](Header::entry_size) to the number of Tar blocks, rounding up.
pub(super) fn get_entry_size_in_blocks(header: &Header) -> anyhow::Result<u64> {
    let entry_size = header.entry_size().context("failed to get the entry's file size")?;
    if entry_size != 0 {
        Ok((entry_size / TAR_BLOCK_SIZE as u64) + (entry_size % TAR_BLOCK_SIZE as u64 != 0) as u64)
    } else {
        Ok(0)
    }
}

/// Determines the type of a blob by reading the first [Tar block](TAR_BLOCK_SIZE) and checking its contents.
///
/// Returns the determined [BlobType] and the number of bytes that was read from the provided reader.
pub(super) fn determine_blob_type<R: Read>(buf: &mut [u8], src: &mut R) -> anyhow::Result<(BlobType, usize)> {
    let mut filled = 0;

    while filled != buf.len() {
        let read = src
            .read(&mut buf[filled..])
            .context("failed to get the beginning of a blob")?;
        filled += read;

        let blob_type = match read {
            0 => {
                if filled != 0 {
                    // If nothing else matched and we reached EOF, then this blob must be a JSON
                    BlobType::Json
                } else {
                    // Nothing to read for this blob, so we can't be sure about it's type
                    BlobType::Unknown
                }
            }
            1.. => {
                if filled == TAR_BLOCK_SIZE && buf.iter().all(|byte| *byte == 0) {
                    BlobType::Empty
                } else if filled >= TAR_MAGIC_NUMBER_START_IDX + TAR_MAGIC_NUMBER.len() && has_tar_magic_number(&buf) {
                    BlobType::Tar
                } else if filled >= GZIP_MAGIC_NUMBER.len() && buf.starts_with(&GZIP_MAGIC_NUMBER) {
                    BlobType::GzippedTar
                } else if filled == TAR_BLOCK_SIZE {
                    // We read a single tar block and weren't able to match this layer to any other type, so it must be a JSON
                    BlobType::Json
                } else {
                    // We need more data
                    continue;
                }
            }
        };

        return Ok((blob_type, filled));
    }

    Ok((BlobType::Json, filled))
}

/// Checks if the provided buffer has the [Tar magic number](TAR_MAGIC_NUMBER) set.
fn has_tar_magic_number(buf: impl AsRef<[u8]>) -> bool {
    let buf = buf.as_ref();
    if buf.len() < TAR_MAGIC_NUMBER_START_IDX + TAR_MAGIC_NUMBER.len()
        || &buf[TAR_MAGIC_NUMBER_START_IDX..TAR_MAGIC_NUMBER_START_IDX + TAR_MAGIC_NUMBER.len()] != TAR_MAGIC_NUMBER
    {
        return false;
    }

    true
}
