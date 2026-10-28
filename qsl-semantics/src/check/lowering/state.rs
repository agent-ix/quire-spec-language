// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-104's state clause nodes: the `state`/`state_clause` node that is a
//! checked clause's identity, and, for a `pre` or `post` clause, the
//! `state`/`operation_anchor` and `state`/`frame` nodes of the operation
//! it names.
//!
//! FR-104 needs each clause's node identity, its `claim` occurrence and the
//! frame node's own occurrence to key the clause's requirement records.
//! The node contents follow FR-105's Outputs table, in the wire spellings
//! QSpec STD-111 published: a `state_clause` node's body is a
//! `quire.op.state.clause` application (FR-341), an `operation_anchor`
//! node's body is FR-342's three-binding aggregate, and a `frame` node's
//! body is FR-340's own `frame` term ([`BodyTerm::Frame`]). The key covers the clause kind, its anchor and
//! its checked body and nothing else: two declarations with equal kind,
//! anchor and body share one node and differ in their `claim` occurrence
//! (FR-088, FR-104-AC-6), and the declared name enters no key.

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, NodeKey, Origin, ValueType};

use super::{fault, Binder, Binders, Lowering};
use crate::check::claims::BinderSite;
use crate::check::family::OccurrenceRole;
use crate::check::ir::CheckedNode;
use crate::check::node_key::{
    BindingValue, BodyTerm, FrameDeletesScoped, FrameField, FrameModifies, FrameScope, GroupTerm,
    LeafTerm, MemberTerm, NodeRef, NodeTag, Operation, Operator,
};
use crate::check::refusal::{CheckCause, CheckRefusal, ExecutionPointRefusal, KeyFault};
use crate::check::state_clause::PopulationDomain;
use crate::model::domain_package::DomainPackageRecord;
use crate::model::intake::member_identity_name;
use crate::model::operation::OperationDeclaration;
use crate::value::member::{AcceptedPoint, Member};
use qsl_forms::StateClauseKind;
use quire_checked::key::DeclarationKey;
use quire_semantic_value::location::{Location, Origin as LocationOrigin};

/// The operation a `pre` or `post` clause names, as lowering reads it.
pub(crate) struct AnchorInput<'a> {
    /// The object type that declares the operation (FR-105: an inherited
    /// operation anchors at its declaring type).
    pub(crate) declaring: EffectiveId,
    /// The operation.
    pub(crate) operation: &'a OperationDeclaration,
    /// A real location of a clause that names this operation, read from the
    /// unit (never [`generated_location`]): the `frame` and
    /// `operation_anchor` nodes carry no source position of their own (the
    /// operation is declared in the domain package, not the unit), so their
    /// nodes are keyed under one clause's own location. It never decides the
    /// node's identity, its dependencies or its ordinal.
    pub(crate) location: &'a Location,
    /// Where the frame's `generated` occurrence for this operation is
    /// placed if this is the first clause or attempt naming it (FR-105
    /// "Outputs"): the operation reference's own position, a real location
    /// of the unit. A clause's is [`state_operation_location`]; an
    /// attempt's is its qualified operation reference; a template's is its
    /// own `location`.
    pub(crate) reference: &'a Location,
}

/// The `Origin::TypeDeclaration` name under which the qualified operation
/// reference (`M::T::op`) of the `pre` or `post` clause at `clause`, its
/// index among the unit's state clauses, is located (FR-105 "Outputs").
/// Spaced, so it names no declared type.
pub(crate) fn state_operation_site(clause: usize) -> String {
    format!("state clause {clause} operation")
}

/// The root location of [`state_operation_site`].
pub(crate) fn state_operation_location(clause: usize) -> Location {
    Location::root(LocationOrigin::TypeDeclaration {
        name: state_operation_site(clause),
    })
}

/// The `Origin::TypeDeclaration` name for state clause `clause`'s authored
/// name token, retained separately from the clause body expression locus.
pub(crate) fn state_clause_name_site(clause: usize) -> String {
    format!("state clause {clause} name")
}

/// The source location of the authored state-clause name token.
pub(crate) fn state_clause_name_location(clause: usize) -> Location {
    Location::root(LocationOrigin::TypeDeclaration {
        name: state_clause_name_site(clause),
    })
}

