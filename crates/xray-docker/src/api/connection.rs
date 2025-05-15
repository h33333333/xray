use std::io::{Read, Write};
#[cfg(windows)]
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use http::StatusCode;
use httparse::{EMPTY_HEADER, Status};

use super::util::process_available_http_chunks;
use crate::{DockerError, Result};

trait ReadWrite: Read + Write {}

impl<T> ReadWrite for T where T: Read + Write {}

pub enum DockerApiConnection {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    Windows(TcpStream), // TODO: change to a named pipe
}

enum BodyParsingMode {
    Chunks,
    FixedLength(usize),
}

impl DockerApiConnection {
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
    pub fn connect<R: AsRef<str>>(addr: R) -> Result<Self> {
        todo!("not implemented yet")
    }

    // TODO: this needs to be optimized/reworked/checked for edge cases
    /// Sends an encoded request from the provided buffer and then reuses the same buffer to get a response.
    pub fn send(&mut self, buf: &mut Vec<u8>) -> Result<StatusCode> {
        let socket = match self {
            #[cfg(unix)]
            DockerApiConnection::Unix(unix_sock) => unix_sock as &mut dyn ReadWrite,
            #[cfg(windows)]
            DockerApiConnection::Windows(npipe) => npipe as &mut dyn ReadWrite,
        };

        socket.write_all(buf).map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                "failed to write a message to the Docker API socket".into()
            })
        })?;

        // Prepare to read the response
        buf.clear();

        let mut response_code = Option::None;
        let mut body_parsing_mode = Option::None;
        let mut current_chunk = Vec::new();

        let mut temp_buf = [0u8; 1024];
        let mut bytes_read = 0;
        loop {
            let filled_bytes = socket.read(&mut temp_buf).map_err(|e| {
                DockerError::from_io_error_with_description(e, || "failed to read an HTTP response".into())
            })?;
            bytes_read += filled_bytes;

            let mut should_extend = true;
            // Parse everything until the body if not done already
            if response_code.is_none() {
                buf.extend_from_slice(&temp_buf[..filled_bytes]);

                let mut headers = [EMPTY_HEADER; 10];
                let mut response = httparse::Response::new(&mut headers);
                match response.parse(buf) {
                    Ok(status) => {
                        if let Status::Complete(body_start_idx) = status {
                            response_code = Some(
                                StatusCode::from_u16(response.code.ok_or_else(|| {
                                    DockerError::Other("parsed an HTTP response without a code".into())
                                })?)
                                .map_err(|_| {
                                    DockerError::Other("got an invalid HTTP response code from Docker API".into())
                                })?,
                            );

                            let body_type = headers.iter()
                                .find(|header| header.name == "Content-Length" || header.name == "Transfer-Encoding")
                                . ok_or(DockerError::Other("missing both content-length and transfer-encoding headers in a response from Docker API".into()))?;

                            // Prepare to read the body
                            let read_body_bytes = bytes_read - body_start_idx;
                            let body_bytes = &temp_buf[filled_bytes - read_body_bytes..filled_bytes];

                            match body_type.name {
                                "Content-Length" => {
                                    let raw_content_length = str::from_utf8(body_type.value).map_err(|_| {
                                        DockerError::Other(
                                            "invalid content-length header value in the Docker API response".into(),
                                        )
                                    })?;
                                    let content_length = raw_content_length.parse::<usize>().map_err(|_| {
                                        DockerError::Other("failed to parse content-length as a number".into())
                                    })?;

                                    // We can simply add the read body bytes to the buffer in this case, as they don't require any additional cleaning
                                    buf.clear();
                                    buf.extend_from_slice(body_bytes);

                                    should_extend = false;
                                    body_parsing_mode = Some(BodyParsingMode::FixedLength(content_length));
                                }
                                "Transfer-Encoding" => {
                                    current_chunk.extend_from_slice(body_bytes);
                                    buf.clear();

                                    should_extend = false;
                                    body_parsing_mode = Some(BodyParsingMode::Chunks);
                                }
                                // Should be unreachable
                                _ => return Err(DockerError::Other("found an unexpected header".into())),
                            };
                        }
                    }
                    Err(_) => return Err(DockerError::Other("failed to parse an HTTP response".into())),
                }
            }

            if let Some(ref parsing_mode) = body_parsing_mode {
                match parsing_mode {
                    BodyParsingMode::Chunks => {
                        if should_extend {
                            current_chunk.extend_from_slice(&temp_buf[..filled_bytes]);
                        }

                        let next_chunk_idx = process_available_http_chunks(&current_chunk, buf)?;
                        match next_chunk_idx {
                            Some(next_chunk_idx) => current_chunk.drain(..next_chunk_idx),
                            None => break,
                        };
                    }
                    BodyParsingMode::FixedLength(body_length) => {
                        if should_extend {
                            buf.extend_from_slice(&temp_buf[..filled_bytes]);
                        }

                        if buf.len() == *body_length {
                            break;
                        }
                    }
                };
            }
        }

        Ok(response_code.expect("must be present at this point"))
    }
}
