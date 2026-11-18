// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! `agenceland-core`: reads JSON-RPC requests on standard input, answers on standard output, and
//! writes nothing else there. It stops after `shutdown`, or when its input ends.

use std::io::{self, BufReader};
use std::process::ExitCode;

use agenceland_core::protocol::{self, RpcError, codes};
use agenceland_core::server::Server;
use serde_json::Value;

fn main() -> ExitCode {
    let mut input = BufReader::new(io::stdin().lock());
    let mut output = io::stdout().lock();
    let mut server = Server::default();
    loop {
        let message = match protocol::read_message(&mut input) {
            Ok(Some(message)) => message,
            Ok(None) => return ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("agenceland-core: {error}");
                return ExitCode::FAILURE;
            }
        };
        let answer = match serde_json::from_slice::<Value>(&message) {
            Err(error) => Some(protocol::response(
                &Value::Null,
                Err(RpcError::new(codes::PARSE_ERROR, error.to_string())),
            )),
            // JSON, but not a request: answered with the id it carries, when it carries one.
            Ok(value) => {
                let id = value.get("id").cloned().unwrap_or(Value::Null);
                match serde_json::from_value(value) {
                    Ok(request) => server.handle(request),
                    Err(error) => Some(protocol::response(
                        &id,
                        Err(RpcError::new(codes::INVALID_REQUEST, error.to_string())),
                    )),
                }
            }
        };
        if let Some(answer) = answer {
            if let Err(error) = protocol::write_message(&mut output, &answer) {
                eprintln!("agenceland-core: {error}");
                return ExitCode::FAILURE;
            }
        }
        if server.is_stopping() {
            return ExitCode::SUCCESS;
        }
    }
}