/// The `Origin::TypeDeclaration` name under which protocol `protocol`'s
/// attempt `attempt` qualified operation reference is located (FR-105
/// Outputs). Spaced, so it names no declared type.
pub(crate) fn protocol_attempt_operation_site(protocol: usize, attempt: usize) -> String {
    format!("protocol {protocol} attempt {attempt} operation")
}

/// The source location of a protocol attempt's qualified operation
/// reference, kept separate from the attempt root used by its other nodes.
pub(crate) fn protocol_attempt_operation_location(protocol: usize, attempt: usize) -> Location {
    Location::root(LocationOrigin::TypeDeclaration {
        name: protocol_attempt_operation_site(protocol, attempt),
    })
}

/// One checked state clause, as lowering reads it.
pub(crate) struct StateClauseInput<'a> {
    /// The clause kind.
    pub(crate) kind: StateClauseKind,
    /// The clause's root location: its `claim` occurrence is recorded here.
    pub(crate) location: &'a Location,
    /// The authored clause-name token, used to locate execution-point
    /// binding refusals precisely.
    pub(crate) name_location: &'a Location,
    /// The context object type `T` of `on M::T`.
    pub(crate) context: EffectiveId,
    /// The operation a `pre` or `post` clause names.
    pub(crate) anchor: Option<AnchorInput<'a>>,
    /// The operation member exactly as written in `on M::T::op`.
    pub(crate) authored_operation_name: Option<&'a str>,
    /// `self`, then `result` when bound, then the operation's parameters,
    /// in slot order.
    pub(crate) parameters: &'a [(String, ValueType)],
    /// The checked Boolean body.
    pub(crate) body: CheckedNode<'a>,
    /// The name each body slot was bound under, indexed by slot.
    pub(crate) body_slots: &'a [String],
    /// The object types whose model nodes name the clause's population
    /// domains.
    pub(crate) population_types: &'a [EffectiveId],
    /// The accepted `ClauseBinding.execution_point` (QSpec FR-341/FR-342,
    /// FR-105-AC-12), or `None` for a clause with no accepted point. It
    /// enters the `state_clause` member and, for an operation clause, the
    /// referenced `operation_anchor` body, so two clauses (or anchors) equal
    /// but for the point are distinct nodes.
    pub(crate) execution_point: Option<AcceptedPoint>,
}

/// Whether `point` is carryable by a `kind` clause whose operation is
/// `operation` (`None` for an invariant).
// The point's operation name and the clause's operation name are both
// identifier text compared once, here.
#[ix_trace_rs::string_edge]
pub(crate) fn check_execution_point(
    kind: StateClauseKind,
    operation: Option<&str>,
    point: &AcceptedPoint,
) -> Result<(), ExecutionPointRefusal> {
    use quire_contract_model::ExecutionPoint;
    let named = |named: &quire_contract_model::ExecutionPointName| match operation {
        Some(operation) if operation == named.as_str() => Ok(()),
        Some(_) | None => Err(ExecutionPointRefusal::OperationMismatch),
    };
    match (kind, point.point()) {
        (
            StateClauseKind::Invariant | StateClauseKind::Constraint,
            ExecutionPoint::Initialization { .. } | ExecutionPoint::Handler { .. },
        ) => Ok(()),
        (StateClauseKind::Precondition, ExecutionPoint::Pre { operation })
        | (StateClauseKind::Postcondition, ExecutionPoint::Post { operation }) => named(operation),
        (
            StateClauseKind::Invariant | StateClauseKind::Constraint,
            ExecutionPoint::Pre { .. } | ExecutionPoint::Post { .. },
        )
        | (
            StateClauseKind::Precondition,
            ExecutionPoint::Initialization { .. }
            | ExecutionPoint::Handler { .. }
            | ExecutionPoint::Post { .. },
        )
        | (
            StateClauseKind::Postcondition,
            ExecutionPoint::Initialization { .. }
            | ExecutionPoint::Handler { .. }
            | ExecutionPoint::Pre { .. },
        ) => Err(ExecutionPointRefusal::InapplicableVariant),
    }
}

