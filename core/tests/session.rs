// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! A whole session through the `agenceland-core` process: a Git repository, a worktree, the
//! permission modes, the file tools against the user's edits, closing, and replay.

use std::fs;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use agenceland_core::protocol::{codes, read_message};
use serde_json::{Value, json};

/// The core, started for one test.
struct Core {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
}

impl Core {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agenceland-core"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the core starts");
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
            next_id: 1,
        }
    }

    fn send(&mut self, body: &Value) {
        let bytes = serde_json::to_vec(body).unwrap();
        write!(self.input, "Content-Length: {}\r\n\r\n", bytes.len()).unwrap();
        self.input.write_all(&bytes).unwrap();
        self.input.flush().unwrap();
    }

    /// Calls `method` and returns the whole response.
    fn call(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let message = read_message(&mut self.output).unwrap().expect("a response");
        let response: Value = serde_json::from_slice(&message).unwrap();
        assert_eq!(response["id"], id);
        response
    }

    fn ok(&mut self, method: &str, params: &Value) -> Value {
        let response = self.call(method, params);
        assert!(response.get("error").is_none(), "{method}: {response}");
        response["result"].clone()
    }

    fn error_code(&mut self, method: &str, params: &Value) -> i64 {
        let response = self.call(method, params);
        response["error"]["code"]
            .as_i64()
            .unwrap_or_else(|| panic!("{method}: {response}"))
    }
}

fn scratch(name: &str) -> PathBuf {
    // Cargo's directory for the temporary files of integration tests, inside the target directory.
    let path =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn git(directory: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success(), "git {arguments:?}");
}

/// A repository with one commit holding `README.md`.
fn repository(root: &Path) -> PathBuf {
    let repository = root.join("repository");
    fs::create_dir_all(&repository).unwrap();
    git(&repository, &["init", "--quiet", "--initial-branch=main"]);
    // Git for Windows turns LF into CRLF in a checkout by default; the test compares the bytes.
    git(&repository, &["config", "core.autocrlf", "false"]);
    fs::write(repository.join("README.md"), "# Example\n").unwrap();
    git(&repository, &["add", "README.md"]);
    git(&repository, &["commit", "--quiet", "-m", "Start"]);
    repository
}

/// Auto: writes in the worktree run; the network and a push ask.
fn auto_decisions(core: &mut Core, session: &str) {
    for (action, decision) in [
        (json!({ "kind": "writeInWorktree" }), "allow"),
        (json!({ "kind": "network", "host": "example.com" }), "ask"),
        (json!({ "kind": "push" }), "ask"),
    ] {
        let params = json!({ "session": session, "action": action });
        assert_eq!(
            core.ok("permission/check", &params)["decision"],
            decision,
            "{action}"
        );
    }
}

/// The agent reads, the user edits, and the agent's write is refused until it reads again; paths
/// outside the worktree are refused.
fn file_tools(core: &mut Core, session: &str, worktree: &Path, repository: &Path) {
    let read = json!({ "session": session, "path": "README.md" });
    assert_eq!(core.ok("file/read", &read)["text"], "# Example\n");
    fs::write(
        worktree.join("README.md"),
        "# Example\n\nThe user's line.\n",
    )
    .unwrap();
    let write = json!({ "session": session, "path": "README.md", "text": "# Agent\n" });
    assert_eq!(core.error_code("file/write", &write), codes::STALE_FILE);
    core.ok("file/read", &read);
    core.ok("file/write", &write);
    assert_eq!(
        fs::read_to_string(worktree.join("README.md")).unwrap(),
        "# Agent\n"
    );
    // The user's checkout is untouched.
    assert_eq!(
        fs::read_to_string(repository.join("README.md")).unwrap(),
        "# Example\n"
    );
    for path in ["../repository/README.md", "/etc/hostname", ".git"] {
        let outside = json!({ "session": session, "path": path });
        assert_eq!(
            core.error_code("file/read", &outside),
            codes::OUTSIDE_WORKTREE,
            "{path}"
        );
    }
}

