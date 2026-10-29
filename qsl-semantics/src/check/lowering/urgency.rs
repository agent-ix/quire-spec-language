// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230, QSpec FR-322 "Timed-model nodes": lower the data predicate of an
//! `urgent when` declaration into the checked expression node its `urgency`
//! node references.
//!
//! `self` is a `parameter` node at level 0 typed `Reference<T>`, `T` the
//! object type the predicate ranges over, written only when the predicate
//! reads it, as QSpec's timed fixtures write the owner term of a
//! `presence_read`. The predicate's nodes are located at its own site
//! ([`crate::check::timed_model::urgency_site`]).

use quire_exact::{EffectiveId, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{Binder, Binders, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::check::timed_model::CheckedTimedPredicate;

impl Lowering<'_> {
    /// Lower `predicate`, an `urgent when` predicate over objects of
    /// `context` typed at `site`, and return its expression node's key.
    pub(crate) fn urgency_predicate(
        &mut self,
        context: EffectiveId,
        predicate: &CheckedTimedPredicate,
        site: &str,
    ) -> Result<NodeKey, CheckRefusal> {
        let at = Location::root(Origin::TypeDeclaration {
            name: site.to_owned(),
        });
        let mut scope = Vec::new();
        if predicate.checked_body().reads_local(0) {
            let self_type = self.type_node(&ValueType::Reference(context), &at)?;
            let parameter = self.parameter("self", 0, self_type, &at)?;
            self.record(self_type, OccurrenceRole::Type, at.clone());
            scope.push(Binder {
                slot: 0,
                parameter,
                semantic_type: self_type,
            });
        }
        let mut binders = Binders::new(predicate.slot_names(), &scope);
        let term = self.expression(predicate.checked_body().root(), &mut binders)?;
        term.key()
            .ok_or_else(|| super::fault(&at, KeyFault::UnkeyableRequirements))
    }
}
