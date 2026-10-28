// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-105 "Boolean model-field transitions": the derived
//! `state`/`transition` node (QSpec FR-341 "State transition body") of an
//! operation and a Boolean model field, and the derived `state_clause` nodes
//! that bind several authored preconditions or postconditions by conjunction.
//!
//! A transition is not a source construct. Every authored clause keeps its
//! own node and occurrences; this pass reads the lowered clauses and adds
//! nodes only, for exactly the (operation, field) pairs FR-105 names as
//! eligible. An ineligible pair gets nothing and is not refused.

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::{EffectiveId, Identifier, NodeKey, ValueType};

use super::state::OperationBinding;
use super::{fault, refuse, LoweredClause, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{
    BindingValue, BodyTerm, GroupTerm, LawRole, LeafTerm, MemberTerm, NodeTag, Operation, Operator,
};
use crate::check::refusal::{CheckCause, CheckRefusal, KeyFault};
use crate::model::domain_package::{DomainPackageRecord, NativeValueType, ValueTypeRef};
use crate::model::operation::OperationDeclaration;
use crate::value::member::Member;
use qsl_forms::StateClauseKind;
use quire_checked::key::DeclarationKey;
use quire_semantic_value::location::Location;

/// One lowered `pre` or `post` clause, as the transition pass reads it.
pub(crate) struct TransitionClause<'a> {
    /// `Precondition` or `Postcondition`; any other kind is never passed.
    pub(crate) kind: StateClauseKind,
    /// The authored clause name: the conjunction order (FR-105 "`post`").
    pub(crate) name: &'a str,
    /// The byte offset of the declaration: source order.
    pub(crate) source_order: usize,
    /// The clause's root location, where its `claim` occurrence is recorded.
    pub(crate) location: &'a Location,
    /// The context type `T` of `on M::T`.
    pub(crate) context: EffectiveId,
    /// The type declaring the operation the clause names.
    pub(crate) declaring: EffectiveId,
    /// The operation the clause names.
    pub(crate) operation: &'a OperationDeclaration,
    /// Whether the clause carries an accepted execution point.
    pub(crate) has_execution_point: bool,
    /// The clause's lowered keys.
    pub(crate) lowered: &'a LoweredClause,
}

/// The distinct nodes of one clause set, each with the declarations that
/// check to it (FR-105: counted by node id, not by declaration).
struct ClauseSet<'a> {
    /// Ascending by (least clause name, node).
    nodes: Vec<(NodeKey, Vec<&'a TransitionClause<'a>>)>,
}

impl<'a> ClauseSet<'a> {
    fn new<'c>(clauses: impl IntoIterator<Item = &'c &'a TransitionClause<'a>>) -> Self
    where
        'a: 'c,
    {
        let mut by_node: BTreeMap<NodeKey, Vec<&'a TransitionClause<'a>>> = BTreeMap::new();
        for clause in clauses {
            by_node.entry(clause.lowered.key).or_default().push(clause);
        }
        let mut nodes: Vec<_> = by_node.into_iter().collect();
        nodes.sort_by(|(left_key, left), (right_key, right)| {
            let least = |clauses: &[&TransitionClause<'_>]| {
                clauses
                    .iter()
                    .map(|clause| clause.name)
                    .min()
                    .map(str::to_owned)
            };
            least(left)
                .cmp(&least(right))
                .then_with(|| left_key.cmp(right_key))
        });
        Self { nodes }
    }

    fn keys(&self) -> Vec<NodeKey> {
        self.nodes.iter().map(|(key, _)| *key).collect()
    }

    /// Every contributing declaration, in source order.
    fn declarations(&self) -> Vec<&'a TransitionClause<'a>> {
        let mut all: Vec<_> = self
            .nodes
            .iter()
            .flat_map(|(_, clauses)| clauses.iter().copied())
            .collect();
        all.sort_by_key(|clause| clause.source_order);
        all
    }
}

