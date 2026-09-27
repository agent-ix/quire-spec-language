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
//! body is FR-340's own non-`SemanticTerm` `frame` term
//! ([`SemanticTerm::Frame`]). The key covers the clause kind, its anchor and
//! its checked body and nothing else: two declarations with equal kind,
//! anchor and body share one node and differ in their `claim` occurrence
//! (FR-088, FR-104-AC-6), and the declared name enters no key.

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, NodeKey, Origin, ValueType};

use super::{fault, Binder, Binders, Lowering};
use crate::check::claims::BinderSite;
use crate::check::family::OccurrenceRole;
use crate::check::ir::Node;
use crate::check::node_key::{FrameField, NodeRef, NodeTag, Operation, Operator, SemanticTerm};
use crate::check::refusal::{CheckRefusal, KeyFault, Location};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::intake::member_identity_name;
use crate::model::key::DeclarationKey;
use crate::value::declaration::OperationDeclaration;
use crate::value::member::Member;
use qsl_forms::StateClauseKind;

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
    /// `generated` occurrence needs one clause's own location to resolve a
    /// region at all (FR-096; the SR-751 round-2 finding this field fixes,
    /// `EmitRefusal::UnlocatedOccurrence` at every clause naming an
    /// operation). Which clause's location is used, when several name the
    /// same operation, is unconstrained by FR-105: it decides only the
    /// source-map region the occurrence is placed at, never the node's
    /// identity, its dependencies or its ordinal.
    pub(crate) location: &'a Location,
}

/// One checked state clause, as lowering reads it.
pub(crate) struct StateClauseInput<'a> {
    /// The clause kind.
    pub(crate) kind: StateClauseKind,
    /// The clause's root location: its `claim` occurrence is recorded here.
    pub(crate) location: &'a Location,
    /// The context object type `T` of `on M::T`.
    pub(crate) context: EffectiveId,
    /// The operation a `pre` or `post` clause names.
    pub(crate) anchor: Option<AnchorInput<'a>>,
    /// `self`, then `result` when bound, then the operation's parameters,
    /// in slot order.
    pub(crate) parameters: &'a [(String, ValueType)],
    /// The checked Boolean body.
    pub(crate) body: &'a Node,
    /// The name each body slot was bound under, indexed by slot.
    pub(crate) body_slots: &'a [String],
    /// The object types whose model nodes name the clause's population
    /// domains.
    pub(crate) population_types: &'a [EffectiveId],
}

/// A lowered state clause's keys.
pub(crate) struct LoweredClause {
    /// The `state_clause` node: the clause's identity.
    pub(crate) key: NodeKey,
    /// Each parameter's `value`/`parameter` node, in slot order.
    pub(crate) parameters: Vec<NodeKey>,
    /// The `frame` node of the operation a `pre` or `post` clause names,
    /// and the `Origin` of its own occurrence: one per named operation,
    /// shared by every clause that names it, distinct even from another
    /// operation whose frame node happens to hold equal content
    /// (FR-104-AC-5).
    pub(crate) frame: Option<(NodeKey, Origin)>,
    /// The `Boolean` scalar type node: the clause's semantic type.
    pub(crate) boolean: NodeKey,
    /// The model object type node of each of `population_types`, in order.
    pub(crate) population_objects: Vec<NodeKey>,
}

/// The FR-105 `<kind>` spelling of a state clause.
fn kind_spelling(kind: StateClauseKind) -> &'static str {
    match kind {
        StateClauseKind::Invariant => "invariant",
        StateClauseKind::Precondition => "precondition",
        StateClauseKind::Postcondition => "postcondition",
    }
}

impl Lowering<'_> {
    /// Lower and key the state clause `clause`, recording its `claim`
    /// occurrence at its root and, for a `pre` or `post` clause, one
    /// `anchor` occurrence of the operation's anchor node.
    pub(crate) fn state_clause(
        &mut self,
        clause: &StateClauseInput<'_>,
    ) -> Result<LoweredClause, CheckRefusal> {
        let location = clause.location;
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
        let mut binders = Binders {
            slot_names: clause.body_slots,
            scope,
        };
        let condition = self.expression(clause.body, &mut binders)?;
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let (anchor, frame) = match &clause.anchor {
            None => (self.object_node(clause.context, location)?, None),
            Some(anchor) => {
                let (anchor, frame, frame_origin) = self.operation_anchor(anchor, location)?;
                self.record(anchor, OccurrenceRole::Anchor, location.clone());
                (anchor, Some((frame, frame_origin)))
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
            SemanticTerm::Application {
                operator: Operator::StateClause,
                operation: Operation {
                    member: Some(Member::StateClause {
                        clause: kind_spelling(clause.kind),
                    }),
                    ..Operation::plain("quire.op.state.clause")
                },
                result_type: NodeRef(boolean),
                arguments: vec![
                    SemanticTerm::Aggregate {
                        members: parameters
                            .iter()
                            .map(|parameter| SemanticTerm::reference(*parameter))
                            .collect(),
                    },
                    SemanticTerm::reference(anchor),
                    condition,
                ],
            },
        )?;
        self.record(key, OccurrenceRole::Claim, location.clone());
        let mut population_objects = Vec::with_capacity(clause.population_types.len());
        for object in clause.population_types {
            population_objects.push(self.object_node(*object, location)?);
        }
        Ok(LoweredClause {
            key,
            parameters,
            frame,
            boolean,
            population_objects,
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
        location: &Location,
    ) -> Result<(NodeKey, NodeKey, Origin), CheckRefusal> {
        let (declaring, frame) = self.frame_node(anchor, location)?;
        let frame_origin =
            self.frame_occurrence(anchor.declaring, anchor.operation.name(), frame, location)?;
        let operation = self.text_literal(anchor.operation.name(), location)?;
        let anchor = self.insert(
            location,
            NodeTag::State,
            "operation_anchor",
            Some(declaring),
            None,
            SemanticTerm::Aggregate {
                members: vec![
                    SemanticTerm::binding("context", SemanticTerm::reference(declaring)),
                    SemanticTerm::binding("operation", operation),
                    SemanticTerm::binding("frame", SemanticTerm::reference(frame)),
                ],
            },
        )?;
        Ok((anchor, frame, frame_origin))
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
        let mut modifies = Vec::with_capacity(effect.modifies.len());
        for field in &effect.modifies {
            modifies.push(self.frame_field(field, location)?);
        }
        // FR-340: entries ascending by (declaring node digest, field name).
        modifies.sort();
        let modifies: Vec<FrameField> = modifies
            .into_iter()
            .map(|(object, name)| FrameField::new(object, name))
            .collect();
        let creates = self.frame_objects(&effect.creates, location)?;
        let deletes = self.frame_objects(&effect.deletes, location)?;
        // FR-340: the frame body is not a `SemanticTerm` at all -- it holds
        // no application, and it never nests inside another term (the
        // schema scopes this shape to a `state`/`frame` node's own body).
        let frame = self.insert(
            location,
            NodeTag::State,
            "frame",
            Some(declaring),
            None,
            SemanticTerm::frame(modifies, creates, deletes),
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
            // `anchor.location` is a real clause location (never
            // `generated_location`): the sort above already fixed this
            // operation's ordinal independently of it, so which clause's
            // location is used here decides only the occurrence's source
            // region.
            let (_, frame) = self.frame_node(anchor, anchor.location)?;
            self.frame_occurrence(anchor.declaring, &name, frame, anchor.location)?;
        }
        Ok(())
    }

    /// A frame `modifies` entry: the field member `field`'s declaring
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
    /// ascending by node digest.
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
        Ok(nodes)
    }
}
