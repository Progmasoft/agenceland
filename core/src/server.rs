// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! The methods of the core, and the sessions they work on.
//!
//! | Method | Parameters | Result |
//! | --- | --- | --- |
//! | `initialize` | none | `name`, `version`, `protocolVersion` |
//! | `session/create` | `repository`, `dataDirectory`, `mode` | `session`, `worktree`, `branch`, `startCommit` |
//! | `permission/check` | `session`, `action` | `decision` |
//! | `file/read` | `session`, `path` | `text` |
//! | `file/write` | `session`, `path`, `text`, `approved` | `{}` |
//! | `session/close` | `session`, `discard` | `branch` |
//! | `session/replay` | `dataDirectory`, `session` | `events` |
//! | `shutdown` | none | `null`; the core then exits |
//!
//! `dataDirectory` is `Projects/<key>` of the user's Agenceland directory, which the front end
//! computes: worktrees go below `worktrees/`, records below `sessions/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::files::{FileError, Files};
use crate::permissions::{Action, Decision, Mode, decide};
use crate::protocol::{Request, RpcError, codes};
use crate::record::{Record, replay};
use crate::worktree::{self, SessionWorktree};

/// The version of the protocol; a front end that speaks another refuses to go on.
pub const PROTOCOL_VERSION: u32 = 1;

/// One open session.
#[derive(Debug)]
struct Session {
    repository: PathBuf,
    worktree: SessionWorktree,
    mode: Mode,
    files: Files,
    record: Record,
}

