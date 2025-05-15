use std::io::Write;

use http::Request;

use crate::{DockerError, Result};

pub fn encode_request<T: AsRef<[u8]>, O: Write>(req: &Request<T>, mut dst: O) -> Result<()> {
    // Write start line
    write!(
        dst,
        "{} {} {:?}\r\n",
        req.method(),
        req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/"),
        req.version()
    )
    .map_err(|e| DockerError::from_io_error_with_description(e, || "failed to encode the HTTP start line".into()))?;

    // Write headers
    for (name, value) in req.headers() {
        write!(
            dst,
            "{}: {}\r\n",
            name,
            value.to_str().map_err(|_| DockerError::Other(
                format!("invalid value for the '{name}' HTTP header: '{value:?}'").into()
            ))?
        )
        .map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                format!("failed to encode the '{name}' HTTP header").into()
            })
        })?;
    }

    // End of headers
    write!(dst, "\r\n").map_err(|e| {
        DockerError::from_io_error_with_description(e, || "failed to write an empty line after HTTP headers".into())
    })?;

    // Write body if it exists
    if !req.body().as_ref().is_empty() {
        dst.write_all(req.body().as_ref()).map_err(|e| {
            DockerError::from_io_error_with_description(e, || "failed to write an HTTP request body".into())
        })?;
    }

    Ok(())
}

pub fn process_available_http_chunks(src: &[u8], dst: &mut Vec<u8>) -> Result<Option<usize>> {
    let mut src = src;
    let mut current_chunk_size_end_pos = src.iter().position(|byte| *byte == b'\r');
    let mut latest_chunk_start_pos = 0;
    while let Some(chunk_size_end_pos) = current_chunk_size_end_pos {
        let raw_current_chunk_size = str::from_utf8(&src[..chunk_size_end_pos])
            .map_err(|_| DockerError::Other("got a non-utf8 chunk size when parsing a chunked HTTP response".into()))?;
        let chunk_size = usize::from_str_radix(raw_current_chunk_size, 16)
            .map_err(|_| DockerError::Other("failed to parse an HTTP chunk size".into()))?;

        if chunk_size == 0 {
            return Ok(None);
        }

        let chunk_start_pos = chunk_size_end_pos + 2 /* \r\n */;
        let chunk_end_pos = chunk_start_pos + chunk_size;
        let next_chunk_start_pos = chunk_end_pos + 2 /* \r\n */;

        if src.len() > next_chunk_start_pos {
            let full_chunk = &src[chunk_start_pos..chunk_end_pos];
            dst.extend_from_slice(full_chunk);
            latest_chunk_start_pos = next_chunk_start_pos;

            src = &src[next_chunk_start_pos..];
            current_chunk_size_end_pos = src.iter().position(|byte| *byte == b'\r');
            // Try to parse the next chunk as well
            continue;
        }

        // We need to pull more data before continuing
        break;
    }

    Ok(Some(latest_chunk_start_pos))
}
