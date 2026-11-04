// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-522 "Result identity": the identity of a temporal evidence result is an
//! equality of the members it was computed from, not a digest of a document.
//!
//! Each member is a tagged, length-prefixed byte string, so two requests
//! that agree in every member have one identity and two that differ in one
//! member have two, whatever the member's content (no member's bytes can
//! run into the next member's). The members that other requirements own
//! (the trace, the progress inputs, the retained instances, the progress
//! ledger entry, the relation) are encoded by their owners with
//! [`MemberEncoder`] and set here.

use std::collections::BTreeMap;

use qsl_package::{CheckedNodeId, ClockConfiguration};

use super::binding::AuthenticatedBinding;
use super::limits::{EvidenceCounter, TemporalEvidenceLimits};

/// A member of the identity list of FR-522.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityMember {
    /// The checked clause identity: the package identity and the
    /// declaration.
    Clause,
    /// The authenticated binding.
    Authenticated,
    /// The clock cover, `Exact` or `Counter { uncertainty }`.
    Cover,
    /// The stamps, valuations and order keys, or the positions, of the
    /// trace.
    Trace,
    /// The progress inputs: watermark, completeness, scope closure,
    /// surrounding-execution closure and lower boundary.
    Progress,
    /// The thirteen ceilings of FR-523.
    Ceilings,
    /// The meter's work-unit ceiling.
    MeterCeiling,
    /// The trigger identity of an evidence result.
    Trigger,
    /// The retained-instance entries of the request's `EvidenceLedger` for
    /// the obligation (FR-524). The ledger's latest-results table is not a
    /// member.
    RetainedInstances,
    /// The progress-ledger entry for the binding (FR-502).
    ProgressLedger,
    /// The relation of an evidence result (FR-526).
    Relation,
    /// The ordered deliveries of an evidence request (FR-524).
    Deliveries,
    /// The ordered origins of an evidence request (FR-524).
    Origins,
    /// The identity of the position the instance activates at.
    Anchor,
    /// The retained records the request reports as evicted.
    Evicted,
    /// The correspondence, every member of its `EvidenceRef`.
    Correspondence,
    /// The occurrence of the clause's declaration the call assesses.
    Occurrence,
}

impl IdentityMember {
    /// Whether the member is part of `input_identity`. The relation, the
    /// whole `EvidenceLedger` and the progress ledger are not: a repeat of
    /// an earlier request's evidence has that request's `input_identity`
    /// whatever ledger it carries.
    pub const fn is_input(self) -> bool {
        match self {
            Self::Clause
            | Self::Authenticated
            | Self::Cover
            | Self::Trace
            | Self::Progress
            | Self::Ceilings
            | Self::MeterCeiling
            | Self::Trigger
            | Self::Deliveries
            | Self::Origins
            | Self::Anchor
            | Self::Evicted
            | Self::Correspondence
            | Self::Occurrence => true,
            Self::RetainedInstances | Self::ProgressLedger | Self::Relation => false,
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Clause => 1,
            Self::Authenticated => 2,
            Self::Cover => 3,
            Self::Trace => 4,
            Self::Progress => 5,
            Self::Ceilings => 6,
            Self::MeterCeiling => 7,
            Self::Trigger => 8,
            Self::RetainedInstances => 9,
            Self::ProgressLedger => 10,
            Self::Relation => 11,
            Self::Deliveries => 12,
            Self::Origins => 13,
            Self::Anchor => 14,
            Self::Evicted => 15,
            Self::Correspondence => 16,
            Self::Occurrence => 17,
        }
    }
}

/// Builds the byte string of one member: unsigned integers, flags and
/// length-prefixed byte strings, so a member's encoding is injective in its
/// values.
#[derive(Clone, Debug, Default)]
pub struct MemberEncoder {
    bytes: Vec<u8>,
}

