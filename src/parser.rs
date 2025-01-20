use std::io::{Read, Seek};

use anyhow::Context;
use tar::Archive;

const GZIP_MAGIC_NUMBER: [u8; 2] = [0x1f, 0x8b];

#[derive(Debug, Clone, Copy)]
pub enum BlobType {
    Tar,
    GzippedTar,
}

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

                let mut buf_offset = 0;
                let blob_type: Option<BlobType> = loop {
                    let read = entry
                        .read(&mut tar_header[buf_offset..])
                        .context("failed to get the beginning of a blob")?;

                    match read {
                        0 => break None,
                        1.. => {
                            if buf_offset + read < 2
                                || (tar_header[..2] != GZIP_MAGIC_NUMBER && buf_offset + read < 262)
                            {
                                // We need more data
                                buf_offset = read;
                                continue;
                            }
                        }
                    }

                    if &tar_header[257..262] == b"ustar" {
                        break Some(BlobType::Tar);
                    }

                    if tar_header[..2] == GZIP_MAGIC_NUMBER {
                        break Some(BlobType::GzippedTar);
                    }

                    break None;
                };

                dbg!(blob_type);
            }
        }

        Ok(())
    }
}
