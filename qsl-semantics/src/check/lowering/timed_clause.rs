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

use quire_exact::{EffectiveId, Identifier, NodeKey, Value, ValueType};
use quire_semantic_value::location::Location;

use super::{Binder, Binders, Lowering, PlaceStep};
use crate::check::family::OccurrenceRole;
use crate::check::ir::{CheckedLiteral, NodeKind};
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LeafTerm, LiteralValue, MemberTerm,
    NodeTag, Operation, Operator, TupleMember, TupleTerm,
};
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::check::state_clause::ClauseOperation;
use crate::check::timed_model::{
    CheckedTimedPredicate, ClockConstraint, ClockField, ClockId, ConstraintNodes,
    ConstraintTargetNodes, GuardNodes, InvariantNodes, PartNodes, ResetNodes, ResetTargetNodes,
    StateAlternative, TimedClauseNodes, TimedModel,
};
use crate::value::member::Member;

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
    /// The root location of a time invariant's alternative's predicate
    /// site, by invariant and alternative.
    pub(crate) invariant_site: &'a dyn Fn(usize, usize) -> Location,
}

/// One clock place's `clock_place` template node (QSpec FR-322
/// "Timed-model nodes"), which a `presence_read` and a clause name.
#[derive(Clone, Debug)]
pub(crate) struct TimedPlace {
    /// The object type declaring the place's field.
    owner: EffectiveId,
    /// The place's path from its owner.
    path: Vec<PlaceStep>,
    /// The template node.
    node: NodeKey,
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
            let context = guard.context.clone();
            let operation = self.clause_anchor(guard.operation(), location)?;
            let mut parts = Vec::with_capacity(guard.parts().len());
            // A covered guard is its always-holding part: the disjuncts that
            // constrain no clock add nothing (QSpec `GJ-03`).
            let covered = guard.covered();
            for (index, part) in guard.parts().iter().enumerate() {
                if covered && part.clocks().is_empty() {
                    continue;
                }
                // A part with no data predicate has no site of its own: its
                // presences are located at the clause.
                let site = match part.data() {
                    Some(_) => (input.part_site)(at, index),
                    None => location.clone(),
                };
                let mut conjuncts =
                    self.self_presences(model.clocks(), part.presences(), context.clone(), &site)?;
                if let Some(data) = part.data() {
                    conjuncts.push(self.timed_predicate(data, &site)?);
                }
                let predicate = match self.conjunction(&conjuncts, &site)? {
                    Some(predicate) => predicate,
                    None => self.timed_true(location)?,
                };
                let constraints = part
                    .clocks()
                    .iter()
                    .map(|constraint| {
                        self.constraint_nodes(
                            constraint,
                            context.clone(),
                            guard.operation(),
                            location,
                        )
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
                    let mut product = Vec::with_capacity(held.parts.len() * parts.len());
                    for left in &held.parts {
                        for right in &parts {
                            let predicate = if self.is_truth(left.predicate.clone()) {
                                right.predicate.clone()
                            } else if self.is_truth(right.predicate.clone())
                                || left.predicate == right.predicate
                            {
                                left.predicate.clone()
                            } else {
                                self.connective(
                                    "quire.op.boolean.and",
                                    left.predicate.clone(),
                                    right.predicate.clone(),
                                    location,
                                )?
                            };
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
            let context = written.context.clone();
            let operation = self.clause_anchor(written.operation(), location)?;
            for reset in written.resets() {
                let target = match reset.parameter() {
                    Some(position) => ResetTargetNodes::Parameter {
                        parameter: clock_parameter_name(written.operation(), position, location)?,
                    },
                    None => {
                        let owner = self.timed_self(context.clone(), location)?;
                        let binders = match reset.selection() {
                            None => Vec::new(),
                            Some(selection) => {
                                let integer = self.type_node(&ValueType::Integer, location)?;
                                vec![self.parameter(selection.binder(), 0, integer, location)?]
                            }
                        };
                        // A selection by `present(e)` resets the held
                        // elements that are present: its condition is the
                        // `presence_read` of the element's place (QSpec
                        // `SD-02`); every other reset holds under `true`.
                        let condition = match reset.selection() {
                            Some(selection) if selection.presence_condition() => {
                                let clock =
                                    model.clocks().get(reset.clock().index()).ok_or_else(|| {
                                        super::fault(location, KeyFault::UnkeyableRequirements)
                                    })?;
                                let place =
                                    self.timed_place(clock.owner(), clock.path()).ok_or_else(
                                        || super::fault(location, KeyFault::UnkeyableRequirements),
                                    )?;
                                self.presence_read(place, owner.clone(), &binders, location)?
                            }
                            _ => self.timed_true(location)?,
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
                    operation: operation.clone(),
                    target,
                    value: reset.value().clone(),
                    span: reset.span(),
                });
            }
        }
        let mut invariants = Vec::with_capacity(model.invariants().len());
        for (at, invariant) in model.invariants().iter().enumerate() {
            let predicate = if invariant.from_bound() {
                self.timed_true(input.declaration)?
            } else {
                let sites: Vec<Location> = (0..invariant.alternatives().len())
                    .map(|alternative| (input.invariant_site)(at, alternative))
                    .collect();
                self.timed_state(
                    model.clocks(),
                    invariant.context(),
                    invariant.alternatives(),
                    &sites,
                )?
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
            declaring: operation.declaring.clone(),
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
        let mut owner = self.timed_self(context.clone(), location)?;
        let mut at = context;
        for step in via {
            let unreachable = || super::fault(location, KeyFault::UnkeyableRequirements);
            let PlaceStep::Field(name) = step else {
                return Err(unreachable());
            };
            let next = self
                .models
                .iter()
                .find_map(|model| model.referenced_object(at.clone(), name))
                .ok_or_else(unreachable)?;
            let object = self.object_node(at, location)?;
            let dereferenced = self.typed_application(
                location,
                object.clone(),
                Operator::Deref,
                Operation::plain("quire.op.model.deref"),
                vec![MemberTerm::Leaf(LeafTerm::reference(owner))],
            )?;
            let field_type = self.type_node(&ValueType::Reference(next.clone()), location)?;
            self.record(field_type.clone(), OccurrenceRole::Type, location.clone());
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
        self.record(self_type.clone(), OccurrenceRole::Type, location.clone());
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
            self.record(semantic_type.clone(), OccurrenceRole::Type, site.clone());
            let parameter = self.parameter(name, slot, semantic_type.clone(), site)?;
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
            Some(boolean.clone()),
            None,
            BodyTerm::literal(boolean, LiteralValue::Boolean(true)),
        )?;
        self.record(
            literal.clone(),
            OccurrenceRole::Expression,
            location.clone(),
        );
        self.boolean_expression(literal, location)
    }

    /// The `expression` node of the Boolean connective `identity` over
    /// `left` and `right`.
    fn connective(
        &mut self,
        identity: &str,
        left: NodeKey,
        right: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let term = self.typed_application(
            location,
            boolean,
            Operator::Binary,
            Operation::plain(identity),
            vec![
                MemberTerm::Leaf(LeafTerm::reference(left)),
                MemberTerm::Leaf(LeafTerm::reference(right)),
            ],
        )?;
        term.key()
            .ok_or_else(|| super::fault(location, KeyFault::UnkeyableRequirements))
    }

    /// The `and` of `conjuncts`, left to right; `None` for none.
    fn conjunction(
        &mut self,
        conjuncts: &[NodeKey],
        location: &Location,
    ) -> Result<Option<NodeKey>, CheckRefusal> {
        let Some((first, rest)) = conjuncts.split_first() else {
            return Ok(None);
        };
        let mut joined = first.clone();
        for conjunct in rest {
            joined = self.connective("quire.op.boolean.and", joined, conjunct.clone(), location)?;
        }
        Ok(Some(joined))
    }

    /// Whether `key` is the node [`Self::timed_true`] writes: the
    /// `expression`/`reference` node over the literal `true`.
    fn is_truth(&self, key: NodeKey) -> bool {
        let content = |key: NodeKey| self.graph.nodes.get(&key).map(|node| &node.content);
        let Some(wrapper) = content(key) else {
            return false;
        };
        let BodyTerm::Leaf(LeafTerm::Reference { target }) = &wrapper.body else {
            return false;
        };
        wrapper.node_tag == NodeTag::Expression
            && target.admitted().cloned().and_then(&content).is_some_and(|literal| {
                matches!(
                    &literal.body,
                    BodyTerm::Leaf(LeafTerm::Literal {
                        value: LiteralValue::Boolean(true),
                        ..
                    })
                )
            })
    }

    /// The `clock_place` template node of every clock of `clocks`, the
    /// clocks of the timed model whose model node is `model_node`, located
    /// at the model's time declaration `location` (QSpec FR-322
    /// "Timed-model nodes"): exactly `model`, `path` and `resolved_path`,
    /// each slot step's selector the `Integer` type node, and the place
    /// self-typed. The places are kept for the `presence_read` and the
    /// clauses that name them.
    pub(crate) fn clock_places(
        &mut self,
        clocks: &[ClockField],
        model_node: NodeKey,
        location: &Location,
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        if clocks.is_empty() {
            return Ok(Vec::new());
        }
        let integer = self.scalar("integer", location)?;
        let count = |value: u64| LeafTerm::literal(integer.clone(), LiteralValue::Count(value));
        let group = |members: Vec<(&str, LeafTerm)>| {
            TupleMember::Group(GroupTerm::new(
                members
                    .into_iter()
                    .map(|(name, leaf)| GroupMember::Binding(Binding::new(name, leaf)))
                    .collect(),
            ))
        };
        let mut nodes = Vec::with_capacity(clocks.len());
        for clock in clocks {
            let mut path = Vec::with_capacity(clock.path().len());
            for step in clock.path() {
                let members = match step {
                    PlaceStep::Field(name) => vec![
                        ("kind", self.text_literal("field", location)?),
                        ("name", self.text_literal(name, location)?),
                    ],
                    PlaceStep::Union(case) => vec![
                        ("kind", self.text_literal("union", location)?),
                        ("case", self.text_literal(case, location)?),
                    ],
                    PlaceStep::Slot(shape) => vec![
                        ("kind", self.text_literal("slot", location)?),
                        (
                            "collection",
                            self.text_literal(super::collection_kind(*shape), location)?,
                        ),
                        ("selector", LeafTerm::reference(integer.clone())),
                        ("minimum", count(shape.min)),
                        (
                            "maximum",
                            match shape.max {
                                Some(maximum) => count(maximum),
                                None => self.text_literal("unbounded", location)?,
                            },
                        ),
                    ],
                };
                path.push(group(members));
            }
            let mut rows = Vec::with_capacity(clock.resolved_path().len());
            for (row, at) in clock.resolved_path().iter().zip(0_u64..) {
                rows.push(group(vec![
                    ("path_step", count(at)),
                    (
                        "declaring_type",
                        self.text_literal(&row.declaring_type, location)?,
                    ),
                    (
                        "member_identity",
                        self.text_literal(&row.member_identity, location)?,
                    ),
                    (
                        "member_type",
                        self.text_literal(&row.member_type, location)?,
                    ),
                    (
                        "collection_kind",
                        self.text_literal(&row.collection_kind, location)?,
                    ),
                    ("minimum", self.text_literal(&row.minimum, location)?),
                    ("maximum", self.text_literal(&row.maximum, location)?),
                ]));
            }
            let body = BodyTerm::aggregate(vec![
                MemberTerm::bound("model", LeafTerm::reference(model_node.clone())),
                MemberTerm::binding("path", BindingValue::Tuple(TupleTerm::new(path))),
                MemberTerm::binding("resolved_path", BindingValue::Tuple(TupleTerm::new(rows))),
            ]);
            let node = self.insert(
                location,
                NodeTag::TimedModel,
                "clock_place",
                None,
                None,
                body,
            )?;
            self.record(node.clone(), OccurrenceRole::Expression, location.clone());
            self.timed_places.push(TimedPlace {
                owner: clock.owner(),
                path: clock.path().to_vec(),
                node: node.clone(),
            });
            nodes.push(node);
        }
        Ok(nodes)
    }

    /// The `clock_place` template node of the place at `path` of the object
    /// type `owner`, once [`Self::clock_places`] wrote it.
    pub(super) fn timed_place(&self, owner: EffectiveId, path: &[PlaceStep]) -> Option<NodeKey> {
        self.timed_places
            .iter()
            .find(|place| place.owner == owner && place.path == path)
            .map(|place| place.node.clone())
    }

    /// The `presence_read` of the place `place` at the owner term `owner`
    /// and one slot term for each slot step (QSpec FR-322 "Presence"): the
    /// application of `quire.op.option.present`, typed `Boolean`.
    fn presence_read(
        &mut self,
        place: NodeKey,
        owner: NodeKey,
        slots: &[NodeKey],
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let mut arguments = vec![
            MemberTerm::Leaf(LeafTerm::reference(place)),
            MemberTerm::Leaf(LeafTerm::reference(owner)),
        ];
        arguments.extend(
            slots
                .iter()
                .map(|slot| MemberTerm::Leaf(LeafTerm::reference(slot.clone()))),
        );
        self.typed_application(
            location,
            boolean,
            Operator::Present,
            Operation::plain("quire.op.option.present"),
            arguments,
        )?
        .key()
        .ok_or_else(|| super::fault(location, KeyFault::UnkeyableRequirements))
    }

    /// The `presence_read` of each of `presences`, a place of `self`, an
    /// object of `context`, with whether present: the application of
    /// `quire.op.option.present` over the place's template and the owner
    /// term `self` (QSpec FR-322 "Presence"), under `not` where absent.
    fn self_presences(
        &mut self,
        clocks: &[ClockField],
        presences: &[(ClockId, bool)],
        context: EffectiveId,
        location: &Location,
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        let mut reads = Vec::with_capacity(presences.len());
        for (clock, present) in presences {
            let unplaced = || super::fault(location, KeyFault::UnkeyableRequirements);
            let clock = clocks.get(clock.index()).ok_or_else(unplaced)?;
            let place = self
                .timed_place(clock.owner(), clock.path())
                .ok_or_else(unplaced)?;
            let owner = self.timed_self(context.clone(), location)?;
            let boolean = self.type_node(&ValueType::Boolean, location)?;
            let read = self.presence_read(place, owner, &[], location)?;
            reads.push(if *present {
                read
            } else {
                self.typed_application(
                    location,
                    boolean,
                    Operator::Unary,
                    Operation::plain("quire.op.boolean.not"),
                    vec![MemberTerm::Leaf(LeafTerm::reference(read))],
                )?
                .key()
                .ok_or_else(unplaced)?
            });
        }
        Ok(reads)
    }

    /// The Boolean `expression` node of a state predicate over objects of
    /// `context` in its disjunctive form over the presences of `self`'s
    /// places (FR-230 "Presence bit"): the `or` of its alternatives, each
    /// the `and` of its presences' `presence_read`s and its data predicate,
    /// typed at the alternative's site in `sites`; a data predicate `true`
    /// beside a presence is left out.
    pub(crate) fn timed_state(
        &mut self,
        clocks: &[ClockField],
        context: EffectiveId,
        alternatives: &[StateAlternative],
        sites: &[Location],
    ) -> Result<NodeKey, CheckRefusal> {
        let mut disjuncts = Vec::with_capacity(alternatives.len());
        for (alternative, site) in alternatives.iter().zip(sites) {
            let mut conjuncts =
                self.self_presences(clocks, alternative.presences(), context.clone(), site)?;
            let data = alternative.predicate();
            let trivial = matches!(
                data.checked_body().root().kind(),
                NodeKind::Literal(CheckedLiteral(Value::Boolean(true)))
            );
            if conjuncts.is_empty() || !trivial {
                conjuncts.push(self.timed_predicate(data, site)?);
            }
            disjuncts.extend(self.conjunction(&conjuncts, site)?);
        }
        let (Some((first, rest)), Some(site)) = (disjuncts.split_first(), sites.first()) else {
            return Err(super::fault(
                &Location::root(quire_semantic_value::location::Origin::Expression),
                KeyFault::UnkeyableRequirements,
            ));
        };
        let mut joined = first.clone();
        for disjunct in rest {
            joined = self.connective("quire.op.boolean.or", joined, disjunct.clone(), site)?;
        }
        self.boolean_expression(joined, site)
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
        self.record(
            wrapped.clone(),
            OccurrenceRole::Expression,
            location.clone(),
        );
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