/// The core: its open sessions, and whether it was asked to stop.
#[derive(Debug, Default)]
pub struct Server {
    sessions: HashMap<String, Session>,
    stopping: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateParams {
    repository: PathBuf,
    data_directory: PathBuf,
    #[serde(default)]
    mode: Mode,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CheckParams {
    session: String,
    action: Action,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadParams {
    session: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WriteParams {
    session: String,
    path: String,
    text: String,
    #[serde(default)]
    approved: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CloseParams {
    session: String,
    #[serde(default)]
    discard: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReplayParams {
    data_directory: PathBuf,
    session: String,
}

impl Server {
    /// Whether `shutdown` was called.
    #[must_use]
    pub fn is_stopping(&self) -> bool {
        self.stopping
    }

    /// Answers `request`: `None` for a notification, which gets no response.
    pub fn handle(&mut self, request: Request) -> Option<Value> {
        let id = request.id.clone();
        let outcome = if request.jsonrpc == "2.0" {
            self.call(&request.method, request.params)
        } else {
            Err(RpcError::new(
                codes::INVALID_REQUEST,
                "jsonrpc must be \"2.0\"",
            ))
        };
        id.map(|id| crate::protocol::response(&id, outcome))
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        match method {
            "initialize" => Ok(json!({
                "name": "agenceland-core",
                "version": env!("CARGO_PKG_VERSION"),
                "protocolVersion": PROTOCOL_VERSION,
            })),
            "session/create" => self.create(parse(params)?),
            "session/replay" => replay_session(&parse(params)?),
            "shutdown" => {
                self.stopping = true;
                Ok(Value::Null)
            }
            _ => self.call_in_session(method, params),
        }
    }

    /// The methods that act on an open session.
    fn call_in_session(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        match method {
            "permission/check" => self.check(&parse(params)?),
            "file/read" => self.read(&parse(params)?),
            "file/write" => self.write(&parse(params)?),
            "session/close" => self.close(&parse(params)?),
            _ => Err(RpcError::new(
                codes::METHOD_NOT_FOUND,
                format!("no method {method}"),
            )),
        }
    }

    fn create(&mut self, params: CreateParams) -> Result<Value, RpcError> {
        let id = new_session_id();
        let worktree = worktree::start(
            &params.repository,
            &params.data_directory.join("worktrees"),
            &id,
        )
        .map_err(failed)?;
        let files = Files::new(&worktree.path).map_err(file_error)?;
        let mut record =
            Record::create(&record_path(&params.data_directory, &id)).map_err(io_failed)?;
        record
            .append(
                "session.started",
                json!({
                    "core": env!("CARGO_PKG_VERSION"),
                    "branch": worktree.branch,
                    "startCommit": worktree.start_commit,
                    "worktree": worktree.path,
                    "mode": params.mode,
                }),
            )
            .map_err(io_failed)?;
        let result = json!({
            "session": id,
            "worktree": worktree.path,
            "branch": worktree.branch,
            "startCommit": worktree.start_commit,
        });
        self.sessions.insert(
            id,
            Session {
                repository: params.repository,
                worktree,
                mode: params.mode,
                files,
                record,
            },
        );
        Ok(result)
    }

    fn check(&mut self, params: &CheckParams) -> Result<Value, RpcError> {
        let session = self.session(&params.session)?;
        let decision = decide(session.mode, &params.action);
        session
            .record
            .append(
                "permission.checked",
                json!({ "action": params.action, "decision": decision }),
            )
            .map_err(io_failed)?;
        Ok(json!({ "decision": decision }))
    }

    fn read(&mut self, params: &ReadParams) -> Result<Value, RpcError> {
        let session = self.session(&params.session)?;
        let outcome = session.files.read(&params.path).map_err(file_error);
        let input = json!({ "path": params.path });
        record_call(
            session,
            "file/read",
            &input,
            outcome.as_ref().map(|text| json!({ "text": text })),
        )?;
        outcome.map(|text| json!({ "text": text }))
    }

    fn write(&mut self, params: &WriteParams) -> Result<Value, RpcError> {
        let session = self.session(&params.session)?;
        let action = Action::WriteInWorktree;
        if decide(session.mode, &action) == Decision::Ask && !params.approved {
            session
                .record
                .append(
                    "permission.asked",
                    json!({ "action": action, "path": params.path }),
                )
                .map_err(io_failed)?;
            return Err(RpcError::new(
                codes::APPROVAL_NEEDED,
                "the permission mode asks the user first",
            )
            .with_data(json!({ "action": action })));
        }
        let outcome = session
            .files
            .write(&params.path, &params.text)
            .map_err(file_error);
        let input =
            json!({ "path": params.path, "text": params.text, "approved": params.approved });
        record_call(
            session,
            "file/write",
            &input,
            outcome.as_ref().map(|()| json!({})),
        )?;
        outcome.map(|()| json!({}))
    }

    fn close(&mut self, params: &CloseParams) -> Result<Value, RpcError> {
        let session = self.session(&params.session)?;
        worktree::close(&session.repository, &session.worktree, params.discard).map_err(failed)?;
        let branch = session.worktree.branch.clone();
        session
            .record
            .append(
                "session.closed",
                json!({ "branch": branch, "discard": params.discard }),
            )
            .map_err(io_failed)?;
        self.sessions.remove(&params.session);
        Ok(json!({ "branch": branch }))
    }

    fn session(&mut self, id: &str) -> Result<&mut Session, RpcError> {
        self.sessions
            .get_mut(id)
            .ok_or_else(|| RpcError::new(codes::UNKNOWN_SESSION, format!("no open session {id}")))
    }
}

/// Records a tool call with its input and its output or error.
fn record_call(
    session: &mut Session,
    tool: &str,
    input: &Value,
    outcome: Result<Value, &RpcError>,
) -> Result<(), RpcError> {
    let data = match outcome {
        Ok(output) => json!({ "tool": tool, "input": input, "output": output }),
        Err(error) => json!({ "tool": tool, "input": input, "error": error }),
    };
    session
        .record
        .append("tool.call", data)
        .map(drop)
        .map_err(io_failed)
}

fn replay_session(params: &ReplayParams) -> Result<Value, RpcError> {
    if !is_session_id(&params.session) {
        return Err(RpcError::new(
            codes::INVALID_PARAMS,
            format!("'{}' is not a session id", params.session),
        ));
    }
    let events =
        replay(&record_path(&params.data_directory, &params.session)).map_err(io_failed)?;
    Ok(json!({ "events": events }))
}

fn record_path(data_directory: &Path, id: &str) -> PathBuf {
    data_directory.join("sessions").join(format!("{id}.jsonl"))
}

/// A new session id: the time in milliseconds, the process and a counter, in hexadecimal.
fn new_session_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_millis());
    format!(
        "s{millis:x}{:x}{:x}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// Session ids are made of lower-case letters and digits, so one never names a path elsewhere.
fn is_session_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

fn parse<T: DeserializeOwned>(params: Value) -> Result<T, RpcError> {
    serde_json::from_value(params)
        .map_err(|error| RpcError::new(codes::INVALID_PARAMS, error.to_string()))
}

fn file_error(error: FileError) -> RpcError {
    match error {
        FileError::Outside(message) => RpcError::new(codes::OUTSIDE_WORKTREE, message),
        FileError::Stale(message) => RpcError::new(codes::STALE_FILE, message),
        FileError::NotText(message) | FileError::Io(message) => {
            RpcError::new(codes::FAILED, message)
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn failed(message: String) -> RpcError {
    RpcError::new(codes::FAILED, message)
}

#[allow(clippy::needless_pass_by_value)]
fn io_failed(error: std::io::Error) -> RpcError {
    RpcError::new(codes::FAILED, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::is_session_id;

    #[test]
    fn session_ids_name_no_paths() {
        assert!(is_session_id("s19a3f0c2b8e1"));
        for bad in ["", "../x", "S1", "a/b", "a.b", &"a".repeat(65)] {
            assert!(!is_session_id(bad), "{bad}");
        }
    }
}
