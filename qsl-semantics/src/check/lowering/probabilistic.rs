// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-498: the `claim`/`probabilistic_claim` node each checked probabilistic
//! claim lowers to, the declaration node QSpec FR-322's
//! `checked_term_data.probabilistic_claims` row names.
//!
//! The node carries the claim's identity and nothing of its formula: its body
//! is the aggregate of the claim's `name` and its `normal_form` (ADR-024
//! PM-8), the lowercase hexadecimal of the normal-form bytes. The normal form
//! is the content that keeps two claims with equal names apart, so two claims
//! share a node only when they assert the same thing. A node's identity is
//! never derived from a span or the declaration order.

use std::fmt::Write as _;

use quire_exact::{NodeKey, ValueType};

use super::Lowering;
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{BodyTerm, MemberTerm, NodeTag};
use crate::check::probabilistic::claim_location;
use crate::check::refusal::CheckRefusal;
use crate::check::CheckedProbabilisticClaim;

/// One checked probabilistic claim, as lowering reads it.
pub(crate) struct ProbabilisticClaimInput<'a> {
    /// The checked claim.
    pub(crate) claim: &'a CheckedProbabilisticClaim,
}

impl Lowering<'_> {
    /// Lower and key the probabilistic claim `input`, recording its one
    /// `claim` occurrence at its declaration.
    pub(crate) fn probabilistic_claim(
        &mut self,
        input: &ProbabilisticClaimInput<'_>,
    ) -> Result<NodeKey, CheckRefusal> {
        let claim = input.claim;
        let at = claim_location(claim.declared());
        let boolean = self.type_node(&ValueType::Boolean, &at)?;
        let name = self.text_literal(&claim.name, &at)?;
        let mut normal_form = String::new();
        for byte in claim.normal_form.as_bytes() {
            // Writing to a `String` cannot fail.
            let _ = write!(normal_form, "{byte:02x}");
        }
        let normal_form = self.text_literal(&normal_form, &at)?;
        let key = self.insert(
            &at,
            NodeTag::Claim,
            "probabilistic_claim",
            Some(boolean),
            None,
            BodyTerm::aggregate(vec![
                MemberTerm::bound("name", name),
                MemberTerm::bound("normal_form", normal_form),
            ]),
        )?;
        self.record(key.clone(), OccurrenceRole::Claim, at);
        Ok(key)
    }
}
