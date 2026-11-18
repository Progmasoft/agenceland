// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! JSON-RPC 2.0 in the frames the Language Server Protocol uses: a `Content-Length` header, an
//! empty line, and that many bytes of JSON. Agenceland and its `IntelliJ IDEA` plugin both speak it
//! to the core on its standard input and output.

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The largest message the core reads; a longer one is refused before it is read.
pub const MAX_MESSAGE: usize = 16 * 1024 * 1024;

/// A request, or a notification when it has no `id`.
#[derive(Debug, Deserialize)]
pub struct Request {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// The id the response carries; none for a notification.
    #[serde(default)]
    pub id: Option<Value>,
    /// The method.
    pub method: String,
    /// Its parameters.
    #[serde(default)]
    pub params: Value,
}

/// An error, as JSON-RPC carries it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RpcError {
    /// The code: JSON-RPC's own below -32000, the core's from -32001 down.
    pub code: i64,
    /// What went wrong, for a person.
    pub message: String,
    /// What a front end needs to act on it, such as the action to ask the user about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// The codes of the core's errors.
pub mod codes {
    /// The message is not JSON.
    pub const PARSE_ERROR: i64 = -32700;
    /// The message is not a JSON-RPC 2.0 request.
    pub const INVALID_REQUEST: i64 = -32600;
    /// The core has no such method.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// The parameters are not what the method takes.
    pub const INVALID_PARAMS: i64 = -32602;
    /// The permission mode asks the user first; send the request again with `approved`.
    pub const APPROVAL_NEEDED: i64 = -32001;
    /// The file changed since the agent last read it; read it again.
    pub const STALE_FILE: i64 = -32002;
    /// The path leaves the session's worktree.
    pub const OUTSIDE_WORKTREE: i64 = -32003;
    /// No session has that id.
    pub const UNKNOWN_SESSION: i64 = -32004;
    /// Git, the file system or the session record failed.
    pub const FAILED: i64 = -32005;
}

impl RpcError {
    /// An error with `code` and `message`, and no data.
    #[must_use]
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// The same error with `data`.
    #[must_use]
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Reads one message: `Ok(None)` at the end of the input.
///
/// # Errors
///
/// A header that is not `Content-Length`, a length over [`MAX_MESSAGE`], or the input ending
/// inside a message.
pub fn read_message(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return if length.is_none() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the input ended in a header",
                ))
            };
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid(format!("'{line}' is not a header")))?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            let parsed: usize = value
                .trim()
                .parse()
                .map_err(|_| invalid(format!("'{}' is not a length", value.trim())))?;
            if parsed > MAX_MESSAGE {
                return Err(invalid(format!(
                    "a message of {parsed} bytes is over {MAX_MESSAGE}"
                )));
            }
            length = Some(parsed);
        }
    }
    let length = length.ok_or_else(|| invalid("a message has no Content-Length".to_owned()))?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

/// Writes `message` in a frame and flushes it.
///
/// # Errors
///
/// Whatever writing to `output` fails with.
pub fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

/// The response to request `id`: its result, or its error.
#[must_use]
pub fn response(id: &Value, outcome: Result<Value, RpcError>) -> Value {
    match outcome {
        Ok(result) => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{MAX_MESSAGE, read_message, write_message};

    #[test]
    fn a_message_goes_round() {
        let mut out = Vec::new();
        write_message(&mut out, &serde_json::json!({ "a": 1 })).unwrap();
        assert_eq!(out, b"Content-Length: 7\r\n\r\n{\"a\":1}");
        let mut input = Cursor::new(out);
        assert_eq!(read_message(&mut input).unwrap().unwrap(), b"{\"a\":1}");
        assert!(read_message(&mut input).unwrap().is_none());
    }

    #[test]
    fn other_headers_are_skipped_and_case_does_not_matter() {
        let mut input =
            Cursor::new(b"content-type: application/json\r\nCONTENT-LENGTH: 2\r\n\r\n{}".to_vec());
        assert_eq!(read_message(&mut input).unwrap().unwrap(), b"{}");
    }

    #[test]
    fn broken_frames_are_refused() {
        for bad in [
            format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE + 1),
            "Content-Length: many\r\n\r\n".to_owned(),
            "no colon here\r\n\r\n".to_owned(),
            "Content-Type: x\r\n\r\n".to_owned(),
            "Content-Length: 10\r\n\r\n{}".to_owned(),
            "Content-Length: 2\r\n".to_owned(),
        ] {
            let mut input = Cursor::new(bad.clone().into_bytes());
            assert!(read_message(&mut input).is_err(), "{bad:?}");
        }
    }
}
