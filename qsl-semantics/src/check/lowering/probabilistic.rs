// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-498: the `claim`/`probabilistic_claim` node each checked probabilistic
//! claim lowers to, with its one declared `claim` occurrence.
//!
//! The producer keys the node structurally under QSpec FR-322's
//! `{package, path, ordinal}` rule: path `["claim", "probabilistic_claim",
//! name]`, and an ordinal that ranks the claims sharing that name by their
//! written content ([`qsl_forms::SignificantContent`]). No span, source position
//! or declaration order enters the key, so permuting the declarations keys
//! every claim as before; claims written identically are indistinguishable,
//! so which of them takes which rank is unobservable. The node's body is an
//! empty aggregate: no QSpec body is defined for the form yet, and none is
//! fabricated.

use std::collections::hash_map;

use quire_exact::{NodeKey, PathSegment};

use super::temporal::stage_fault;
use super::{Lowering, NodeContent, SemanticNode};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{BodyTerm, NodeTag};
use crate::check::probabilistic::CheckedProbabilisticClaim;
use crate::check::refusal::CheckRefusal;

/// The semantic form of a probabilistic claim's node.
const FORM: &str = "probabilistic_claim";

impl Lowering<'_> {
    /// Key and add the node of each of `claims`, recording its one `claim`
    /// occurrence at the claim's declaration. The keys are in `claims`'
    /// order.
    pub(crate) fn probabilistic_claims(
        &mut self,
        claims: &[CheckedProbabilisticClaim],
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        let mut ranked: Vec<(usize, &CheckedProbabilisticClaim)> =
            claims.iter().enumerate().collect();
        // Within one name, by content; equal content is equal claims, whose
        // tie order is unobservable.
        ranked.sort_by(|(_, left), (_, right)| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.content.cmp(&right.content))
        });
        let mut keys = Vec::with_capacity(claims.len());
        let mut ordinal = 0_u64;
        let mut previous: Option<&str> = None;
        for (index, claim) in ranked {
            if previous == Some(claim.name.as_str()) {
                ordinal += 1;
            } else {
                ordinal = 0;
            }
            previous = Some(&claim.name);
            keys.push((index, self.probabilistic_claim(claim, ordinal)?));
        }
        keys.sort_by_key(|(index, _)| *index);
        Ok(keys.into_iter().map(|(_, key)| key).collect())
    }

    /// Add `claim`'s node at `ordinal` of its name's path.
    fn probabilistic_claim(
        &mut self,
        claim: &CheckedProbabilisticClaim,
        ordinal: u64,
    ) -> Result<NodeKey, CheckRefusal> {
        let at = claim.declaration_location();
        self.charge(1, &at)?;
        let key = super::super::derived_node_key(
            &self.package_name,
            NodeTag::Claim,
            FORM,
            vec![PathSegment::Text(claim.name.clone())],
            ordinal,
        )
        .map_err(|_| stage_fault(&at, "probabilistic-claim-key-is-well-formed"))?;
        match self.graph.nodes.entry(key.clone()) {
            hash_map::Entry::Vacant(slot) => {
                slot.insert(SemanticNode {
                    key: key.clone(),
                    recursion: None,
                    nominal: None,
                    content: NodeContent {
                        node_tag: NodeTag::Claim,
                        semantic_form: FORM,
                        semantic_type: None,
                        declaration: None,
                        owner: None,
                        owner_model: None,
                        body: BodyTerm::aggregate(Vec::new()),
                    },
                });
            }
            hash_map::Entry::Occupied(_) => {
                return Err(stage_fault(&at, "probabilistic-claim-key-is-unheld"));
            }
        }
        self.record(key.clone(), OccurrenceRole::Claim, at);
        Ok(key)
    }
}
