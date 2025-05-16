use super::util::{add_base16_digit, hex_digit_to_value};
use crate::{DockerError, Result};

const CRLF_DELIMITER_SIZE: usize = b"\r\n".len();

/// An HTTP chunk response processor.
pub struct ChunkProcessor {
    state: ChunkProcessorState,
    skip_bytes: Option<usize>,
}

impl ChunkProcessor {
    pub fn new() -> Self {
        ChunkProcessor {
            state: ChunkProcessorState::ReadyForChunkSize,
            skip_bytes: None,
        }
    }

    /// Processes the data available in the provided source buffer and writes it to the destination buffer.
    ///
    /// Returns `true` if there is more data to be read.
    pub fn process_available_data(&mut self, mut src: &[u8], dst: &mut Vec<u8>) -> Result<bool> {
        let should_continue = loop {
            if self.state.is_done() {
                break false;
            }

            // Skip bytes if it was requested
            src = self.skip_bytes(src);
            if self.skip_bytes.is_some() {
                // Need more data to skip the required number of bytes
                break true;
            }

            let (need_more_data, offset, bytes_to_skip) = self.state.advance(src, dst)?;
            if need_more_data {
                // Need more data to continue the processing
                break true;
            }

            // Continue processing the available data until we are done or need more data
            self.skip_bytes = bytes_to_skip;
            src = &src[offset..]
        };

        Ok(should_continue)
    }

    /// Skips the stored in this instance number of bytes (if any).
    ///
    /// Returns the updated buffer without the skipped bytes.
    fn skip_bytes<'a>(&mut self, src: &'a [u8]) -> &'a [u8] {
        if let Some(bytes_to_skip) = self.skip_bytes {
            let availalbe_bytes_to_skip = bytes_to_skip.min(src.len());
            self.skip_bytes = match availalbe_bytes_to_skip < bytes_to_skip {
                true => None,
                false => Some(bytes_to_skip - availalbe_bytes_to_skip),
            };
            &src[availalbe_bytes_to_skip..]
        } else {
            src
        }
    }
}

/// The inner state machine of the [ChunkProcessor].
enum ChunkProcessorState {
    /// Expecting a chunk size
    ReadyForChunkSize,
    /// Parsing the chunk size
    PartialChunkSize(usize),
    /// Parsing the chunk
    RemainingChunkSize(usize),
    /// Encountered the zero-length chunk
    Done,
}

impl ChunkProcessorState {
    /// Advances the processor further.
    ///
    /// Returns a boolean that indicates whether it requires more data to continue, the new offset into the provided source buffer, and the number of bytes that need
    /// to be skipped before the next viable data.
    fn advance(&mut self, src: &[u8], dst: &mut Vec<u8>) -> Result<(bool, usize, Option<usize>)> {
        match self {
            Self::ReadyForChunkSize => {
                *self = Self::PartialChunkSize(0);
                self.advance(src, dst)
            }
            Self::PartialChunkSize(size) => {
                if src.is_empty() {
                    return Ok((true, 0, None));
                }

                let chunk_size_end_pos = src.iter().position(|byte| *byte == b'\r');
                let size_is_complete = chunk_size_end_pos.is_some();
                let chunk_size_end_pos = chunk_size_end_pos.unwrap_or(src.len());

                let chunk_size_bytes = &src[..chunk_size_end_pos];
                for byte in chunk_size_bytes {
                    let digit = hex_digit_to_value(*byte)
                        .ok_or(DockerError::Other("got an invalid digit in HTTP chunk size".into()))?;
                    *size = add_base16_digit(*size, digit);
                }

                if !size_is_complete {
                    // Continue where more data is available
                    return Ok((true, chunk_size_end_pos, None));
                }

                *self = if *size == 0 {
                    ChunkProcessorState::Done
                } else {
                    ChunkProcessorState::RemainingChunkSize(*size)
                };

                let bytes_to_skip = if self.is_done() { 0 } else { CRLF_DELIMITER_SIZE };
                Ok((false, chunk_size_end_pos, Some(bytes_to_skip)))
            }
            Self::RemainingChunkSize(size) => {
                if src.is_empty() {
                    return Ok((true, 0, None));
                }

                let present_chunk_bytes = (*size).min(src.len());
                let chunk_part = &src[..present_chunk_bytes];
                dst.extend_from_slice(chunk_part);

                let remaining_chunk_size = *size - present_chunk_bytes;
                if remaining_chunk_size != 0 {
                    // We will need to finalize reading this chunk once we have more data
                    *self = Self::RemainingChunkSize(remaining_chunk_size);
                    Ok((true, present_chunk_bytes, None))
                } else {
                    // Move on to the next chunk
                    *self = Self::ReadyForChunkSize;
                    Ok((false, present_chunk_bytes, Some(CRLF_DELIMITER_SIZE)))
                }
            }
            Self::Done => Ok((false, 0, None)),
        }
    }

    #[inline]
    fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}
