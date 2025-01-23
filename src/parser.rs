use std::io::{Read, Seek};
use std::path::PathBuf;

use anyhow::Context;
use tar::Archive;

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

pub type LayerChangeSet = Vec<ChangedFile>;

#[derive(Debug, Default)]
pub struct Parser {}

impl Parser {
    pub fn parse_image<R: Read + Seek>(&self, src: R) -> anyhow::Result<()> {
        let mut archive = Archive::new(src);
        let mut headers = vec![];

        let mut tar_header = [0u8; 262];
        for entry in archive
            .entries_with_seek()
            .context("failed to get entries from the archive")?
        {
            let mut entry = entry.context("error while reading an entry")?;
            let header = entry.header();
            headers.push(entry.header().clone());

            if header.path_bytes().starts_with(b"blobs/sha256/") {
                // Check if blob is tar/gzipped tar

                let mut filled = 0;
                let blob_type = loop {
                    let read = entry
                        .read(&mut tar_header[filled..])
                        .context("failed to get the beginning of a blob")?;

                    filled += read;
                    match read {
                        0 => break BlobType::Json,
                        1.. => {
                            if filled < 2 || (tar_header[..2] != GZIP_MAGIC_NUMBER && filled != 262) {
                                // We need more data
                                continue;
                            }
                        }
                    }

                    if &tar_header[257..262] == b"ustar" {
                        break BlobType::Tar;
                    }

                    if tar_header[..2] == GZIP_MAGIC_NUMBER {
                        break BlobType::GzippedTar;
                    }

                    break BlobType::Json;
                };

                dbg!(blob_type);

                // Restore the original entry so that it gets parsed correctly
                let entry = tar_header[..filled].chain(entry);

                match blob_type {
                    BlobType::Tar => self.parse_tar_layer(entry).context("error while parsing a tar layer")?,
                    BlobType::GzippedTar => unimplemented!("Not supported for now"),
                    BlobType::Json => {}
                }
            }
        }

        Ok(())
    }

    fn parse_tar_layer<R: Read>(&self, src: R) -> anyhow::Result<()> {
        let mut archive = Archive::new(src);

        let mut change_set = LayerChangeSet::new();
        for entry in archive.entries().context("failed to get entries from the archive")? {
            let entry = entry.context("error while reading an entry")?;
            let header = entry.header();
            if let Ok(path) = header.path() {
                change_set.push(ChangedFile {
                    path: path.into_owned(),
                    size: header.size().unwrap_or(0),
                })
            }
        }

        Ok(())
    }
}
