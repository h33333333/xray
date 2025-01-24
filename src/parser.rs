use std::io::{Read, Seek};
use std::path::PathBuf;

use anyhow::Context;
use serde::de::DeserializeOwned;
use serde_query::Deserialize;
use tar::{Archive, Entry};

const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];

#[derive(Debug, Clone, Copy)]
pub enum BlobType {
    Tar,
    GzippedTar,
    Json,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    path: PathBuf,
    size: u64,
}

#[derive(Debug, Deserialize)]
pub struct Index {
    #[query(".manifests.[0].digest")]
    manifest_digest: String,
}

pub type LayerChangeSet = Vec<ChangedFile>;

#[derive(Debug, Default)]
pub struct Parser {}

const TAR_BLOCK_SIZE: u64 = 512;
const TAR_MAGIC_NUMBER: &[u8] = b"ustar";
const BLOB_PATH_PREFIX: &[u8] = b"blobs/sha256/";
const IMAGE_INDEX_PATH: &[u8] = b"index.json";

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
                        // TODO: add parsing of JSON blobs
                    }
                }
            } else if header.path_bytes() == IMAGE_INDEX_PATH {
                let index = self.parse_json_blob::<Index>(&mut entry)?;
                dbg!(index);
            }
        }

        Ok(())
    }

    fn parse_json_blob<T: DeserializeOwned>(&self, entry: &mut impl Read) -> anyhow::Result<T> {
        let parsed = serde_json::from_reader::<_, T>(entry).context("failed to parse a json blob")?;
        Ok(parsed)
    }

    fn parse_tar_blob<R: Read + Seek>(&self, src: &mut R, blob_size: u64) -> anyhow::Result<LayerChangeSet> {
        let mut archive = Archive::new(src);
        archive.set_ignore_zeros(true);

        let mut change_set = LayerChangeSet::new();
        for entry in archive.entries().context("failed to get entries from the tar blob")? {
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
