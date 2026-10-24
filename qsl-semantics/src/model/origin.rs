// SPDX-License-Identifier: AGPL-3.0-or-later
//! Raw JSON values retained at the semantic-IR intake boundary.

/// JSON data whose authored shape must survive intake without normalization.
pub type JsonValue = serde_json::Value;
