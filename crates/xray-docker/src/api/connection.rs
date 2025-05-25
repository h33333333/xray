use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use http::StatusCode;
use httparse::{EMPTY_HEADER, Status};
#[cfg(windows)]
use interprocess::os::windows::named_pipe::{DuplexPipeStream, pipe_mode};

use super::chunk_processor::ChunkProcessor;
use crate::{DockerError, Result};

trait ReadWrite: Read + Write {}

impl<T> ReadWrite for T where T: Read + Write {}

pub enum DockerApiConnection {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    Windows(DuplexPipeStream<pipe_mode::Bytes>),
}

enum BodyParsingMode {
    Chunks(ChunkProcessor),
    FixedLength(usize),
}

impl DockerApiConnection {
    const DEFAULT_BUF_SIZE: usize = 8_192;

    #[cfg(unix)]
    pub fn connect<R: AsRef<str>>(docker_socket_addr: R) -> Result<Self> {
        let conn = UnixStream::connect(
            // Strip the unix socket addr prefix if it's present
            docker_socket_addr
                .as_ref()
                .strip_prefix("unix://")
                .unwrap_or(docker_socket_addr.as_ref()),
        )
        .map_err(|e| {
            DockerError::from_io_error_with_description(e, || "failed to connect to the Docker socket".into())
        })?;

        Ok(DockerApiConnection::Unix(conn))
    }

    #[cfg(windows)]
    pub fn connect<R: AsRef<str>>(pipe: R) -> Result<Self> {
        let pipe_stream = DuplexPipeStream::<pipe_mode::Bytes>::connect_by_path(
            // Strip the named pipe addr prefix if it's present
            pipe.as_ref().strip_prefix(r"npipe://").unwrap_or(pipe.as_ref()),
        )
        .map_err(|e| {
            DockerError::from_io_error_with_description(e, || "failed to connect to the Docker named pipe".into())
        })?;

        Ok(DockerApiConnection::Windows(pipe_stream))
    }

    /// Sends an encoded request from the provided buffer and then reuses the same buffer to get a response.
    pub fn make_request(&mut self, buf: &mut Vec<u8>) -> Result<StatusCode> {
        // Send the request
        self.send_request(buf)?;

        // Extract the response meta like response code and headers that we need to process the body
        let mut temp_buf = [0u8; Self::DEFAULT_BUF_SIZE];
        let (response_code, parsing_mode) = self.read_response_meta(buf, &mut temp_buf)?;

        // Read the response body
        self.read_response_body(buf, parsing_mode, &mut temp_buf)?;

        Ok(response_code)
    }

    /// Sends the encoded HTTP request to the underlying socket.
    fn send_request(&mut self, buf: &[u8]) -> Result<()> {
        let socket = self.get_socket();

        socket.write_all(buf).map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                "failed to write a message to the Docker API socket".into()
            })
        })
    }

    /// Parses everything until the HTTP response body and returns a [StatusCode] and a [BodyParsingMode].
    ///
    /// # Note
    ///
    /// The provided buffer will be cleared and may contain some parts of or even a complete response body upon return.
    fn read_response_meta(&mut self, buf: &mut Vec<u8>, temp_buf: &mut [u8]) -> Result<(StatusCode, BodyParsingMode)> {
        buf.clear();
        let socket = self.get_socket();

        loop {
            let filled_bytes = socket.read(temp_buf).map_err(|e| {
                DockerError::from_io_error_with_description(e, || "failed to read an HTTP response".into())
            })?;
            buf.extend_from_slice(&temp_buf[..filled_bytes]);

            let mut headers = [EMPTY_HEADER; 10];
            let mut response = httparse::Response::new(&mut headers);

            let Ok(status) = response.parse(buf) else {
                return Err(DockerError::Other("failed to parse an HTTP response".into()));
            };
            let Status::Complete(body_start_idx) = status else {
                // We need more data
                continue;
            };

            let response_code = StatusCode::from_u16(
                response
                    .code
                    .ok_or_else(|| DockerError::Other("parsed an HTTP response without a code".into()))?,
            )
            .map_err(|_| DockerError::Other("got an invalid HTTP response code from Docker API".into()))?;

            let body_type = headers
                .iter()
                .find(|header| header.name == "Content-Length" || header.name == "Transfer-Encoding")
                .ok_or(DockerError::Other(
                    "missing both content-length and transfer-encoding headers in a response from Docker API".into(),
                ))?;

            // Prepare to read the body
            let read_body_bytes = buf.len() - body_start_idx;
            // Use the temp buf instead of the main one, as we are going to clear it before adding any body data
            let body_bytes = &temp_buf[filled_bytes - read_body_bytes..filled_bytes];

            let parsing_mode = match body_type.name {
                "Content-Length" => {
                    let raw_content_length = str::from_utf8(body_type.value).map_err(|_| {
                        DockerError::Other("invalid content-length header value in the Docker API response".into())
                    })?;
                    let content_length = raw_content_length
                        .parse::<usize>()
                        .map_err(|_| DockerError::Other("failed to parse content-length as a number".into()))?;

                    buf.clear();
                    // We can simply add the read body bytes to the buffer in this case, as they don't require any additional cleaning
                    buf.extend_from_slice(body_bytes);

                    BodyParsingMode::FixedLength(content_length)
                }
                "Transfer-Encoding" => {
                    buf.clear();

                    let mut chunk_processor = ChunkProcessor::new();
                    // Process the available body bytes
                    chunk_processor.process_available_data(body_bytes, buf)?;

                    BodyParsingMode::Chunks(chunk_processor)
                }
                // Should be unreachable
                _ => return Err(DockerError::Other("found an unexpected header".into())),
            };

            return Ok((response_code, parsing_mode));
        }
    }

    /// Reads the remaining resonse body from the underlying HTTP socket into the provided buffer.
    ///
    /// This function might not do any operations if we've already read the full body while parsing the HTTP request metadata.
    fn read_response_body(
        &mut self,
        buf: &mut Vec<u8>,
        mut parsing_mode: BodyParsingMode,
        temp_buf: &mut [u8],
    ) -> Result<()> {
        let socket = self.get_socket();

        let mut filled_bytes = 0;
        loop {
            // Do reading in processing in the reversed order to correctly handle cases when we read all body data while processing the metadata (i.e. headers)
            match &mut parsing_mode {
                BodyParsingMode::Chunks(chunk_processor) => {
                    if !chunk_processor.process_available_data(&temp_buf[..filled_bytes], buf)? {
                        break;
                    }
                }
                BodyParsingMode::FixedLength(body_length) => {
                    buf.extend_from_slice(&temp_buf[..filled_bytes]);
                    if buf.len() == *body_length {
                        break;
                    }
                }
            };

            filled_bytes = socket.read(temp_buf).map_err(|e| {
                DockerError::from_io_error_with_description(e, || "failed to read an HTTP response".into())
            })?;
        }

        Ok(())
    }

    fn get_socket(&mut self) -> &mut dyn ReadWrite {
        match self {
            #[cfg(unix)]
            DockerApiConnection::Unix(unix_sock) => unix_sock as &mut dyn ReadWrite,
            #[cfg(windows)]
            DockerApiConnection::Windows(npipe) => npipe as &mut dyn ReadWrite,
        }
    }
}
