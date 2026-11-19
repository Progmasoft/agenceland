// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! The session record: every event of a session, one JSON object per line, in order. Replay reads
//! it back and shows the session as it was, calling no model and running no tool.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One event of a session.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Event {
    /// Its place in the session, from 1.
    pub seq: u64,
    /// When it happened, in milliseconds since 1970 (UTC).
    pub time: u64,
    /// What it is, such as `session.started`, `tool.call` or `permission.asked`.
    pub kind: String,
    /// What it carries: the input and output of a tool call, the action of a permission.
    pub data: Value,
}

/// The record of one session, open for appending.
#[derive(Debug)]
pub struct Record {
    path: PathBuf,
    file: File,
    next: u64,
}

impl Record {
    /// Starts the record at `path`; a record that is there already is refused, so that one is
    /// never mixed into another.
    ///
    /// # Errors
    ///
    /// When the directory cannot be made or the file is there or cannot be made.
    pub fn create(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            path: path.to_owned(),
            file,
            next: 1,
        })
    }

    /// Where the record is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends an event of `kind` with `data`, and writes it out before returning.
    ///
    /// # Errors
    ///
    /// When it cannot be written.
    pub fn append(&mut self, kind: &str, data: Value) -> io::Result<Event> {
        let event = Event {
            seq: self.next,
            time: now(),
            kind: kind.to_owned(),
            data,
        };
        let mut line = serde_json::to_vec(&event)?;
        line.push(b'\n');
        self.file.write_all(&line)?;
        self.file.flush()?;
        self.next += 1;
        Ok(event)
    }
}

/// Reads the record at `path` for replay.
///
/// # Errors
///
/// When it cannot be read, a line is not an event, or the events are not numbered 1, 2, 3 and
/// so on: a record with a gap or a line out of order was changed, and replay would not show the
/// session as it was.
pub fn replay(path: &Path) -> io::Result<Vec<Event>> {
    let mut events = Vec::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(&line).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {}: {error}", index + 1),
            )
        })?;
        let expected = events.len() as u64 + 1;
        if event.seq != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "line {}: event {} where {expected} should be",
                    index + 1,
                    event.seq
                ),
            ));
        }
        events.push(event);
    }
    Ok(events)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| {
            u64::try_from(time.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;

    use super::{Record, replay};

    #[test]
    fn events_come_back_in_order() {
        // Below the crate's own target directory, not the temporary directory other users share.
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-scratch");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("record-{}.jsonl", std::process::id()));
        let _ = fs::remove_file(&path);
        let mut record = Record::create(&path).unwrap();
        record
            .append("session.started", json!({ "branch": "b" }))
            .unwrap();
        record
            .append("tool.call", json!({ "tool": "file/read" }))
            .unwrap();
        assert!(Record::create(&path).is_err());
        let events = replay(&path).unwrap();
        assert_eq!(events.iter().map(|e| e.seq).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(events[1].kind, "tool.call");

        let text = fs::read_to_string(&path).unwrap();
        let swapped: Vec<&str> = text.lines().rev().collect();
        fs::write(&path, swapped.join("\n")).unwrap();
        assert!(replay(&path).is_err());
        let _ = fs::remove_file(&path);
    }
}
