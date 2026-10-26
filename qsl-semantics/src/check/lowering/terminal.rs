// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411, QSpec FR-476: lower a checked model `terminal` declaration into
//! its content-addressed `temporal`/`terminal` node.
//!
//! The node's body is the aggregate `model`, `mode`, `parameters`,
//! `condition`. It carries no `declaration`: the written name enters no key,
//! only the node's `claim` occurrence, so renaming the declaration changes
//! neither the node id nor the `package_id`.

use quire_exact::{EffectiveId, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{Binder, Binders, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{BindingValue, BodyTerm, GroupTerm, LeafTerm, MemberTerm, NodeTag};
use crate::check::refusal::CheckRefusal;
use crate::check::{CheckedTerminalPredicate, TerminalDeclaration, TerminalMode};

/// One place in a `terminal` declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalSite {
    /// The whole declaration: the node's `claim` occurrence.
    Claim,
    /// The `when` predicate's expression tree.
    Predicate,
}

/// The `Origin::TypeDeclaration` name of `site` of the `terminal` declaration
/// at `index`, its index among the unit's `terminal` declarations. Spaced, so
/// it names no declared type.
pub(crate) fn terminal_site(index: usize, site: TerminalSite) -> String {
    match site {
        TerminalSite::Claim => format!("terminal {index}"),
        TerminalSite::Predicate => format!("terminal {index} predicate"),
    }
}

/// The root location of [`terminal_site`].
pub(crate) fn terminal_location(index: usize, site: TerminalSite) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: terminal_site(index, site),
    })
}

/// One checked `terminal` declaration, as lowering reads it.
pub(crate) struct TerminalInput<'a> {
    /// The declaration's index among the unit's `terminal` declarations.
    pub(crate) index: usize,
    /// The model alias the declaration is on.
    pub(crate) model: &'a str,
    /// The checked declaration: never [`TerminalDeclaration::None`].
    pub(crate) declaration: &'a TerminalDeclaration,
}

impl Lowering<'_> {
    /// Lower and key the `terminal` declaration `input`, recording its one
    /// `claim` occurrence at the declaration. Returns the node's key.
    pub(crate) fn terminal(&mut self, input: &TerminalInput<'_>) -> Result<NodeKey, CheckRefusal> {
        let claim_at = terminal_location(input.index, TerminalSite::Claim);
        let boolean = self.type_node(&ValueType::Boolean, &claim_at)?;
        self.record(boolean, OccurrenceRole::Type, claim_at.clone());
        let object = self.terminal_object(input.model, &claim_at)?;
        let model = self.object_node(object, &claim_at)?;

        let (mode, parameters, condition) = match input.declaration {
            TerminalDeclaration::When(predicate) => {
                let (parameters, condition) =
                    self.terminal_predicate(input.index, object, predicate)?;
                (
                    TerminalMode::When.wire(),
                    parameters,
                    BindingValue::Leaf(LeafTerm::reference(condition)),
                )
            }
            TerminalDeclaration::Any => (TerminalMode::Any.wire(), empty_group(), empty_group()),
            TerminalDeclaration::None => {
                return Err(super::fault(
                    &claim_at,
                    crate::check::refusal::KeyFault::UnkeyableRequirements,
                ))
            }
        };
        let mode = self.text_literal(mode, &claim_at)?;
        let key = self.insert(
            &claim_at,
            NodeTag::Temporal,
            "terminal",
            Some(boolean),
            None,
            BodyTerm::aggregate(vec![
                MemberTerm::bound("model", LeafTerm::reference(model)),
                MemberTerm::bound("mode", mode),
                MemberTerm::binding("parameters", parameters),
                MemberTerm::binding("condition", condition),
            ]),
        )?;
        self.record(key, OccurrenceRole::Claim, claim_at);
        Ok(key)
    }

    /// The object type the model `alias`'s `self` ranges over: the one object
    /// type its populations range over (FR-124), else its least object type,
    /// so that a `terminal any` over a model with no population still names
    /// a model node.
    fn terminal_object(
        &self,
        alias: &str,
        location: &Location,
    ) -> Result<EffectiveId, CheckRefusal> {
        self.models
            .iter()
            .find(|model| model.alias() == Some(alias))
            .and_then(|model| model.terminal_object())
            .ok_or_else(|| {
                super::fault(
                    location,
                    crate::check::refusal::KeyFault::UnkeyableRequirements,
                )
            })
    }

    /// The `parameters` aggregate and the `expression` node of a `when`
    /// predicate: `self` is bound, at level 0 and typed `Reference<M::T>`,
    /// only when the predicate reads it.
    fn terminal_predicate(
        &mut self,
        index: usize,
        object: EffectiveId,
        predicate: &CheckedTerminalPredicate,
    ) -> Result<(BindingValue, NodeKey), CheckRefusal> {
        let claim_at = terminal_location(index, TerminalSite::Claim);
        let mut scope = Vec::new();
        if predicate.body.reads_local(0) {
            let self_type = self.type_node(&ValueType::Reference(object), &claim_at)?;
            let parameter = self.parameter("self", 0, self_type, &claim_at)?;
            self.record(self_type, OccurrenceRole::Type, claim_at.clone());
            scope.push(Binder {
                slot: 0,
                parameter,
                semantic_type: self_type,
            });
        }
        let parameters = BindingValue::Group(GroupTerm::references(
            scope.iter().map(|binder| binder.parameter),
        ));
        let mut binders = Binders::new(&predicate.slot_names, &scope);
        let term = self.expression(predicate.body.root(), &mut binders)?;
        let key = term.key().ok_or_else(|| {
            super::fault(
                &claim_at,
                crate::check::refusal::KeyFault::UnkeyableRequirements,
            )
        })?;
        Ok((parameters, key))
    }
}

fn empty_group() -> BindingValue {
    BindingValue::Group(GroupTerm::new(Vec::new()))
}
