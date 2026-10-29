// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230, QSpec FR-322 "Clause nodes": lower the terms a timed model's
//! guards, resets and time invariants name into checked nodes, which the
//! emitter's `clock_guard`, `clock_reset`, `time_invariant` and
//! `clock_constraint` nodes reference.
//!
//! A clause names its operation by the operation's no-point
//! `state`/`operation_anchor` (QSpec FR-342-AC-11), the anchor a point-free
//! `pre` or `post` clause already binds, so the reference adds no
//! occurrence. The owner term of a place is `self`, a `parameter` node at
//! level 0 typed `Reference<T>`, `T` the clause's context. A guard part's or
//! a time invariant's predicate is the Boolean `expression` node of its
//! data predicate, lowered at its own site so that its nodes are located,
//! or of `true` when it reads no data. A predicate whose root is a value (a
//! literal, or a read of a parameter) is wrapped in an
//! `expression`/`reference` node, as QSpec's clause fixture writes `true`.
//! A reset by selection binds its element by a `parameter` node of the
//! slot's selector domain, `Integer`, which is its one slot term.

use quire_exact::{EffectiveId, Identifier, NodeKey, ValueType};
use quire_semantic_value::location::Location;

use super::{Binder, Binders, Lowering, PlaceStep};
use crate::value::member::Member;
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{
    BodyTerm, LeafTerm, LiteralValue, MemberTerm, NodeTag, Operation, Operator,
};
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::check::state_clause::ClauseOperation;
use crate::check::timed_model::{
    CheckedTimedPredicate, ClockConstraint, ConstraintNodes, ConstraintTargetNodes, GuardNodes,
    InvariantNodes, PartNodes, ResetNodes, ResetTargetNodes, TimedClauseNodes, TimedModel,
};

/// What [`Lowering::timed_clauses`] reads beside the model.
pub(crate) struct TimedClauseInput<'a> {
    /// The checked model, which [`TimedModel::unwritten`] admits.
    pub(crate) model: &'a TimedModel,
    /// Each state clause's root location, by its index among the unit's
    /// state clauses.
    pub(crate) clause_locations: &'a [Location],
    /// The location of the model's time declaration, where the nodes of a
    /// bound of a `max` constraint on clocks are located.
    pub(crate) declaration: &'a Location,
    /// The root location of a guard part's predicate site, by guard and
    /// part.
    pub(crate) part_site: &'a dyn Fn(usize, usize) -> Location,
    /// The root location of a time invariant's predicate site.
    pub(crate) invariant_site: &'a dyn Fn(usize) -> Location,
}

