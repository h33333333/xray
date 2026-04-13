use std::io::Read;

use anyhow::Context as _;
use tar::Header;

use super::constants::{
    GZIP_MAGIC_NUMBER, SHA256_DIGEST_LENGTH, TAR_BLOCK_SIZE,
};
use super::{
    BlobType, Sha256Digest, TAR_MAGIC_NUMBER, TAR_MAGIC_NUMBER_START_IDX,
};

/// Converts a SHA256 digest from hex bytes to [Sha256Digest].
pub(super) fn sha256_digest_from_hex(
    src: impl AsRef<[u8]>,
) -> anyhow::Result<Sha256Digest> {
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
        let byte = u8::from_str_radix(byte_str, 16)
            .context("error while parsing a sha256 from a hex str")?;
        sha256_hash[idx] = byte;
    }

    Ok(sha256_hash)
}

/// Converts a Tar entry's [size](Header::entry_size) to the number of Tar blocks, rounding up.
pub(super) fn get_entry_size_in_blocks(header: &Header) -> anyhow::Result<u64> {
    let entry_size = header
        .entry_size()
        .context("failed to get the entry's file size")?;
    if entry_size == 0 {
        return Ok(0);
    }

    Ok((entry_size / TAR_BLOCK_SIZE as u64)
        + (entry_size % TAR_BLOCK_SIZE as u64 != 0) as u64)
}

