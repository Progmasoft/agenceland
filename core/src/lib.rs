// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! The agent core of Agenceland.
//!
//! The core runs as a process of its own, and Agenceland and its `IntelliJ IDEA` plugin are two
//! front ends to it: they start `agenceland-core`, speak JSON-RPC 2.0 to it on its standard input
//! and output ([`protocol`]), and the logic of sessions is written once, here. A session works in
//! a Git worktree of its own ([`worktree`]), under a permission mode ([`permissions`]), through
//! file tools that keep it inside that worktree and off the user's edits ([`files`]), and leaves
//! a record of everything it did, which replay shows again ([`record`]). [`server`] has the
//! methods.

pub mod files;
pub mod permissions;
pub mod protocol;
pub mod record;
pub mod server;
pub mod worktree;