impl MemberEncoder {
    /// An empty member.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an unsigned integer.
    #[must_use]
    pub fn u64(mut self, value: u64) -> Self {
        self.bytes.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// Append a flag.
    #[must_use]
    pub fn flag(mut self, value: bool) -> Self {
        self.bytes.push(u8::from(value));
        self
    }

    /// Append a length-prefixed byte string.
    #[must_use]
    pub fn bytes(mut self, value: &[u8]) -> Self {
        self.bytes
            .extend_from_slice(&(value.len() as u64).to_be_bytes());
        self.bytes.extend_from_slice(value);
        self
    }

    /// Append a length-prefixed text.
    #[must_use]
    pub fn text(self, value: &str) -> Self {
        self.bytes(value.as_bytes())
    }

    /// Append an optional value: absent and present never encode alike.
    #[must_use]
    pub fn option<T>(self, value: Option<&T>, encode: impl FnOnce(Self, &T) -> Self) -> Self {
        match value {
            None => self.flag(false),
            Some(value) => encode(self.flag(true), value),
        }
    }

    /// The finished member.
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// The identity of a result: equal members give equal identities and a
/// change of any bound member gives a distinct one (FR-522).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResultIdentity(Box<[u8]>);

impl ResultIdentity {
    /// The identity's bytes, the encoding of its members.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The identity over the evidence members only (FR-522).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InputIdentity(Box<[u8]>);

/// Why a member cannot be encoded.
#[derive(Debug, thiserror::Error)]
#[error("an identity member does not encode: {0}")]
pub struct MemberFault(#[from] serde_json::Error);

/// The members of one request, from which both identities are computed.
#[derive(Clone, Debug, Default)]
pub struct IdentityBuilder {
    members: BTreeMap<IdentityMember, Vec<u8>>,
}

impl IdentityBuilder {
    /// A builder with no member set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `member` to `bytes`, replacing an earlier value.
    #[must_use]
    pub fn member(mut self, member: IdentityMember, bytes: Vec<u8>) -> Self {
        self.members.insert(member, bytes);
        self
    }

    /// Set the checked clause identity: the package identity and the
    /// declaration node.
    ///
    /// # Errors
    ///
    /// [`MemberFault`] if the node identity does not encode.
    pub fn clause(self, declaration: &CheckedNodeId) -> Result<Self, MemberFault> {
        let bytes = MemberEncoder::new()
            .text(&declaration.package)
            .bytes(&serde_json::to_vec(declaration)?)
            .finish();
        Ok(self.member(IdentityMember::Clause, bytes))
    }

    /// Set the authenticated binding.
    ///
    /// # Errors
    ///
    /// [`MemberFault`] if the binding does not encode.
    pub fn authenticated(self, binding: &AuthenticatedBinding) -> Result<Self, MemberFault> {
        let bytes = MemberEncoder::new()
            .text(&binding.package)
            .bytes(&serde_json::to_vec(&binding.declaration)?)
            .bytes(&serde_json::to_vec(&binding.profile)?)
            .text(&binding.clock_name)
            .bytes(&clock_bytes(&binding.clock)?)
            .finish();
        Ok(self.member(IdentityMember::Authenticated, bytes))
    }

    /// Set the thirteen ceilings of FR-523.
    #[must_use]
    pub fn ceilings(self, limits: &TemporalEvidenceLimits) -> Self {
        let bytes = EvidenceCounter::ALL
            .into_iter()
            .fold(MemberEncoder::new(), |encoder, counter| {
                encoder.u64(limits.ceiling(counter))
            })
            .finish();
        self.member(IdentityMember::Ceilings, bytes)
    }

    /// Set the meter's work-unit ceiling.
    #[must_use]
    pub fn meter_ceiling(self, work_units: u64) -> Self {
        let bytes = MemberEncoder::new().u64(work_units).finish();
        self.member(IdentityMember::MeterCeiling, bytes)
    }

    fn encode(&self, input_only: bool) -> Box<[u8]> {
        let mut out = Vec::new();
        for (member, bytes) in &self.members {
            if input_only && !member.is_input() {
                continue;
            }
            out.push(member.tag());
            out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        out.into_boxed_slice()
    }

    /// The identity of the result over every member set.
    pub fn result_identity(&self) -> ResultIdentity {
        ResultIdentity(self.encode(false))
    }

    /// The identity over the evidence members only.
    pub fn input_identity(&self) -> InputIdentity {
        InputIdentity(self.encode(true))
    }
}

fn clock_bytes(clock: &ClockConfiguration) -> Result<Vec<u8>, MemberFault> {
    Ok(serde_json::to_vec(clock)?)
}
