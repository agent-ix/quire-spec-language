// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-410: the `claim`/`state_graph` node each checked state-graph claim
//! lowers to (QSpec FR-475), with the nodes it references: the `over`
//! parameter and the two predicate nodes.
//!
//! The node's body is the aggregate of FR-475's four bindings, `kind`,
//! `over`, `from` and `target`, in that order. A predicate is the `expression`
//! node of its Boolean term or, for a constant such as `true`, the
//! `value`/`literal` node. An `always possible` claim written without `from`
//! references the node of the constant `true`, so it keys as the claim
//! written `from (true)`.

use quire_exact::{NodeKey, ValueType};

use super::temporal::stage_fault;
use super::{Binder, Binders, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{
    BindingValue, BodyTerm, GroupTerm, LeafTerm, LiteralValue, MemberTerm, NodeTag,
};
use crate::check::refusal::CheckRefusal;
use crate::check::state_graph_claim::{
    state_graph_location, CheckedStateGraphClaim, CheckedStatePredicate, Site, StateGraphForm,
};

/// One checked state-graph claim, as lowering reads it.
pub(crate) struct StateGraphClaimInput<'a> {
    /// The claim's index among the unit's state-graph claims.
    pub(crate) index: usize,
    /// The checked claim.
    pub(crate) claim: &'a CheckedStateGraphClaim,
}

/// A lowered state-graph claim's keys.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoweredStateGraphClaim {
    /// The `claim`/`state_graph` node: the claim's identity.
    pub(crate) key: NodeKey,
    /// The `Boolean` scalar type node: the claim's semantic type.
    pub(crate) boolean: NodeKey,
}

impl Lowering<'_> {
    /// Lower and key the state-graph claim `input`, recording its one
    /// `claim` occurrence at its declared name.
    pub(crate) fn state_graph_claim(
        &mut self,
        input: &StateGraphClaimInput<'_>,
    ) -> Result<LoweredStateGraphClaim, CheckRefusal> {
        let claim = input.claim;
        let at = |site| state_graph_location(input.index, site);
        let claim_at = at(Site::Claim);
        let boolean = self.type_node(&ValueType::Boolean, &claim_at)?;

        let mut scope: Vec<Binder> = Vec::new();
        let over = match claim.over() {
            None => None,
            Some(over) => {
                let over_at = at(Site::Over);
                let semantic_type =
                    self.binder_type(&ValueType::Reference(over.object), None, &over_at)?;
                let parameter = self.parameter(&over.name, 0, semantic_type, &over_at)?;
                self.record(semantic_type, OccurrenceRole::Type, over_at);
                scope.push(Binder {
                    slot: 0,
                    parameter,
                    semantic_type,
                });
                Some(parameter)
            }
        };

        let (from, target) = claim.node_predicates();
        let from = from
            .map(|predicate| {
                self.state_predicate(predicate, Site::From, input.index, boolean, &scope)
            })
            .transpose()?;
        let target = self.state_predicate(target, Site::Target, input.index, boolean, &scope)?;

        let kind = self.text_literal(
            match claim.form() {
                StateGraphForm::Possible { .. } => "possible",
                StateGraphForm::AlwaysPossible { .. } => "always-possible",
                StateGraphForm::UniquePath { .. } => "unique-path",
            },
            &claim_at,
        )?;
        let absent = || BindingValue::Group(GroupTerm::new(Vec::new()));
        let key = self.insert(
            &claim_at,
            NodeTag::Claim,
            "state_graph",
            Some(boolean),
            None,
            BodyTerm::aggregate(vec![
                MemberTerm::bound("kind", kind),
                match over {
                    Some(over) => MemberTerm::bound("over", LeafTerm::reference(over)),
                    None => MemberTerm::binding("over", absent()),
                },
                match from {
                    Some(from) => MemberTerm::bound("from", from),
                    None => MemberTerm::binding("from", absent()),
                },
                MemberTerm::bound("target", target),
            ]),
        )?;
        self.record(key, OccurrenceRole::Claim, claim_at);
        Ok(LoweredStateGraphClaim { key, boolean })
    }

    /// The reference to the node of `predicate` located at `site`: its
    /// lowered Boolean term, or the `true` literal node for the constant.
    fn state_predicate(
        &mut self,
        predicate: &CheckedStatePredicate,
        site: Site,
        index: usize,
        boolean: NodeKey,
        scope: &[Binder],
    ) -> Result<LeafTerm, CheckRefusal> {
        let at = state_graph_location(index, site);
        match predicate.written() {
            Some(written) => {
                let mut binders = Binders::new(written.slot_names(), scope);
                self.expression(written.body(), &mut binders)
            }
            None => {
                if !predicate.is_constant_true() {
                    return Err(stage_fault(&at, "constant-predicate-is-true"));
                }
                let key = self.insert(
                    &at,
                    NodeTag::Value,
                    "literal",
                    Some(boolean),
                    None,
                    BodyTerm::literal(boolean, LiteralValue::Boolean(true)),
                )?;
                self.record(key, OccurrenceRole::Expression, at);
                Ok(LeafTerm::reference(key))
            }
        }
    }
}