/// One protocol `attempt` naming an operation (FR-114), as
/// lowering reads it: no application node of its own, unlike a
/// `state_clause` -- only the operation's anchor and frame identity, by
/// FR-105's shared one-anchor/one-frame-node-per-operation shape (the same
/// `operation_anchor`/`frame_node`/`frame_occurrence` helpers a `pre`/`post`
/// clause's own binding calls).
pub(crate) struct AttemptInput<'a> {
    /// A real location of this attempt (never [`generated_location`]): the
    /// `operation_anchor` node's own `Anchor` occurrence is recorded here
    /// (FR-096 needs a real position to resolve its region).
    ///
    /// [`generated_location`]: super::generated_location
    pub(crate) location: &'a Location,
    /// The operation `on M::T::op` names.
    pub(crate) anchor: AnchorInput<'a>,
    /// The population domain of the type declaring the operation, resolved
    /// the same way a state clause's own `frame_population` is (FR-104
    /// "Requirements"), for the attempt's own `operation-contract` Frame
    /// record (FR-114 "Requirements", reusing `state_clause::record`).
    pub(crate) frame_population: Option<PopulationDomain>,
}

/// The nodes lowering binds one named operation to (FR-105): its
/// `operation_anchor` node, its `frame` node and the `Origin` of the frame's
/// own `generated` occurrence (FR-104). One per (declaring object type,
/// operation name): every clause and attempt naming the operation gets the
/// same three back, and the occurrence stays distinct even from another
/// operation whose frame node holds equal content (FR-104-AC-5).
#[derive(Clone, Debug)]
pub(crate) struct OperationBinding {
    /// The `operation_anchor` node.
    pub(crate) anchor: NodeKey,
    /// The `frame` node.
    pub(crate) frame: NodeKey,
    /// The `Origin` of the frame node's own `generated` occurrence.
    pub(crate) frame_origin: Origin,
}

/// One protocol `attempt`'s lowered keys (FR-114 "Outputs").
pub(crate) struct LoweredAttempt {
    /// The operation's anchor and frame nodes and the frame's occurrence:
    /// the anchor is the identity FR-114 records the attempt's operation
    /// by, and the frame is shared with every clause or attempt naming the
    /// same operation (FR-104-AC-5/FR-114 "Behavior": "SHALL NOT copy the
    /// frame's entries into the attempt").
    pub(crate) binding: OperationBinding,
    /// The model node of `AttemptInput::frame_population`'s own object
    /// type, when the declaring type shares a population.
    pub(crate) population_object: Option<NodeKey>,
    /// The `Boolean` scalar type node: the attempt's own `operation-contract`
    /// record's result type (`state_clause::record`'s own `boolean`
    /// parameter).
    pub(crate) boolean: NodeKey,
}

/// A lowered state clause's keys.
pub(crate) struct LoweredClause {
    /// The `state_clause` node: the clause's identity.
    pub(crate) key: NodeKey,
    /// Each parameter's `value`/`parameter` node, in slot order.
    pub(crate) parameters: Vec<NodeKey>,
    /// The anchor and frame nodes of the operation a `pre` or `post` clause
    /// names, and the frame's occurrence; `None` for an invariant.
    pub(crate) binding: Option<OperationBinding>,
    /// The `Boolean` scalar type node: the clause's semantic type.
    pub(crate) boolean: NodeKey,
    /// The model object type node of each of `population_types`, in order.
    pub(crate) population_objects: Vec<NodeKey>,
}