impl Lowering<'_> {
    /// Lower the terms of every guard, reset and time invariant of
    /// `input.model`, one guard for each guarded operation.
    pub(crate) fn timed_clauses(
        &mut self,
        input: &TimedClauseInput<'_>,
    ) -> Result<TimedClauseNodes, CheckRefusal> {
        let model = input.model;
        // Every guard and reset is of a state clause of the unit.
        let clause_at = |index: usize| {
            input
                .clause_locations
                .get(index)
                .ok_or_else(|| super::fault(input.declaration, KeyFault::UnkeyableRequirements))
        };
        let mut guards = Vec::with_capacity(model.guards().len());
        for (at, guard) in model.guards().iter().enumerate() {
            let location = clause_at(guard.index)?;
            let context = guard.context;
            let operation = self.clause_anchor(guard.operation(), location)?;
            let mut parts = Vec::with_capacity(guard.parts().len());
            for (index, part) in guard.parts().iter().enumerate() {
                let predicate = match part.data() {
                    Some(data) => self.timed_predicate(data, &(input.part_site)(at, index))?,
                    None => self.timed_true(location)?,
                };
                let constraints = part
                    .clocks()
                    .iter()
                    .map(|constraint| {
                        self.constraint_nodes(constraint, context, guard.operation(), location)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                parts.push(PartNodes {
                    predicate,
                    constraints,
                });
            }
            // QSpec FR-322 "Clause nodes": one `clock_guard` for each
            // operation anchor. A second guarded precondition of the same
            // operation conjoins with the first: each part of the one guard
            // is a part of each, its predicate their conjunction and its
            // constraints both lists.
            match guards
                .iter_mut()
                .find(|held: &&mut GuardNodes| held.operation == operation)
            {
                Some(held) => {
                    let truth = self.timed_true(location)?;
                    let mut product = Vec::with_capacity(held.parts.len() * parts.len());
                    for left in &held.parts {
                        for right in &parts {
                            let predicate =
                                self.timed_and(left.predicate, right.predicate, truth, location)?;
                            let mut constraints = left.constraints.clone();
                            constraints.extend(right.constraints.iter().cloned());
                            let part = PartNodes {
                                predicate,
                                constraints,
                            };
                            if !product.contains(&part) {
                                product.push(part);
                            }
                        }
                    }
                    held.parts = product;
                }
                None => guards.push(GuardNodes {
                    operation,
                    span: guard.span(),
                    parts,
                }),
            }
        }
        let mut resets = Vec::new();
        for written in model.resets() {
            let location = clause_at(written.index)?;
            let context = written.context;
            let operation = self.clause_anchor(written.operation(), location)?;
            for reset in written.resets() {
                let target = match reset.parameter() {
                    Some(position) => ResetTargetNodes::Parameter {
                        parameter: clock_parameter_name(written.operation(), position, location)?,
                    },
                    None => {
                        let owner = self.timed_self(context, location)?;
                        let condition = self.timed_true(location)?;
                        let binders = match reset.selection() {
                            None => Vec::new(),
                            Some(selection) => {
                                let integer = self.type_node(&ValueType::Integer, location)?;
                                vec![self.parameter(selection.binder(), 0, integer, location)?]
                            }
                        };
                        ResetTargetNodes::Place {
                            clock: reset.clock(),
                            owner,
                            slots: binders.clone(),
                            binders,
                            condition,
                        }
                    }
                };
                resets.push(ResetNodes {
                    operation,
                    target,
                    value: reset.value().clone(),
                    span: reset.span(),
                });
            }
        }
        let mut invariants = Vec::with_capacity(model.invariants().len());
        for (at, invariant) in model.invariants().iter().enumerate() {
            let [alternative] = invariant.alternatives() else {
                return Err(super::fault(
                    input.declaration,
                    KeyFault::UnkeyableRequirements,
                ));
            };
            let predicate = if invariant.from_bound() {
                self.timed_true(input.declaration)?
            } else {
                self.timed_predicate(alternative.predicate(), &(input.invariant_site)(at))?
            };
            let bounds = invariant
                .bounds()
                .iter()
                .map(|bound| self.place_constraint(bound, invariant.context(), input.declaration))
                .collect::<Result<Vec<_>, _>>()?;
            invariants.push(InvariantNodes {
                predicate,
                bounds,
                span: invariant.span(),
            });
        }
        Ok(TimedClauseNodes {
            guards,
            resets,
            invariants,
        })
    }

    /// The no-point `operation_anchor` of `operation`, which a point-free
    /// clause naming it binds at `location` with its `anchor` occurrence.
    fn clause_anchor(
        &mut self,
        operation: &ClauseOperation,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let anchor = super::AnchorInput {
            declaring: operation.declaring,
            operation: &operation.declaration,
            location,
            reference: location,
        };
        Ok(self.operation_anchor(&anchor, None, location)?.anchor)
    }

    /// The nodes of a guard part's constraint.
    fn constraint_nodes(
        &mut self,
        constraint: &ClockConstraint,
        context: EffectiveId,
        operation: &ClauseOperation,
        location: &Location,
    ) -> Result<ConstraintNodes, CheckRefusal> {
        match constraint.parameter() {
            Some(position) => Ok(ConstraintNodes {
                target: ConstraintTargetNodes::Parameter {
                    operation: self.clause_anchor(operation, location)?,
                    parameter: clock_parameter_name(operation, position, location)?,
                },
                comparison: constraint.op(),
                bound: constraint.bound().clone(),
                span: constraint.span(),
            }),
            None => self.place_constraint(constraint, context, location),
        }
    }

    /// The nodes of a constraint on a clock of `self`, an object of
    /// `context`.
    fn place_constraint(
        &mut self,
        constraint: &ClockConstraint,
        context: EffectiveId,
        location: &Location,
    ) -> Result<ConstraintNodes, CheckRefusal> {
        Ok(ConstraintNodes {
            target: ConstraintTargetNodes::Place {
                clock: constraint.clock(),
                owner: self.timed_owner(context, constraint.via(), location)?,
                slots: Vec::new(),
            },
            comparison: constraint.op(),
            bound: constraint.bound().clone(),
            span: constraint.span(),
        })
    }

    /// The owner term of a clock `self` reaches through the reference
    /// fields `via` (FR-230-AC-10's `self.peer`): `self`, then for each
    /// field the `quire.op.model.deref` of the reference so far, typed at
    /// its object type's model node, and over it the `record.project` of the
    /// field, typed `Reference<T>`, as S3 lowers an attribute read.
    fn timed_owner(
        &mut self,
        context: EffectiveId,
        via: &[PlaceStep],
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let mut owner = self.timed_self(context, location)?;
        let mut at = context;
        for step in via {
            let unreachable = || super::fault(location, KeyFault::UnkeyableRequirements);
            let PlaceStep::Field(name) = step else {
                return Err(unreachable());
            };
            let next = self
                .models
                .iter()
                .find_map(|model| model.referenced_object(at, name))
                .ok_or_else(unreachable)?;
            let object = self.object_node(at, location)?;
            let dereferenced = self.typed_application(
                location,
                object,
                Operator::Deref,
                Operation::plain("quire.op.model.deref"),
                vec![MemberTerm::Leaf(LeafTerm::reference(owner))],
            )?;
            let field_type = self.type_node(&ValueType::Reference(next), location)?;
            self.record(field_type, OccurrenceRole::Type, location.clone());
            let name = Identifier::new(name.as_str()).map_err(|_| unreachable())?;
            let read = self.typed_application(
                location,
                field_type,
                Operator::Query,
                Operation {
                    member: Some(Member::Field {
                        declaration: object,
                        name,
                    }),
                    ..Operation::plain("quire.op.record.project")
                },
                vec![MemberTerm::Leaf(dereferenced)],
            )?;
            owner = read.key().ok_or_else(unreachable)?;
            at = next;
        }
        Ok(owner)
    }

    /// `self`: the `parameter` node at level 0 typed `Reference<context>`.
    fn timed_self(
        &mut self,
        context: EffectiveId,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let self_type = self.type_node(&ValueType::Reference(context), location)?;
        self.record(self_type, OccurrenceRole::Type, location.clone());
        self.parameter("self", 0, self_type, location)
    }

    /// The Boolean `expression` node of `predicate`, typed at `site`: each
    /// slot the body reads is a `parameter` node at its level.
    fn timed_predicate(
        &mut self,
        predicate: &CheckedTimedPredicate,
        site: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let body = predicate.checked_body();
        let mut scope = Vec::new();
        for (slot, (name, value_type)) in predicate.parameters().iter().enumerate() {
            if !body.reads_local(slot) {
                continue;
            }
            let semantic_type = self.binder_type(value_type, None, site)?;
            self.record(semantic_type, OccurrenceRole::Type, site.clone());
            let parameter = self.parameter(name, slot, semantic_type, site)?;
            scope.push(Binder {
                slot,
                parameter,
                semantic_type,
            });
        }
        let mut binders = Binders::new(predicate.slot_names(), &scope);
        let term = self.expression(body.root(), &mut binders)?;
        let key = term
            .key()
            .ok_or_else(|| super::fault(site, KeyFault::UnkeyableRequirements))?;
        self.boolean_expression(key, site)
    }

    /// The `expression` node of the predicate `true`.
    fn timed_true(&mut self, location: &Location) -> Result<NodeKey, CheckRefusal> {
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let literal = self.insert(
            location,
            NodeTag::Value,
            "literal",
            Some(boolean),
            None,
            BodyTerm::literal(boolean, LiteralValue::Boolean(true)),
        )?;
        self.record(literal, OccurrenceRole::Expression, location.clone());
        self.boolean_expression(literal, location)
    }

    /// The Boolean `expression` node of `left and right`, `truth` being the
    /// node of `true`: either operand when the other is `truth`.
    fn timed_and(
        &mut self,
        left: NodeKey,
        right: NodeKey,
        truth: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        if left == truth {
            return Ok(right);
        }
        if right == truth || left == right {
            return Ok(left);
        }
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let term = self.typed_application(
            location,
            boolean,
            Operator::Binary,
            Operation::plain("quire.op.boolean.and"),
            vec![
                MemberTerm::Leaf(LeafTerm::reference(left)),
                MemberTerm::Leaf(LeafTerm::reference(right)),
            ],
        )?;
        term.key()
            .ok_or_else(|| super::fault(location, KeyFault::UnkeyableRequirements))
    }

    /// `key` when it is an `expression` node; else the
    /// `expression`/`reference` node over it, typed `Boolean`.
    pub(super) fn boolean_expression(
        &mut self,
        key: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let tag = self.graph.nodes.get(&key).map(|node| node.content.node_tag);
        if tag == Some(NodeTag::Expression) {
            return Ok(key);
        }
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let wrapped = self.insert(
            location,
            NodeTag::Expression,
            "reference",
            Some(boolean),
            None,
            BodyTerm::Leaf(LeafTerm::reference(key)),
        )?;
        self.record(wrapped, OccurrenceRole::Expression, location.clone());
        Ok(wrapped)
    }
}

/// The declared name of `operation`'s `Clock` parameter at `position`.
fn clock_parameter_name(
    operation: &ClauseOperation,
    position: usize,
    location: &Location,
) -> Result<String, CheckRefusal> {
    operation
        .declaration
        .clock_parameters()
        .iter()
        .find(|(at, _)| *at == position)
        .map(|(_, name)| name.clone())
        .ok_or_else(|| super::fault(location, KeyFault::UnkeyableRequirements))
}
