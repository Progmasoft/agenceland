// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! The permission modes, and what each decides for an action of an agent.

use serde::{Deserialize, Serialize};

/// How much an agent may do without asking.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Every action that changes something, or reaches outside the session's worktree, asks.
    AlwaysAsk,
    /// Inside the sandbox, writes in the worktree and commands run without asking; the network,
    /// a push and anything outside the worktree ask. The default.
    #[default]
    Auto,
    /// Nothing asks and the sandbox is off; turning it on shows a warning.
    BypassPermissions,
}

/// Something an agent wants to do.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Action {
    /// Reading a file of the session's worktree.
    ReadInWorktree,
    /// Writing or deleting a file of the session's worktree.
    WriteInWorktree,
    /// Reading a file outside the worktree.
    ReadOutsideWorktree,
    /// Writing a file outside the worktree.
    WriteOutsideWorktree,
    /// Running a command in the worktree, inside the sandbox.
    RunCommand,
    /// Reaching a host on the network.
    Network {
        /// The host, as the request names it.
        host: String,
    },
    /// Pushing the session's branch, or opening a pull request.
    Push,
}

/// What a mode answers for an action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    /// It runs.
    Allow,
    /// The user is asked first.
    Ask,
}

/// What `mode` decides for `action`.
#[must_use]
pub fn decide(mode: Mode, action: &Action) -> Decision {
    match (mode, action) {
        (Mode::BypassPermissions, _)
        | (_, Action::ReadInWorktree)
        | (Mode::Auto, Action::WriteInWorktree | Action::RunCommand) => Decision::Allow,
        (
            Mode::AlwaysAsk | Mode::Auto,
            Action::WriteInWorktree
            | Action::ReadOutsideWorktree
            | Action::WriteOutsideWorktree
            | Action::RunCommand
            | Action::Network { .. }
            | Action::Push,
        ) => Decision::Ask,
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Decision, Mode, decide};

    fn every_action() -> Vec<Action> {
        vec![
            Action::ReadInWorktree,
            Action::WriteInWorktree,
            Action::ReadOutsideWorktree,
            Action::WriteOutsideWorktree,
            Action::RunCommand,
            Action::Network {
                host: "example.com".to_owned(),
            },
            Action::Push,
        ]
    }

    #[test]
    fn auto_lets_the_worktree_and_commands_through_and_asks_for_the_rest() {
        let allowed: Vec<bool> = every_action()
            .iter()
            .map(|action| decide(Mode::Auto, action) == Decision::Allow)
            .collect();
        assert_eq!(allowed, [true, true, false, false, true, false, false]);
    }

    #[test]
    fn always_ask_asks_for_all_but_reading_the_worktree() {
        for action in every_action() {
            let expected = if action == Action::ReadInWorktree {
                Decision::Allow
            } else {
                Decision::Ask
            };
            assert_eq!(decide(Mode::AlwaysAsk, &action), expected, "{action:?}");
        }
    }

    #[test]
    fn bypass_asks_for_nothing() {
        assert!(
            every_action()
                .iter()
                .all(|action| decide(Mode::BypassPermissions, action) == Decision::Allow)
        );
    }

    #[test]
    fn auto_is_the_default_and_modes_read_as_the_settings_write_them() {
        assert_eq!(Mode::default(), Mode::Auto);
        let mode: Mode = serde_json::from_str("\"bypassPermissions\"").unwrap();
        assert_eq!(mode, Mode::BypassPermissions);
        let action: Action = serde_json::from_str(r#"{"kind":"network","host":"a.b"}"#).unwrap();
        assert_eq!(
            action,
            Action::Network {
                host: "a.b".to_owned()
            }
        );
    }
}
