// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-291: the typed cause of a `failed` record that settles a plugin-run
//! failure, one variant per `defect` of QSpec FR-462's failure table.

use serde_json::Value;

/// A plugin-run failure (FR-291): the `defect` of FR-462's failure table.
/// Only the two byte-limit variants carry a payload: the caller's configured
/// maximum and the byte count that crossed it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ToolFailure {
    /// `crash`.
    Crash,
    /// `nonzero-exit`.
    NonzeroExit,
    /// `malformed-frame`.
    MalformedFrame,
    /// `noncanonical-frame`.
    NoncanonicalFrame,
    /// `frame-length`: a frame longer than the configured maximum.
    FrameLength {
        /// The configured maximum, in bytes.
        limit: u64,
        /// The frame's length, in bytes.
        value: u64,
    },
    /// `total-bytes`: a run's running total over the configured maximum.
    TotalBytes {
        /// The configured maximum, in bytes.
        limit: u64,
        /// The run's running total, in bytes.
        value: u64,
    },
    /// `unknown-protocol`.
    UnknownProtocol,
    /// `unknown-frame-kind`.
    UnknownFrameKind,
    /// `envelope-member`.
    EnvelopeMember,
    /// `unknown-request-id`.
    UnknownRequestId,
    /// `malformed-body`.
    MalformedBody,
    /// `unrouted-item`.
    UnroutedItem,
    /// `unadvertised-kind`.
    UnadvertisedKind,
    /// `missing-terminal-record`.
    MissingTerminalRecord,
}

impl ToolFailure {
    /// The FR-462 `defect` spelling of this variant.
    pub const fn defect(self) -> &'static str {
        match self {
            Self::Crash => "crash",
            Self::NonzeroExit => "nonzero-exit",
            Self::MalformedFrame => "malformed-frame",
            Self::NoncanonicalFrame => "noncanonical-frame",
            Self::FrameLength { .. } => "frame-length",
            Self::TotalBytes { .. } => "total-bytes",
            Self::UnknownProtocol => "unknown-protocol",
            Self::UnknownFrameKind => "unknown-frame-kind",
            Self::EnvelopeMember => "envelope-member",
            Self::UnknownRequestId => "unknown-request-id",
            Self::MalformedBody => "malformed-body",
            Self::UnroutedItem => "unrouted-item",
            Self::UnadvertisedKind => "unadvertised-kind",
            Self::MissingTerminalRecord => "missing-terminal-record",
        }
    }

    /// The `limit` and `value` members of the two byte-limit variants.
    pub const fn byte_counts(self) -> Option<(u64, u64)> {
        match self {
            Self::FrameLength { limit, value } | Self::TotalBytes { limit, value } => {
                Some((limit, value))
            }
            _ => None,
        }
    }

    /// Read a serialized `tool-failure{defect}` cause back into its variant.
    /// `None` when the value is not exactly the outcome document's
    /// `tool-failure` cause (FR-286, `kind` spelling):
    /// another `kind`, an unknown `defect`, a missing or non-integer
    /// `limit` or `value` on a byte-limit defect, or any other member.
    #[ix_trace_rs::string_edge]
    pub fn read_cause(cause: &Value) -> Option<Self> {
        let members = cause.as_object()?;
        if members.get("kind")?.as_str()? != "tool-failure" {
            return None;
        }
        let defect = members.get("defect")?.as_str()?;
        let counts = || {
            Some((
                members.get("limit")?.as_u64()?,
                members.get("value")?.as_u64()?,
            ))
        };
        let (failure, expected_members) = match defect {
            "crash" => (Self::Crash, 2),
            "nonzero-exit" => (Self::NonzeroExit, 2),
            "malformed-frame" => (Self::MalformedFrame, 2),
            "noncanonical-frame" => (Self::NoncanonicalFrame, 2),
            "frame-length" => {
                let (limit, value) = counts()?;
                (Self::FrameLength { limit, value }, 4)
            }
            "total-bytes" => {
                let (limit, value) = counts()?;
                (Self::TotalBytes { limit, value }, 4)
            }
            "unknown-protocol" => (Self::UnknownProtocol, 2),
            "unknown-frame-kind" => (Self::UnknownFrameKind, 2),
            "envelope-member" => (Self::EnvelopeMember, 2),
            "unknown-request-id" => (Self::UnknownRequestId, 2),
            "malformed-body" => (Self::MalformedBody, 2),
            "unrouted-item" => (Self::UnroutedItem, 2),
            "unadvertised-kind" => (Self::UnadvertisedKind, 2),
            "missing-terminal-record" => (Self::MissingTerminalRecord, 2),
            _ => return None,
        };
        (members.len() == expected_members).then_some(failure)
    }
}
