//! Newline-delimited JSON framing, shared by the daemon and clients.

use std::io::{self, BufRead, Read};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{ErrorKind, MAX_MESSAGE_LEN, PROTOCOL_VERSION, Request, Response};

/// A message that could not be framed or parsed.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    /// The line exceeds [`MAX_MESSAGE_LEN`].
    #[error("message longer than {MAX_MESSAGE_LEN} bytes")]
    TooLong,
    /// The line is not the expected JSON.
    #[error("malformed message: {0}")]
    Json(#[from] serde_json::Error),
    /// Reading failed.
    #[error("read failed: {0}")]
    Io(#[from] io::Error),
}

/// Serializes `message` as one line, newline included.
pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, CodecError> {
    let mut line = serde_json::to_vec(message)?;
    line.push(b'\n');
    if line.len() > MAX_MESSAGE_LEN {
        return Err(CodecError::TooLong);
    }
    Ok(line)
}

/// Parses one line, with or without its newline.
pub fn decode<T: DeserializeOwned>(line: &[u8]) -> Result<T, CodecError> {
    if line.len() > MAX_MESSAGE_LEN {
        return Err(CodecError::TooLong);
    }
    Ok(serde_json::from_slice(line)?)
}

/// A request the daemon cannot act on; it converts into the [`Response`] to
/// send back.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RequestError {
    /// [`ErrorKind::Version`] or [`ErrorKind::BadRequest`].
    pub kind: ErrorKind,
    /// A message ready to show the user.
    pub message: String,
}

impl From<RequestError> for Response {
    fn from(err: RequestError) -> Self {
        Self::Error {
            kind: err.kind,
            message: err.message,
        }
    }
}

/// Parses a request, checking the protocol version before the command so a
/// client on another version gets a clear answer instead of a parse error.
pub fn decode_request(line: &[u8]) -> Result<Request, RequestError> {
    #[derive(serde::Deserialize)]
    struct Header {
        v: u32,
    }

    let bad = |err: CodecError| RequestError {
        kind: ErrorKind::BadRequest,
        message: err.to_string(),
    };
    let header: Header = decode(line).map_err(bad)?;
    if header.v != PROTOCOL_VERSION {
        return Err(RequestError {
            kind: ErrorKind::Version,
            message: format!(
                "The app and the OrcBlock helper are out of step (protocol {} vs {PROTOCOL_VERSION}); reinstall the helper",
                header.v
            ),
        });
    }
    decode(line).map_err(bad)
}

/// Reads one line into `buf` (cleared first), refusing lines longer than
/// [`MAX_MESSAGE_LEN`]. Returns `false` at end of stream.
pub fn read_line<R: BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> Result<bool, CodecError> {
    buf.clear();
    let limit = u64::try_from(MAX_MESSAGE_LEN).unwrap_or(u64::MAX);
    let read = reader.take(limit).read_until(b'\n', buf)?;
    if read == 0 {
        return Ok(false);
    }
    if buf.last() != Some(&b'\n') && buf.len() >= MAX_MESSAGE_LEN {
        return Err(CodecError::TooLong);
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
