use core::str;
use std::io::{Read, Seek};
use std::path::PathBuf;

use anyhow::Context;
use serde::de::{DeserializeOwned, Visitor};
use serde::{Deserialize, Deserializer};
use tar::{Archive, Entry};

const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];
const TAR_BLOCK_SIZE: u64 = 512;
const TAR_MAGIC_NUMBER: &[u8] = b"ustar";
const BLOB_PATH_PREFIX: &[u8] = b"blobs/sha256/";
const SHA256_DIGEST_PREFIX: &[u8] = b"sha256:";
const SHA256_HASH_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy)]
pub enum BlobType {
    Tar,
    GzippedTar,
    Json,
}

#[derive(Debug, Deserialize)]
pub struct ConfigLayer {
    #[serde(rename = "mediaType")]
    media_type: String,
    #[serde(deserialize_with = "deserialize_sha256_hash")]
    digest: Sha256Hash,
    size: u64,
}

#[derive(Debug, Deserialize)]
pub struct HistoryEntry {
    created_by: String,
    comment: String,
    empty_layer: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum JsonBlob {
    Manifest { layers: Vec<ConfigLayer> },
    Config { history: Vec<HistoryEntry> },
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    path: PathBuf,
    size: u64,
}

pub type LayerChangeSet = Vec<ChangedFile>;

#[derive(Debug, Default)]
pub struct Parser {}

type Sha256Hash = [u8; SHA256_HASH_LENGTH];

/// Deserializes a hex string with sha256 hash that is prepended with the `sha256:` prefix
fn deserialize_sha256_hash<'de, D>(de: D) -> Result<Sha256Hash, D::Error>
where
    D: Deserializer<'de>,
{
    struct Sha256HashVisitor;

    impl<'de> Visitor<'de> for Sha256HashVisitor {
        type Value = Sha256Hash;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sha256 hash string prefixed with `sha256:`")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // Strip the `sha256:` prefix
            let raw = &v[SHA256_DIGEST_PREFIX.len()..];

            // multiply by 2 because we are dealing with a hex str
            if raw.len() != SHA256_HASH_LENGTH * 2 {
                return Err(serde::de::Error::custom("Invalid sha256 digest format"));
            }

            sha256_hash_from_hex(raw).map_err(|_| serde::de::Error::custom("Failed to parse the sha256 hash"))
        }
    }

    de.deserialize_str(Sha256HashVisitor)
}
pub fn sha256_hash_from_hex(src: impl AsRef<str>) -> anyhow::Result<Sha256Hash> {
    let mut sha256_hash = [0u8; 32];

    for (idx, byte_str) in src
        .as_ref()
        .as_bytes()
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

impl Parser {
    pub fn parse_image<R: Read + Seek>(&self, src: R) -> anyhow::Result<()> {
        let mut headers = vec![];

        let mut archive = Archive::new(src);
        let mut entries = archive
            .entries_with_seek()
            .context("failed to get entries from the archive")?;

        let mut tar_header = [0u8; 262];
        while let Some(entry) = entries.next() {
            let mut entry = entry.context("error while reading an entry")?;

            let header = entry.header();
            headers.push(entry.header().clone());

            dbg!(header);
            let entry_size_in_blocks = {
                let entry_size = header.entry_size().context("failed to get the entry's file size")?;
                if entry_size != 0 {
                    (entry_size / TAR_BLOCK_SIZE) + (entry_size % TAR_BLOCK_SIZE != 0) as u64
                } else {
                    0
                }
            };

            // Check if it's a blob or index/manifest
            if header.path_bytes().starts_with(BLOB_PATH_PREFIX) && entry_size_in_blocks != 0 {
                // Check if blob is tar/gzipped tar
                let (blob_type, offset) = self
                    .determine_blob_type(&mut tar_header, &mut entry)
                    .context("failed to determine the blob type of an entry")?;

                match blob_type {
                    BlobType::Tar => {
                        let mut reader = archive.into_inner();

                        if offset != 0 {
                            // Restore the original entry so that it gets parsed correctly
                            reader
                                .seek_relative(-(offset as i64))
                                .context("failed to wind back the reader")?;
                        }

                        self.parse_tar_blob(&mut reader, entry_size_in_blocks * TAR_BLOCK_SIZE)
                            .context("error while parsing a tar layer")?;

                        archive = Archive::new(reader);
                        entries = archive.entries_with_seek()?;
                    }
                    BlobType::GzippedTar => todo!("Add support for gzipped tar layers"),
                    BlobType::Json => {
                        dbg!(offset);
                        let json_blob = self.parse_json_blob::<JsonBlob>(&mut tar_header[..offset].chain(entry))?;
                        dbg!(json_blob);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_json_blob<T: DeserializeOwned>(&self, entry: &mut impl Read) -> anyhow::Result<Option<T>> {
        let parsed = match serde_json::from_reader::<_, T>(entry) {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                if e.is_data() {
                    None
                } else {
                    anyhow::bail!("faield to parse a JSON blob: {}", e)
                }
            }
        };

        Ok(parsed)
    }

    fn parse_tar_blob<R: Read + Seek>(&self, src: &mut R, blob_size: u64) -> anyhow::Result<LayerChangeSet> {
        let mut archive = Archive::new(src);
        archive.set_ignore_zeros(true);

        let mut change_set = LayerChangeSet::new();
        for entry in archive
            .entries_with_seek()
            .context("failed to get entries from the tar blob")?
        {
            let entry = entry.context("error while reading an entry")?;
            let header = entry.header();

            if entry.raw_header_position() >= blob_size {
                // We parsed the current blob: wind back the header and return
                archive
                    .into_inner()
                    .seek_relative(-(TAR_BLOCK_SIZE as i64))
                    .context("failed to wind back the header")?;

                return Ok(change_set);
            }

            if let Ok(path) = header.path() {
                change_set.push(ChangedFile {
                    path: path.into_owned(),
                    size: header.size().unwrap_or(0),
                })
            }
        }

        Ok(change_set)
    }

    fn determine_blob_type<R: Read + Seek>(
        &self,
        buf: &mut [u8],
        entry: &mut Entry<'_, R>,
    ) -> anyhow::Result<(BlobType, usize)> {
        let mut filled = 0;

        while filled != buf.len() {
            let read = entry
                .read(&mut buf[filled..])
                .context("failed to get the beginning of a blob")?;
            filled += read;

            let blob_type = match read {
                0 => BlobType::Json,
                1.. => {
                    if filled < 2 || (buf[..2] != GZIP_MAGIC_NUMBER && filled != 262) {
                        // We need more data
                        continue;
                    }
                    // TODO: a layer can be a set of zeroes, account for that here
                    // because otherwise we will try to parse it as a JSON :)
                    if &buf[257..262] == TAR_MAGIC_NUMBER {
                        BlobType::Tar
                    } else if buf[..2] == GZIP_MAGIC_NUMBER {
                        BlobType::GzippedTar
                    } else {
                        BlobType::Json
                    }
                }
            };

            return Ok((blob_type, filled));
        }

        Ok((BlobType::Json, filled))
    }
}