/// Replay shows the session from its record, in order, the refused write included.
fn replayed(core: &mut Core, data: &Path, session: &str) {
    let events = core.ok(
        "session/replay",
        &json!({ "dataDirectory": data, "session": session }),
    )["events"]
        .clone();
    let events = events.as_array().unwrap();
    let kinds: Vec<&str> = events
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.first(), Some(&"session.started"));
    assert_eq!(kinds.last(), Some(&"session.closed"));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "permission.checked")
            .count(),
        3
    );
    let calls: Vec<&Value> = events
        .iter()
        .filter(|event| event["kind"] == "tool.call")
        .collect();
    // read, the refused write, read again, the write.
    assert_eq!(calls[1]["data"]["error"]["code"], codes::STALE_FILE);
    assert_eq!(calls[3]["data"]["input"]["text"], "# Agent\n");
    let bad = json!({ "dataDirectory": data, "session": "../x" });
    assert_eq!(
        core.error_code("session/replay", &bad),
        codes::INVALID_PARAMS
    );
}

#[test]
fn a_session_from_start_to_replay() {
    let root = scratch("session");
    let repository = repository(&root);
    let data = root.join("Projects").join("example");
    let mut core = Core::start();
    assert_eq!(core.ok("initialize", &json!({}))["protocolVersion"], 1);

    let created = core.ok(
        "session/create",
        &json!({ "repository": repository, "dataDirectory": data, "mode": "auto" }),
    );
    let session = created["session"].as_str().unwrap().to_owned();
    let worktree = PathBuf::from(created["worktree"].as_str().unwrap());
    assert!(
        created["branch"]
            .as_str()
            .unwrap()
            .starts_with("agenceland/")
    );
    assert_eq!(
        fs::read_to_string(worktree.join("README.md")).unwrap(),
        "# Example\n"
    );

    auto_decisions(&mut core, &session);
    file_tools(&mut core, &session, &worktree, &repository);

    // A worktree with changes is not removed silently; once committed, the branch is kept.
    let close = json!({ "session": session });
    assert_eq!(core.error_code("session/close", &close), codes::FAILED);
    git(&worktree, &["commit", "--quiet", "-am", "Agent"]);
    assert_eq!(
        core.ok("session/close", &close)["branch"],
        created["branch"]
    );
    assert!(!worktree.exists());
    let read = json!({ "session": session, "path": "README.md" });
    assert_eq!(core.error_code("file/read", &read), codes::UNKNOWN_SESSION);

    replayed(&mut core, &data, &session);
    assert!(core.ok("shutdown", &json!(null)).is_null());
    assert!(core.child.wait().unwrap().success());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn always_ask_waits_for_approval_and_records_the_question() {
    let root = scratch("ask");
    let repository = repository(&root);
    let data = root.join("data");
    let mut core = Core::start();
    let session = core.ok(
        "session/create",
        &json!({ "repository": repository, "dataDirectory": data, "mode": "alwaysAsk" }),
    )["session"]
        .as_str()
        .unwrap()
        .to_owned();
    let write = json!({ "session": session, "path": "new.txt", "text": "x" });
    let response = core.call("file/write", &write);
    assert_eq!(response["error"]["code"], codes::APPROVAL_NEEDED);
    assert_eq!(
        response["error"]["data"]["action"]["kind"],
        "writeInWorktree"
    );
    core.ok(
        "file/write",
        &json!({ "session": session, "path": "new.txt", "text": "x", "approved": true }),
    );
    core.ok(
        "session/close",
        &json!({ "session": session, "discard": true }),
    );
    let events = core.ok(
        "session/replay",
        &json!({ "dataDirectory": data, "session": session }),
    )["events"]
        .clone();
    assert!(
        events
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "permission.asked")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn broken_requests_get_errors_and_the_core_goes_on() {
    let mut core = Core::start();
    assert_eq!(
        core.error_code("no/such", &json!({})),
        codes::METHOD_NOT_FOUND
    );
    assert_eq!(
        core.error_code("session/create", &json!({ "repository": 1 })),
        codes::INVALID_PARAMS
    );
    assert_eq!(
        core.error_code(
            "file/read",
            &json!({ "session": "s1", "path": "a", "extra": true })
        ),
        codes::INVALID_PARAMS
    );
    core.send(&json!({ "jsonrpc": "1.0", "id": 99, "method": "initialize" }));
    let message = read_message(&mut core.output).unwrap().unwrap();
    let response: Value = serde_json::from_slice(&message).unwrap();
    assert_eq!(response["error"]["code"], codes::INVALID_REQUEST);
    assert_eq!(core.ok("initialize", &json!({}))["name"], "agenceland-core");
    drop(core.input);
    assert!(core.child.wait().unwrap().success());
}