/// Determines the type of a blob by reading the first [Tar block](TAR_BLOCK_SIZE) and checking its contents.
///
/// Returns the determined [BlobType] and the number of bytes that were read from the provided reader.
pub(super) fn determine_blob_type<R: Read>(
    buf: &mut [u8],
    src: &mut R,
) -> anyhow::Result<(BlobType, usize)> {
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
                if filled == TAR_BLOCK_SIZE && buf.iter().all(|byte| *byte == 0)
                {
                    BlobType::Empty
                } else if filled
                    >= TAR_MAGIC_NUMBER_START_IDX + TAR_MAGIC_NUMBER.len()
                    && has_tar_magic_number(&buf)
                {
                    BlobType::Tar
                } else if filled >= GZIP_MAGIC_NUMBER.len()
                    && buf.starts_with(&GZIP_MAGIC_NUMBER)
                {
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
        || &buf[TAR_MAGIC_NUMBER_START_IDX
            ..TAR_MAGIC_NUMBER_START_IDX + TAR_MAGIC_NUMBER.len()]
            != TAR_MAGIC_NUMBER
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    // --- sha256_digest_from_hex ---

    #[test]
    fn sha256_from_hex_valid() {
        let hex = b"a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let result = sha256_digest_from_hex(hex).unwrap();
        let expected: [u8; 32] = [
            0xa6, 0x65, 0xa4, 0x59, 0x20, 0x42, 0x2f, 0x9d,
            0x41, 0x7e, 0x48, 0x67, 0xef, 0xdc, 0x4f, 0xb8,
            0xa0, 0x4a, 0x1f, 0x3f, 0xff, 0x1f, 0xa0, 0x7e,
            0x99, 0x8e, 0x86, 0xf7, 0xf7, 0xa2, 0x7a, 0xe3,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn sha256_from_hex_uppercase() {
        let hex = b"A665A45920422F9D417E4867EFDC4FB8A04A1F3FFF1FA07E998E86F7F7A27AE3";
        let result = sha256_digest_from_hex(hex).unwrap();
        assert_eq!(result[0], 0xa6);
        assert_eq!(result[31], 0xe3);
    }

    #[test]
    fn sha256_from_hex_all_zeros() {
        let hex = b"0000000000000000000000000000000000000000000000000000000000000000";
        let result = sha256_digest_from_hex(hex).unwrap();
        assert_eq!(result, [0u8; 32]);
    }

    #[test]
    fn sha256_from_hex_wrong_length() {
        let hex = b"a665a459";
        let result = sha256_digest_from_hex(hex);
        assert!(result.is_err());
    }

    #[test]
    fn sha256_from_hex_too_long() {
        let hex = b"a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3ff";
        let result = sha256_digest_from_hex(hex);
        assert!(result.is_err());
    }

    #[test]
    fn sha256_from_hex_invalid_chars() {
        // 'zz' is not valid hex
        let hex = b"zz65a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let result = sha256_digest_from_hex(hex);
        assert!(result.is_err());
    }

    // --- get_entry_size_in_blocks ---

    #[test]
    fn entry_size_zero_returns_zero_blocks() {
        let mut header = Header::new_gnu();
        header.set_size(0);
        assert_eq!(get_entry_size_in_blocks(&header).unwrap(), 0);
    }

    #[test]
    fn entry_size_exactly_one_block() {
        let mut header = Header::new_gnu();
        header.set_size(TAR_BLOCK_SIZE as u64);
        assert_eq!(get_entry_size_in_blocks(&header).unwrap(), 1);
    }

    #[test]
    fn entry_size_one_byte_over_block() {
        let mut header = Header::new_gnu();
        header.set_size(TAR_BLOCK_SIZE as u64 + 1);
        assert_eq!(get_entry_size_in_blocks(&header).unwrap(), 2);
    }

    #[test]
    fn entry_size_one_byte() {
        let mut header = Header::new_gnu();
        header.set_size(1);
        assert_eq!(get_entry_size_in_blocks(&header).unwrap(), 1);
    }

    #[test]
    fn entry_size_multiple_blocks_exact() {
        let mut header = Header::new_gnu();
        header.set_size(TAR_BLOCK_SIZE as u64 * 5);
        assert_eq!(get_entry_size_in_blocks(&header).unwrap(), 5);
    }

    // --- determine_blob_type ---

    #[test]
    fn blob_type_gzip() {
        let mut buf = [0u8; TAR_BLOCK_SIZE];
        // Gzip magic: 0x1f 0x8b followed by other data
        let data = {
            let mut d = vec![0x1f, 0x8b];
            d.extend(vec![0u8; TAR_BLOCK_SIZE - 2]);
            d
        };
        let mut cursor = Cursor::new(data);
        let (blob_type, _) = determine_blob_type(&mut buf, &mut cursor).unwrap();
        assert!(matches!(blob_type, BlobType::GzippedTar));
    }

    #[test]
    fn blob_type_tar() {
        let mut buf = [0u8; TAR_BLOCK_SIZE];
        let mut data = vec![0u8; TAR_BLOCK_SIZE];
        data[TAR_MAGIC_NUMBER_START_IDX
            ..TAR_MAGIC_NUMBER_START_IDX + TAR_MAGIC_NUMBER.len()]
            .copy_from_slice(TAR_MAGIC_NUMBER);
        // Put non-zero data before the magic so it doesn't match Empty
        data[0] = 0x01;
        let mut cursor = Cursor::new(data);
        let (blob_type, _) = determine_blob_type(&mut buf, &mut cursor).unwrap();
        assert!(matches!(blob_type, BlobType::Tar));
    }

    #[test]
    fn blob_type_empty() {
        let mut buf = [0u8; TAR_BLOCK_SIZE];
        let data = vec![0u8; TAR_BLOCK_SIZE];
        let mut cursor = Cursor::new(data);
        let (blob_type, _) = determine_blob_type(&mut buf, &mut cursor).unwrap();
        assert!(matches!(blob_type, BlobType::Empty));
    }

    #[test]
    fn blob_type_json_fallback() {
        let mut buf = [0u8; TAR_BLOCK_SIZE];
        // Non-zero data that isn't gzip or tar magic
        let mut data = vec![0x7b; TAR_BLOCK_SIZE]; // 0x7b = '{'
        // Make sure it doesn't accidentally have tar magic
        data[TAR_MAGIC_NUMBER_START_IDX] = 0x7b;
        let mut cursor = Cursor::new(data);
        let (blob_type, _) = determine_blob_type(&mut buf, &mut cursor).unwrap();
        assert!(matches!(blob_type, BlobType::Json));
    }

    #[test]
    fn blob_type_unknown_on_empty_reader() {
        let mut buf = [0u8; TAR_BLOCK_SIZE];
        let data: Vec<u8> = vec![];
        let mut cursor = Cursor::new(data);
        let (blob_type, _) = determine_blob_type(&mut buf, &mut cursor).unwrap();
        assert!(matches!(blob_type, BlobType::Unknown));
    }

    #[test]
    fn blob_type_json_on_short_input() {
        let mut buf = [0u8; TAR_BLOCK_SIZE];
        // Short non-gzip data that hits EOF before filling a full block
        let data = b"{\"layers\": []}".to_vec();
        let mut cursor = Cursor::new(data);
        let (blob_type, _) = determine_blob_type(&mut buf, &mut cursor).unwrap();
        assert!(matches!(blob_type, BlobType::Json));
    }
}
