use std::io::Write;

use http::Request;

use crate::{DockerError, Result};

pub fn encode_request<T: AsRef<[u8]>, O: Write>(
    req: &Request<T>,
    mut dst: O,
) -> Result<()> {
    // Write start line
    write!(
        dst,
        "{} {} {:?}\r\n",
        req.method(),
        req.uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/"),
        req.version()
    )
    .map_err(|e| {
        DockerError::from_io_error_with_description(e, || {
            "failed to encode the HTTP start line".into()
        })
    })?;

    // Write headers
    for (name, value) in req.headers() {
        write!(
            dst,
            "{}: {}\r\n",
            name,
            value.to_str().map_err(|_| DockerError::Other(
                format!(
                    "invalid value for the '{name}' HTTP header: '{value:?}'"
                )
                .into()
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
        DockerError::from_io_error_with_description(e, || {
            "failed to write an empty line after HTTP headers".into()
        })
    })?;

    // Write body if it exists
    if !req.body().as_ref().is_empty() {
        dst.write_all(req.body().as_ref()).map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                "failed to write an HTTP request body".into()
            })
        })?;
    }

    Ok(())
}

pub fn add_base16_digit(left: usize, right: u8) -> usize {
    (left << 4) | right as usize
}

pub fn hex_digit_to_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