impl Lowering<'_> {
    /// Lower one resolved tick operation to the same operation anchor and
    /// frame used by state clauses and protocol attempts.
    pub(crate) fn time_source_anchor(
        &mut self,
        anchor: &AnchorInput<'_>,
    ) -> Result<NodeKey, CheckRefusal> {
        let binding = self.operation_anchor(anchor, None, anchor.location)?;
        self.record(
            binding.anchor,
            OccurrenceRole::Anchor,
            anchor.location.clone(),
        );
        Ok(binding.anchor)
    }

    /// Lower one operation an `align skip` list names to the same operation
    /// anchor a state clause naming it references (FR-488 "The node and its
    /// aggregate"), recording an `anchor` occurrence at `anchor.location`.
    pub(crate) fn align_skip_anchor(
        &mut self,
        anchor: &AnchorInput<'_>,
    ) -> Result<NodeKey, CheckRefusal> {
        self.time_source_anchor(anchor)
    }

    /// Lower and key the state clause `clause`, recording its `claim`
    /// occurrence at its root and, for a `pre` or `post` clause, one
    /// `anchor` occurrence of the operation's anchor node.
    pub(crate) fn state_clause(
        &mut self,
        clause: &StateClauseInput<'_>,
    ) -> Result<LoweredClause, CheckRefusal> {
        let location = clause.location;
        if let Some(point) = &clause.execution_point {
            let authored = clause.authored_operation_name;
            let resolved = clause.anchor.as_ref().map(|anchor| anchor.operation.name());
            check_execution_point(clause.kind, authored, point)
                .and_then(|()| {
                    let is_operation_point = matches!(
                        point.point(),
                        quire_contract_model::ExecutionPoint::Pre { .. }
                            | quire_contract_model::ExecutionPoint::Post { .. }
                    );
                    if is_operation_point && authored != resolved {
                        Err(ExecutionPointRefusal::OperationMismatch)
                    } else {
                        Ok(())
                    }
                })
                .map_err(|refusal| CheckRefusal {
                    location: clause.name_location.clone(),
                    cause: CheckCause::ExecutionPointBinding(refusal),
                })?;
        }
        let mut scope = Vec::with_capacity(clause.parameters.len());
        for (level, (name, value_type)) in clause.parameters.iter().enumerate() {
            let type_key = self.binder_type(value_type, None, location)?;
            let key = self.parameter(name, level, type_key, location)?;
            self.record(type_key, OccurrenceRole::Type, location.clone());
            self.record_binder(
                BinderSite {
                    binder: location.clone(),
                    slot: level,
                },
                key,
            );
            scope.push(Binder {
                slot: level,
                parameter: key,
                semantic_type: type_key,
            });
        }
        let parameters: Vec<NodeKey> = scope.iter().map(|binder| binder.parameter).collect();
        let mut binders = Binders::new(clause.body_slots, &scope);
        let condition = self.expression(clause.body, &mut binders)?;
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let (anchor, binding) = match &clause.anchor {
            None => (self.object_node(clause.context, location)?, None),
            Some(anchor) => {
                let binding =
                    self.operation_anchor(anchor, clause.execution_point.as_ref(), location)?;
                self.record(binding.anchor, OccurrenceRole::Anchor, location.clone());
                (binding.anchor, Some(binding))
            }
        };
        // FR-341: a `state_clause` node's body is a `quire.op.state.clause`
        // application whose `state_clause` member names the clause kind and
        // whose three arguments are, in order, the parameter aggregate, the
        // anchor and the condition. `result_type` names the same `Boolean`
        // node as `semantic_type` (FR-341 "the application's `result_type`
        // names the same node").
        let key = self.insert(
            location,
            NodeTag::State,
            "state_clause",
            Some(boolean),
            None,
            BodyTerm::application(
                Operator::StateClause,
                Operation {
                    member: Some(Member::StateClause {
                        clause: clause.kind,
                        execution_point: clause.execution_point.clone(),
                    }),
                    ..Operation::plain("quire.op.state.clause")
                },
                boolean,
                vec![
                    MemberTerm::Group(GroupTerm::references(parameters.iter().copied())),
                    MemberTerm::reference(anchor),
                    MemberTerm::Leaf(condition),
                ],
            ),
        )?;
        self.record(key, OccurrenceRole::Claim, location.clone());
        let mut population_objects = Vec::with_capacity(clause.population_types.len());
        for object in clause.population_types {
            let object = self.object_node(*object, location)?;
            // The requirement names this type even when the clause's nodes
            // do not, as for a population covering a proper subtype.
            self.record(object, OccurrenceRole::Type, location.clone());
            population_objects.push(object);
        }
        Ok(LoweredClause {
            key,
            parameters,
            binding,
            boolean,
            population_objects,
        })
    }

    /// FR-114: bind a protocol `attempt` to its operation's own
    /// `operation_anchor` and `frame` node, reusing exactly the machinery a
    /// `pre`/`post` clause's own binding calls -- no second frame node
    /// concept. Records the same `Anchor` occurrence `state_clause` records
    /// for a clause naming an operation, at `input.location`, and a `Type`
    /// occurrence of each type node the attempt's own record names.
    pub(crate) fn protocol_attempt(
        &mut self,
        input: &AttemptInput<'_>,
    ) -> Result<LoweredAttempt, CheckRefusal> {
        let binding = self.operation_anchor(&input.anchor, None, input.location)?;
        self.record(
            binding.anchor,
            OccurrenceRole::Anchor,
            input.location.clone(),
        );
        // SR-770 FND-002: the attempt's own record names these two nodes
        // (its population domain's object type and its result type), but no
        // node of the attempt does -- unlike a clause, whose `state_clause`
        // node names its `Boolean` -- so neither is reachable for a
        // generated occurrence to be placed under. Each gets a `Type`
        // occurrence at the attempt itself instead, the same role a clause
        // parameter's type node records.
        let population_object = input
            .frame_population
            .map(|domain| self.object_node(domain.object, input.location))
            .transpose()?;
        if let Some(object) = population_object {
            self.record(object, OccurrenceRole::Type, input.location.clone());
        }
        let boolean = self.type_node(&ValueType::Boolean, input.location)?;
        self.record(boolean, OccurrenceRole::Type, input.location.clone());
        Ok(LoweredAttempt {
            binding,
            population_object,
            boolean,
        })
    }

    /// The `operation_anchor` and `frame` nodes of `anchor`'s operation, and
    /// the frame's own occurrence: one of each per (declaring object type,
    /// operation name), however many clauses name it (their contents are
    /// equal, so their keys are, but the occurrence is minted once per
    /// operation identity, not per node -- see [`Lowering::frame_occurrence`]).
    fn operation_anchor(
        &mut self,
        anchor: &AnchorInput<'_>,
        point: Option<&AcceptedPoint>,
        location: &Location,
    ) -> Result<OperationBinding, CheckRefusal> {
        let (declaring, frame) = self.frame_node(anchor, location)?;
        let frame_origin =
            self.frame_occurrence(anchor.declaring, anchor.operation.name(), frame, location)?;
        let operation = self.text_literal(anchor.operation.name(), location)?;
        // FR-342: the optional fourth binding follows `frame`; a point-free
        // anchor (a protocol attempt's, an `align skip`'s) omits it.
        let mut members = vec![
            MemberTerm::bound("context", LeafTerm::reference(declaring)),
            MemberTerm::bound("operation", operation),
            MemberTerm::bound("frame", LeafTerm::reference(frame)),
        ];
        if let Some(point) = point {
            members.push(MemberTerm::binding(
                "execution_point",
                BindingValue::ExecutionPoint(point.clone()),
            ));
        }
        let anchor = self.insert(
            location,
            NodeTag::State,
            "operation_anchor",
            Some(declaring),
            None,
            BodyTerm::aggregate(members),
        )?;
        Ok(OperationBinding {
            anchor,
            frame,
            frame_origin,
        })
    }

    /// `anchor`'s declaring object type node and its `frame` node: content-
    /// addressed over its `modifies`/`creates`/`deletes` and its declaring
    /// type (FR-105), so any clause naming this operation gets the same
    /// pair back, and calling this ahead of a specific clause (from
    /// [`Lowering::register_frame_occurrences`]) costs nothing extra.
    fn frame_node(
        &mut self,
        anchor: &AnchorInput<'_>,
        location: &Location,
    ) -> Result<(NodeKey, NodeKey), CheckRefusal> {
        let declaring = self.object_node(anchor.declaring, location)?;
        let effect = anchor.operation.effect();
        let mut modifies =
            Vec::with_capacity(effect.modifies.len() + effect.receiver_modifies.len());
        for entry in &effect.modifies {
            let (declaration, name) = self.frame_modifies_entry(entry, location)?;
            modifies.push((declaration, name, FrameScope::EveryObject));
        }
        for entry in &effect.receiver_modifies {
            let (declaration, name) = self.frame_field(entry, location)?;
            modifies.push((declaration, Some(name), FrameScope::Receiver));
        }
        // FR-340: entries ascending by (declaring node digest, field name,
        // scope with `every-object` first), a relationship's name sorting as
        // empty, and no member holds a duplicate entry -- two clauses naming
        // the same field twice (or a domain package listing it twice) still
        // produce one `modifies` entry, so the frame's node id matches the
        // deduplicated frame's and IR's reader (which rejects a repeated
        // entry) never sees one.
        modifies.sort();
        modifies.dedup();
        let modifies: Vec<FrameModifies> = modifies
            .into_iter()
            .map(|(declaration, name, scope)| match name {
                Some(name) => FrameModifies::Field(FrameField::new(declaration, name, scope)),
                None => FrameModifies::Relationship(NodeRef(declaration)),
            })
            .collect();
        let creates = self.frame_objects(&effect.creates, location)?;
        let deletes = self.frame_objects(&effect.deletes, location)?;
        // FR-340: `deletes self` first, then each `deletes p` by parameter
        // name as UTF-8 bytes, no entry twice.
        let mut parameters: Vec<&String> = effect.deletes_arguments.iter().collect();
        parameters.sort();
        parameters.dedup();
        let deletes_scoped: Vec<FrameDeletesScoped> = effect
            .deletes_receiver
            .then_some(FrameDeletesScoped::Receiver)
            .into_iter()
            .chain(
                parameters
                    .into_iter()
                    .map(|name| FrameDeletesScoped::Parameter(name.clone())),
            )
            .collect();
        // FR-340: the frame body is no other term at all -- it holds
        // no application, and it never nests inside another term (the
        // schema scopes this shape to a `state`/`frame` node's own body).
        let frame = self.insert(
            location,
            NodeTag::State,
            "frame",
            Some(declaring),
            None,
            BodyTerm::frame(modifies, creates, deletes, deletes_scoped),
        )?;
        Ok((declaring, frame))
    }

    /// Mint every named operation's frame occurrence up front, in ascending
    /// (declaring `DeclarationKey`, operation name as UTF-8 bytes) order
    /// over the distinct operations `operations` names -- never in source
    /// order, so the ordinal an operation's frame record gets does not
    /// depend on which clause of the unit names it first (FR-104
    /// "Requirements", SR-736 FND-008). Two operations sharing one frame
    /// node (equal `modifies`/`creates`/`deletes` and declaring type) still
    /// get two occurrences of it, ordered this way. Call this once, before
    /// lowering any clause.
    pub(crate) fn register_frame_occurrences(
        &mut self,
        operations: &[AnchorInput<'_>],
    ) -> Result<(), CheckRefusal> {
        let mut distinct: BTreeMap<(EffectiveId, String), &AnchorInput<'_>> = BTreeMap::new();
        for anchor in operations {
            distinct
                .entry((anchor.declaring, anchor.operation.name().to_owned()))
                .or_insert(anchor);
        }
        let mut ordered: Vec<(DeclarationKey, String, &AnchorInput<'_>)> =
            Vec::with_capacity(distinct.len());
        for ((declaring, name), anchor) in distinct {
            let key = self.declaration_key_of(declaring, anchor.location)?;
            ordered.push((key, name, anchor));
        }
        ordered.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (_, name, anchor) in ordered {
            // `anchor.reference` is a real position of the first clause or
            // attempt naming the operation (never `generated_location`): the
            // sort above already fixed this operation's ordinal
            // independently of it, so it decides only the occurrence's
            // source region (FR-105 "Outputs").
            let (_, frame) = self.frame_node(anchor, anchor.location)?;
            self.frame_occurrence(anchor.declaring, &name, frame, anchor.reference)?;
        }
        Ok(())
    }

    /// A frame `modifies` entry as its sort key: the node it names and, for
    /// a field member, its member name (a relationship has none).
    fn frame_modifies_entry(
        &mut self,
        entry: &DeclarationKey,
        location: &Location,
    ) -> Result<(NodeKey, Option<String>), CheckRefusal> {
        let (_, model) = self.model_owner(entry, location)?;
        match model.record(entry) {
            Some(DomainPackageRecord::Relationship(_)) => {
                Ok((self.model_node(entry, location)?, None))
            }
            _ => self
                .frame_field(entry, location)
                .map(|(object, name)| (object, Some(name))),
        }
    }

    /// A frame `modifies` field entry: the field member `field`'s declaring
    /// object type node and its member name.
    fn frame_field(
        &mut self,
        field: &DeclarationKey,
        location: &Location,
    ) -> Result<(NodeKey, String), CheckRefusal> {
        let (_, model) = self.model_owner(field, location)?;
        let Some(DomainPackageRecord::FieldMember(member)) = model.record(field) else {
            return Err(fault(location, KeyFault::UnnamedRecordKind(field.clone())));
        };
        let owner = member.owner.clone();
        let name = member_identity_name(&owner.node, &member.key.node)
            .ok_or_else(|| fault(location, KeyFault::UnknownDeclaration(field.clone())))?
            .to_owned();
        let object = self.model_node(&owner, location)?;
        Ok((object, name))
    }

    /// A frame `creates` or `deletes` list: each object type's model node,
    /// ascending by node digest and holding no duplicate entry (FR-340).
    fn frame_objects(
        &mut self,
        objects: &[DeclarationKey],
        location: &Location,
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        let mut nodes = Vec::with_capacity(objects.len());
        for object in objects {
            nodes.push(self.model_node(object, location)?);
        }
        nodes.sort();
        nodes.dedup();
        Ok(nodes)
    }
}

#[cfg(test)]
mod execution_point_tests {
    use super::*;
    use quire_contract_model::{AnchorName, ExecutionPoint, ExecutionPointName};

    fn name(text: &str) -> ExecutionPointName {
        AnchorName::new(text)
            .expect("test anchor name is valid")
            .into()
    }

    fn init(text: &str) -> AcceptedPoint {
        AcceptedPoint::new(ExecutionPoint::Initialization { name: name(text) })
    }

    fn handler(text: &str) -> AcceptedPoint {
        AcceptedPoint::new(ExecutionPoint::Handler { name: name(text) })
    }

    fn pre(text: &str) -> AcceptedPoint {
        AcceptedPoint::new(ExecutionPoint::Pre {
            operation: name(text),
        })
    }

    fn post(text: &str) -> AcceptedPoint {
        AcceptedPoint::new(ExecutionPoint::Post {
            operation: name(text),
        })
    }

    type Outcome = Result<(), ExecutionPointRefusal>;

    /// QSpec FR-341 "Execution point" / FR-342: every (clause kind, point)
    /// pair is either carried or refused with its own typed reason, and a
    /// `pre`/`post` point must name the clause's operation.
    #[test]
    fn each_clause_kind_point_pair_is_admitted_or_refused_typed() {
        use ExecutionPointRefusal::{InapplicableVariant, OperationMismatch};
        use StateClauseKind::{Invariant, Postcondition, Precondition};
        type Case = (
            &'static str,
            StateClauseKind,
            Option<&'static str>,
            AcceptedPoint,
            Outcome,
        );
        let cases: Vec<Case> = vec![
            (
                "invariant initialization",
                Invariant,
                None,
                init("boot"),
                Ok(()),
            ),
            ("invariant handler", Invariant, None, handler("go"), Ok(())),
            (
                "precondition pre",
                Precondition,
                Some("go"),
                pre("go"),
                Ok(()),
            ),
            (
                "postcondition post",
                Postcondition,
                Some("go"),
                post("go"),
                Ok(()),
            ),
            (
                "invariant pre",
                Invariant,
                None,
                pre("go"),
                Err(InapplicableVariant),
            ),
            (
                "invariant post",
                Invariant,
                None,
                post("go"),
                Err(InapplicableVariant),
            ),
            (
                "precondition initialization",
                Precondition,
                Some("go"),
                init("go"),
                Err(InapplicableVariant),
            ),
            (
                "precondition handler",
                Precondition,
                Some("go"),
                handler("go"),
                Err(InapplicableVariant),
            ),
            (
                "precondition post",
                Precondition,
                Some("go"),
                post("go"),
                Err(InapplicableVariant),
            ),
            (
                "postcondition initialization",
                Postcondition,
                Some("go"),
                init("go"),
                Err(InapplicableVariant),
            ),
            (
                "postcondition handler",
                Postcondition,
                Some("go"),
                handler("go"),
                Err(InapplicableVariant),
            ),
            (
                "postcondition pre",
                Postcondition,
                Some("go"),
                pre("go"),
                Err(InapplicableVariant),
            ),
            (
                "precondition naming another operation",
                Precondition,
                Some("go"),
                pre("stop"),
                Err(OperationMismatch),
            ),
            (
                "postcondition naming another operation",
                Postcondition,
                Some("go"),
                post("stop"),
                Err(OperationMismatch),
            ),
            (
                "operation clause with no operation",
                Precondition,
                None,
                pre("go"),
                Err(OperationMismatch),
            ),
        ];
        for (label, kind, operation, point, expected) in cases {
            assert_eq!(
                check_execution_point(kind, operation, &point),
                expected,
                "{label}"
            );
        }
    }
}