/// The derived nodes built so far. Derived nodes are content-keyed: two
/// subjects with the same clause set share one node, and its occurrences are
/// recorded once.
#[derive(Default)]
struct Built {
    /// Each derived `state_clause`, by its kind and contributing nodes.
    clauses: BTreeMap<(StateClauseKind, Vec<NodeKey>), NodeKey>,
    /// Each `and` of two conditions.
    conjunctions: BTreeMap<(NodeKey, NodeKey), NodeKey>,
}

/// What `field_transition` reads of one operation.
struct OperationClauses<'a, 'c> {
    object: NodeKey,
    binding: &'a OperationBinding,
    operation: &'a OperationDeclaration,
    posts: &'a [&'a &'c TransitionClause<'c>],
    pres: &'a [&'a &'c TransitionClause<'c>],
}

impl Lowering<'_> {
    /// Emit the `state`/`transition` node of every eligible (operation,
    /// Boolean model field) pair of `clauses`, and the derived `state_clause`
    /// nodes each needs (FR-105 "Boolean model-field transitions").
    pub(crate) fn state_transitions(
        &mut self,
        clauses: &[TransitionClause<'_>],
    ) -> Result<(), CheckRefusal> {
        // The lock must select the protocol profile (QSpec FR-341 `laws`).
        if self.lock.protocol_profile().is_none() {
            return Ok(());
        }
        let mut operations: BTreeMap<(EffectiveId, &str), Vec<&TransitionClause<'_>>> =
            BTreeMap::new();
        for clause in clauses {
            operations
                .entry((clause.declaring, clause.operation.name()))
                .or_default()
                .push(clause);
        }
        let mut built = Built::default();
        for (_, group) in operations {
            self.operation_transitions(&group, &mut built)?;
        }
        Ok(())
    }

    fn operation_transitions<'c>(
        &mut self,
        group: &[&'c TransitionClause<'c>],
        built: &mut Built,
    ) -> Result<(), CheckRefusal> {
        let Some(first) = group.first() else {
            return Ok(());
        };
        let declaring = first.declaring;
        let operation = first.operation;
        // No clause of the operation carries an execution point, and none
        // names the operation through a proper subtype of its declaring type.
        if group
            .iter()
            .any(|clause| clause.has_execution_point || clause.context != declaring)
        {
            return Ok(());
        }
        // QSpec FR-341 `inputs`: every parameter is Boolean.
        if operation
            .parameters()
            .iter()
            .any(|(_, value_type)| *value_type != ValueType::Boolean)
        {
            return Ok(());
        }
        let posts: Vec<&&TransitionClause<'_>> = group
            .iter()
            .filter(|clause| clause.kind == StateClauseKind::Postcondition)
            .collect();
        let pres: Vec<&&TransitionClause<'_>> = group
            .iter()
            .filter(|clause| clause.kind == StateClauseKind::Precondition)
            .collect();
        if posts.is_empty() {
            return Ok(());
        }
        let Some(binding) = first.lowered.binding.as_ref() else {
            return Ok(());
        };
        let location = first.location;
        let object = self.object_node(declaring, location)?;

        let effect = operation.effect();
        let mut modifies: Vec<&DeclarationKey> = effect
            .modifies
            .iter()
            .chain(&effect.receiver_modifies)
            .collect();
        modifies.sort();
        modifies.dedup();
        let mut fields: BTreeMap<String, ()> = BTreeMap::new();
        for entry in modifies {
            if !self.boolean_required_field(entry, location)? {
                continue;
            }
            let (owner, name) = self.frame_field(entry, location)?;
            // The subject projects the field of the anchor's context itself.
            if owner == object {
                fields.insert(name, ());
            }
        }
        for name in fields.into_keys() {
            let clauses = OperationClauses {
                object,
                binding,
                operation,
                posts: &posts,
                pres: &pres,
            };
            self.field_transition(&name, &clauses, built)?;
        }
        Ok(())
    }

    /// Whether `entry` names a required, single-valued `Boolean` field.
    fn boolean_required_field(
        &self,
        entry: &DeclarationKey,
        location: &Location,
    ) -> Result<bool, CheckRefusal> {
        let (_, model) = self.model_owner(entry, location)?;
        Ok(matches!(
            model.record(entry),
            Some(DomainPackageRecord::FieldMember(field))
                if matches!(field.value_type, ValueTypeRef::Native(NativeValueType::Boolean))
                    && field.presence == quire_exact::Presence::Required
                    && field.multiplicity.lower == 1
                    && field.multiplicity.upper == Some(1)
        ))
    }

    fn field_transition(
        &mut self,
        field: &str,
        clauses: &OperationClauses<'_, '_>,
        built: &mut Built,
    ) -> Result<(), CheckRefusal> {
        let OperationClauses {
            object,
            binding,
            operation,
            posts,
            pres,
        } = *clauses;
        // Q: the postconditions whose condition holds the subject node.
        let mut subject = None;
        let mut constraining = Vec::new();
        for clause in posts {
            let Some(&self_parameter) = clause.lowered.parameters.first() else {
                continue;
            };
            let mut visited = BTreeSet::new();
            if let Some(found) = self.subject_read(
                clause.lowered.condition,
                self_parameter,
                object,
                field,
                &mut visited,
            ) {
                subject.get_or_insert(found);
                constraining.push(*clause);
            }
        }
        let Some(subject) = subject else {
            return Ok(());
        };
        let post_set = ClauseSet::new(constraining.iter().copied());
        let pre_set = ClauseSet::new(pres.iter().copied());
        let location = post_set
            .declarations()
            .first()
            .map(|clause| clause.location)
            .ok_or_else(|| {
                fault(
                    posts[0].location,
                    KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
                        "S4",
                        "transition-has-a-postcondition",
                    )),
                )
            })?;
        let boolean = self.type_node(&ValueType::Boolean, location)?;

        let post = self.clause_for_set(
            &post_set,
            StateClauseKind::Postcondition,
            binding,
            boolean,
            location,
            built,
        )?;
        let pre = if pre_set.nodes.is_empty() {
            Vec::new()
        } else {
            vec![self.clause_for_set(
                &pre_set,
                StateClauseKind::Precondition,
                binding,
                boolean,
                location,
                built,
            )?]
        };
        // The operation's parameters, in declared order: the last entries of
        // every clause's parameter aggregate.
        let parameters = &posts[0].lowered.parameters;
        let inputs: Vec<NodeKey> = parameters
            .iter()
            .skip(
                parameters
                    .len()
                    .saturating_sub(operation.parameters().len()),
            )
            .copied()
            .collect();

        let law = self.law(LawRole::ProtocolProfile, location)?;
        let operator = Identifier::new("transition".to_owned()).map_err(|_| {
            refuse(
                location,
                CheckCause::NodePreimage(
                    crate::check::node_key::NodeKeyRefusal::EmptyQualifiedName,
                ),
            )
        })?;
        let key = self.insert(
            location,
            NodeTag::State,
            "transition",
            Some(boolean),
            None,
            BodyTerm::application(
                Operator::StateTransition,
                Operation {
                    laws: vec![law],
                    member: Some(Member::ProfileOperator { operator }),
                    ..Operation::plain("quire.op.state.transition")
                },
                boolean,
                vec![
                    MemberTerm::bound("anchor", LeafTerm::reference(binding.anchor)),
                    MemberTerm::bound("subject", LeafTerm::reference(subject)),
                    MemberTerm::binding(
                        "inputs",
                        BindingValue::Group(GroupTerm::references(inputs)),
                    ),
                    MemberTerm::binding("pre", BindingValue::Group(GroupTerm::references(pre))),
                    MemberTerm::bound("post", LeafTerm::reference(post)),
                ],
            ),
        )?;
        for clause in post_set.declarations() {
            self.record(key, OccurrenceRole::Claim, clause.location.clone());
        }
        Ok(())
    }

    /// The node `set` is bound to: its one clause node, or a derived
    /// `state_clause` whose condition is the left fold of
    /// `quire.op.boolean.and` over the clauses' conditions in ascending
    /// clause-name order.
    fn clause_for_set(
        &mut self,
        set: &ClauseSet<'_>,
        kind: StateClauseKind,
        binding: &OperationBinding,
        boolean: NodeKey,
        location: &Location,
        built: &mut Built,
    ) -> Result<NodeKey, CheckRefusal> {
        let keys = set.keys();
        if let [only] = keys.as_slice() {
            return Ok(*only);
        }
        if let Some(existing) = built.clauses.get(&(kind, keys.clone())) {
            return Ok(*existing);
        }
        let mut members = set.nodes.iter();
        let Some((_, first)) = members.next() else {
            return Err(fault(
                location,
                KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
                    "S4",
                    "derived-clause-set-is-not-empty",
                )),
            ));
        };
        let lead = first[0];
        let mut condition = lead.lowered.condition;
        for (_, clauses) in members {
            let next = clauses[0];
            let right = next.lowered.condition;
            condition = match built.conjunctions.get(&(condition, right)) {
                Some(existing) => *existing,
                None => {
                    let leaf = self.typed_application(
                        next.location,
                        boolean,
                        Operator::Binary,
                        Operation::plain("quire.op.boolean.and"),
                        vec![
                            MemberTerm::reference(condition),
                            MemberTerm::reference(right),
                        ],
                    )?;
                    let LeafTerm::Reference { target } = leaf else {
                        return Err(fault(
                            next.location,
                            KeyFault::StageFault(
                                quire_semantic_value::catalog::InternalFault::new(
                                    "S4",
                                    "conjunction-is-a-node",
                                ),
                            ),
                        ));
                    };
                    built.conjunctions.insert((condition, right), target.0);
                    target.0
                }
            };
        }
        let location = lead.location;
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
                        clause: kind,
                        execution_point: None,
                    }),
                    ..Operation::plain("quire.op.state.clause")
                },
                boolean,
                vec![
                    MemberTerm::Group(GroupTerm::references(
                        lead.lowered.parameters.iter().copied(),
                    )),
                    MemberTerm::reference(binding.anchor),
                    MemberTerm::Leaf(LeafTerm::reference(condition)),
                ],
            ),
        )?;
        for clause in set.declarations() {
            self.record(key, OccurrenceRole::Claim, clause.location.clone());
        }
        built.clauses.insert((kind, keys), key);
        Ok(key)
    }

    // The only string compare is the field's name against the member's wire
    // spelling; the operator and operation are matched by their typed enum.
    #[ix_trace_rs::string_edge]
    /// The subject node `deref(self).field` when the expression rooted at
    /// `node` holds it outside every `quire.op.state.pre` application.
    fn subject_read(
        &self,
        node: NodeKey,
        self_parameter: NodeKey,
        object: NodeKey,
        field: &str,
        visited: &mut BTreeSet<NodeKey>,
    ) -> Option<NodeKey> {
        if !visited.insert(node) {
            return None;
        }
        let semantic = self.graph.node(node)?;
        let application = semantic.content.body.application_term()?;
        if semantic.content.node_tag != NodeTag::Expression {
            return None;
        }
        if application.operator == Operator::Pre {
            return None;
        }
        if application.operator == Operator::Query
            && matches!(
                &application.operation.member,
                Some(Member::Field { declaration, name })
                    if *declaration == object && name.as_str() == field
            )
            && self.is_deref_of(application.arguments.first(), self_parameter)
        {
            return Some(node);
        }
        application
            .arguments
            .iter()
            .flat_map(MemberTerm::leaves)
            .find_map(|leaf| match leaf {
                LeafTerm::Reference { target } => {
                    self.subject_read(target.0, self_parameter, object, field, visited)
                }
                LeafTerm::Literal { .. } | LeafTerm::DependencyReference { .. } => None,
            })
    }

    /// Whether `argument` is the reference to `quire.op.model.deref` of
    /// the `self` parameter node.
    fn is_deref_of(&self, argument: Option<&MemberTerm>, self_parameter: NodeKey) -> bool {
        let Some(MemberTerm::Leaf(LeafTerm::Reference { target })) = argument else {
            return false;
        };
        let Some(application) = self
            .graph
            .node(target.0)
            .and_then(|node| node.content.body.application_term())
        else {
            return false;
        };
        application.operator == Operator::Deref
            && matches!(
                application.arguments.first(),
                Some(MemberTerm::Leaf(LeafTerm::Reference { target })) if target.0 == self_parameter
            )
    }
}
