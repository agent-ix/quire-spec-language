// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-302: the unit compile links dispatch for each operation a dispatched
//! call names.
//!
//! [`PackageDeclarations::link_dispatch`] is the production caller of
//! [`checked_dispatch_operation`], the S3 seam. For every distinct member
//! name a `receiver.member(args)` call in the unit's state clauses (or kept
//! operation clauses) names, it picks the family roots of that name in the
//! selected model views, supplies the bridge the data it asks for, keyed by
//! each operation's [`DeclarationKey`], and merges what the bridge returns
//! (clause functions, call sites and linked tables) into the package before
//! [`PackageDeclarations::check`] types the calls against them.
//!
//! What the bridge is given is the unit's own: an operation's body from its
//! `operation-body` declaration ([`PackageDeclarations::operation_body`]),
//! its precondition from the unit `pre` clauses that resolved to it
//! ([`PackageDeclarations::unit_clauses_for`]), and its signature from the
//! operation table. The domain package's own `requires`/`ensures` text is
//! never read (FR-103): it is the operation's contract, not a body or a unit
//! clause.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{BinaryOperator, DeclaredClauseKind, ExprNode, Expression, StateClauseKind};
use qsl_foundation::diagnostic::{CatalogCode, Code};
use qsl_foundation::Span;
use quire_exact::{EffectiveId, IllTypedCause, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::check::DispatchOperation;
use super::checked_dispatch::{
    checked_dispatch_operation, rename_names, DispatchBridgeRefusal, DispatchRoot, OperationClauses,
};
use super::ir::{DispatchCandidate, DispatchTable};
use super::refusal::{DispatchLinkCause, DispatchLinkLimit, DispatchSubtypeFailure};
use super::{CheckCause, CheckRefusal, PackageDeclarations};
use crate::model::accounting::{Meter, ModelNormalizationLimits};
use crate::model::dispatch::{DispatchLinkOutcome, GeneralizationClosure, LinkCheckOutcome};
use crate::model::domain_package::{DomainPackageRecord, OperationMemberRecord};
use crate::model::intake::member_identity_name;
use crate::model::key::DeclarationKey;
use crate::model::normalize::{EffectiveView, ModelRefusal};

/// One called family: the alias of the selected model that declares it and
/// the key of its root operation, an operation that redefines none.
struct CalledFamily {
    alias: String,
    root: DeclarationKey,
}

impl PackageDeclarations {
    /// FR-302: links the dispatch table of each operation the unit's
    /// dispatched calls name, under `limits`' `family_steps` ceiling and the
    /// other model normalization counters, one meter for the whole unit.
    ///
    /// Every called family is attempted, so the returned refusals name every
    /// family that failed; none leaves a partial table behind, because the
    /// caller refuses the compile on any. A body whose parameter count
    /// differs from its operation's refuses `ill_typed`/`type-mismatch`
    /// located at the body, when linking first reads it.
    pub fn link_dispatch(
        &mut self,
        limits: ModelNormalizationLimits,
    ) -> Result<(), Vec<CheckRefusal>> {
        let called = self.called_members();
        if called.is_empty() {
            return Ok(());
        }
        let mut meter = Meter::new(limits);
        let mut refusals = Vec::new();
        let mut linked = Vec::new();
        for family in self.called_families(&called) {
            let Some(view) = self.selected_views.get(&family.alias) else {
                continue;
            };
            let clauses = self.family_clauses(view, &family.root);
            let root = DispatchRoot {
                key: family.root.clone(),
                closure: GeneralizationClosure::Closed,
            };
            match checked_dispatch_operation(
                view,
                &root,
                &clauses,
                self.source.clone(),
                &mut meter,
                self.identity,
            ) {
                Ok(declarations) => {
                    linked.push((declarations, CallSite::of(&family.root, &clauses)))
                }
                Err(refusal) => refusals.push(CheckRefusal {
                    location: Location::root(Origin::Expression),
                    cause: CheckCause::DispatchLink(Box::new(link_cause(family.root, &refusal))),
                }),
            }
        }
        let first_new = self.functions.len();
        for (declarations, call) in linked {
            self.merge_linked(declarations, call);
        }
        refusals.extend(self.body_arity_refusals(first_new));
        if refusals.is_empty() {
            Ok(())
        } else {
            Err(refusals)
        }
    }

    /// The member names of the dispatched calls in the unit's state clauses
    /// and in the kept operation clauses: the only places a dispatched call
    /// checks (FR-151).
    fn called_members(&self) -> BTreeSet<String> {
        let expressions = self.state_clauses.iter().map(|clause| &clause.body).chain(
            self.operation_clauses
                .iter()
                .map(|clause| &clause.clause.expression),
        );
        let mut called = BTreeSet::new();
        for expression in expressions {
            for node in expression.iter() {
                if let ExprNode::Dispatch { member, .. } = node.node() {
                    called.insert(member.clone());
                }
            }
        }
        called
    }

    /// The family root of each called member name in each selected model, in
    /// alias then key order: the operations of that name that redefine none.
    fn called_families(&self, called: &BTreeSet<String>) -> Vec<CalledFamily> {
        let mut families = Vec::new();
        for (alias, view) in &self.selected_views {
            let mut roots: Vec<&DeclarationKey> = view
                .domain_package()
                .records
                .iter()
                .filter_map(|record| match record {
                    DomainPackageRecord::OperationMember(operation)
                        if operation.redefines.is_none()
                            && member_identity_name(&operation.owner.node, &operation.key.node)
                                .is_some_and(|name| called.contains(name)) =>
                    {
                        Some(&operation.key)
                    }
                    _ => None,
                })
                .collect();
            roots.sort();
            families.extend(roots.into_iter().map(|root| CalledFamily {
                alias: alias.clone(),
                root: root.clone(),
            }));
        }
        families
    }

    /// What the bridge reads of every operation in the family of `root`: the
    /// signature from the operation table, the body from the unit's
    /// `operation-body` declaration and the precondition from the unit's
    /// `pre` clauses, each keyed by the operation's `DeclarationKey`.
    fn family_clauses(&self, view: &EffectiveView, root: &DeclarationKey) -> OperationClauses {
        let operations: Vec<&OperationMemberRecord> = view
            .domain_package()
            .records
            .iter()
            .filter_map(|record| match record {
                DomainPackageRecord::OperationMember(operation) => Some(operation),
                _ => None,
            })
            .collect();
        let mut family: BTreeSet<&DeclarationKey> = BTreeSet::from([root]);
        loop {
            let before = family.len();
            for operation in &operations {
                if operation
                    .redefines
                    .as_ref()
                    .is_some_and(|redefined| family.contains(redefined))
                {
                    family.insert(&operation.key);
                }
            }
            if family.len() == before {
                break;
            }
        }
        let mut clauses = OperationClauses::default();
        for operation in operations
            .into_iter()
            .filter(|operation| family.contains(&operation.key))
        {
            let key = &operation.key;
            let Some(owner) = view.type_identities().get(&operation.owner).copied() else {
                continue;
            };
            let Some(name) = member_identity_name(&operation.owner.node, &key.node) else {
                continue;
            };
            let Some(declaration) = self
                .operations
                .declared_by(owner)
                .iter()
                .find(|declaration| declaration.name() == name)
            else {
                continue;
            };
            if key == root {
                clauses.member.insert(key.clone(), name.to_owned());
            }
            let signature: Vec<(String, ValueType)> =
                std::iter::once(("self".to_owned(), ValueType::Reference(owner)))
                    .chain(declaration.parameters().iter().cloned())
                    .collect();
            if let Some(result) = declaration.result() {
                clauses.result.insert(key.clone(), result.clone());
            }
            if let Some(body) = self.operation_body(key) {
                // The body names its parameters as it wrote them, in the
                // order of the operation's; the bridge's function names them
                // as the operation does.
                let rename: BTreeMap<String, String> = body
                    .parameters
                    .iter()
                    .zip(signature.iter().skip(1))
                    .filter(|(written, (declared, _))| &written.name != declared)
                    .map(|(written, (declared, _))| (written.name.clone(), declared.clone()))
                    .collect();
                clauses.own_body.insert(
                    key.clone(),
                    rename_names(&receiver_as_parameter(&body.body), &rename),
                );
            }
            let precondition = self
                .unit_clauses_for(key)
                .into_iter()
                .filter(|clause| clause.kind == StateClauseKind::Precondition)
                .map(|clause| receiver_as_parameter(&clause.body))
                .reduce(|left, right| Expression::binary(BinaryOperator::And, left, right));
            if let Some(precondition) = precondition {
                clauses.own_precondition.insert(key.clone(), precondition);
            }
            clauses.parameters.insert(key.clone(), signature);
        }
        clauses
    }

    /// Appends what the bridge built for one family: its functions after the
    /// package's own (a source function keeps its index), its call sites and
    /// its table, with every function index and table index the bridge
    /// numbered from zero moved to its place in the package. A body function
    /// carries the spans of the `operation-body` it came from, so a
    /// refusal at the body resolves to its source region (FR-096).
    fn merge_linked(&mut self, linked: PackageDeclarations, call: Option<CallSite>) {
        let function_base = self.functions.len();
        let table_base = self.dispatch_tables.len();
        let PackageDeclarations {
            functions,
            dispatch_operations,
            dispatch_tables,
            resolved_signatures,
            model_clauses,
            ..
        } = linked;
        for (offset, function) in functions.into_iter().enumerate() {
            let index = function_base + offset;
            if let Some(signature) = resolved_signatures.get(offset) {
                self.resolved_signatures.insert(index, signature.clone());
            }
            let clause = model_clauses.get(&offset);
            let spans = clause
                .filter(|clause| clause.kind == DeclaredClauseKind::Body)
                .and_then(|clause| self.operation_body(&clause.declaration))
                .map(|body| body.spans.clone());
            // A precondition function the bridge synthesized from the unit's
            // `pre` clauses (one clause, their conjunction, or the
            // disjunction with a redefined operation's) has no tree of spans
            // to match its shape: it is located at those clauses.
            if let Some(span) = clause
                .filter(|clause| clause.kind == DeclaredClauseKind::Precondition)
                .and_then(|clause| self.precondition_span(&clause.declaration))
            {
                self.synthesized_spans.insert(index, span);
            }
            self.functions.push(match spans {
                Some(spans) => function.clone().with_spans(spans).unwrap_or(function),
                None => function,
            });
        }
        for (offset, clause) in model_clauses {
            self.model_clauses.insert(function_base + offset, clause);
        }
        self.dispatch_operations
            .extend(
                dispatch_operations
                    .into_iter()
                    .map(|operation| DispatchOperation {
                        table: operation.table + table_base,
                        ..operation
                    }),
            );
        let mut exposed: BTreeSet<EffectiveId> = self
            .dispatch_operations
            .iter()
            .filter(|operation| operation.table == table_base)
            .map(|operation| operation.receiver_type)
            .collect();
        for table in dispatch_tables {
            // A call through any type the family's table links resolves to
            // that table, not only through a type whose own member is the
            // root: a subtype that redefines the operation is reached by the
            // same name, and no other call exposes it.
            if let Some(call) = &call {
                let receivers = std::iter::once(call.owner)
                    .chain(table.entries().iter().map(|(subtype, _)| *subtype));
                for receiver_type in receivers {
                    if exposed.insert(receiver_type) {
                        self.dispatch_operations.push(DispatchOperation {
                            receiver_type,
                            member: call.member.clone(),
                            parameters: call.parameters.clone(),
                            result: call.result.clone(),
                            table: table_base,
                        });
                    }
                }
            }
            let entries = table
                .entries()
                .iter()
                .map(|(subtype, candidate)| {
                    (
                        *subtype,
                        DispatchCandidate {
                            body: candidate.body + function_base,
                            precondition: candidate.precondition.map(|index| index + function_base),
                            precondition_clauses: candidate
                                .precondition_clauses
                                .iter()
                                .map(|index| index + function_base)
                                .collect(),
                        },
                    )
                })
                .collect();
            self.dispatch_tables
                .push(DispatchTable::new(entries, table.candidate_count()));
        }
    }

    /// The span covering the unit `pre` clauses that resolved to `key`.
    fn precondition_span(&self, key: &DeclarationKey) -> Option<Span> {
        self.unit_clauses_for(key)
            .into_iter()
            .filter(|clause| clause.kind == StateClauseKind::Precondition)
            .map(|clause| clause.spans.declaration)
            .reduce(|left, right| Span {
                start: left.start.min(right.start),
                end: left.end.max(right.end),
            })
    }

    /// FR-302-AC-8: a body function from `first` on whose `operation-body`
    /// declares a different number of parameters than its operation refuses
    /// `ill_typed`/`type-mismatch`, located at the body.
    fn body_arity_refusals(&self, first: usize) -> Vec<CheckRefusal> {
        let mut refusals = Vec::new();
        for (index, function) in self.functions.iter().enumerate().skip(first) {
            let Some(clause) = self
                .model_clauses
                .get(&index)
                .filter(|clause| clause.kind == DeclaredClauseKind::Body)
            else {
                continue;
            };
            let Some(body) = self.operation_body(&clause.declaration) else {
                continue;
            };
            if body.parameters.len() != body.operation.declaration.parameters().len() {
                refusals.push(CheckRefusal::ill_typed(
                    &Location::root(Origin::Body {
                        function: function.name.clone(),
                        index,
                    }),
                    IllTypedCause::TypeMismatch,
                ));
            }
        }
        refusals
    }
}

/// The signature a call to a family's root names: what every call site
/// through the family's table is checked against.
struct CallSite {
    /// The root's declaring type, which a call through it names.
    owner: EffectiveId,
    member: String,
    /// The arguments after the receiver.
    parameters: Vec<ValueType>,
    result: ValueType,
}

impl CallSite {
    /// The call site of `root` from the data the bridge was given, or `None`
    /// when the bridge would have refused for lack of it.
    fn of(root: &DeclarationKey, clauses: &OperationClauses) -> Option<Self> {
        let signature = clauses.parameters.get(root)?;
        let (_, ValueType::Reference(owner)) = signature.first()? else {
            return None;
        };
        Some(Self {
            owner: *owner,
            member: clauses.member.get(root)?.clone(),
            parameters: signature
                .iter()
                .skip(1)
                .map(|(_, value_type)| value_type.clone())
                .collect(),
            result: clauses.result.get(root)?.clone(),
        })
    }
}

/// `expression` with `self` spelled as the receiver parameter the bridge's
/// functions bind it as; no other node changes.
fn receiver_as_parameter(expression: &Expression) -> Expression {
    expression
        .respelled(|_, node| {
            if matches!(node, ExprNode::SelfRef) {
                *node = ExprNode::Name("self".to_owned());
            }
        })
        // `respelled` refuses only a rewrite that changes the tree's shape;
        // this one changes a leaf's payload alone.
        .unwrap_or_else(|_| expression.clone())
}

/// The cause for a family the bridge refused.
fn link_cause(operation: DeclarationKey, refusal: &DispatchBridgeRefusal) -> DispatchLinkCause {
    match refusal {
        DispatchBridgeRefusal::Unlinked(outcome) => match outcome.as_ref() {
            LinkCheckOutcome::Completed(DispatchLinkOutcome::Ambiguous(failed)) => {
                let subtypes: Vec<DispatchSubtypeFailure> = failed
                    .iter()
                    .map(|failure| DispatchSubtypeFailure {
                        subtype: failure.subtype.clone(),
                        cause: failure.cause.catalog_code().cause(),
                        candidates: failure.candidates.clone(),
                        dominance_pairs: failure
                            .dominance_pairs
                            .iter()
                            .map(|pair| (pair.dominant.clone(), pair.dominated.clone()))
                            .collect(),
                    })
                    .collect();
                DispatchLinkCause {
                    operation,
                    code: Code::AmbiguousDispatch,
                    catalog: CatalogCode::new(
                        "ambiguous_dispatch",
                        subtypes
                            .first()
                            .map_or("no-applicable", |failure| failure.cause),
                    ),
                    limit: None,
                    subtypes,
                }
            }
            LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(_)) => invariant(operation),
            LinkCheckOutcome::Refused(model) => model_cause(operation, model),
            LinkCheckOutcome::Incomplete(incomplete) => {
                let limit = incomplete.limit_exceeded();
                DispatchLinkCause {
                    operation,
                    code: Code::StageLimitExceeded,
                    catalog: CatalogCode::new("stage_limit_exceeded", limit.kind().catalog_cause()),
                    limit: Some(DispatchLinkLimit {
                        setting: limit.setting(),
                        bound: limit.configured_bound(),
                        reached: limit.actual(),
                    }),
                    subtypes: Vec::new(),
                }
            }
            LinkCheckOutcome::OpenClosure(open) => DispatchLinkCause {
                operation,
                code: open.code,
                catalog: open.cause.catalog_code(),
                limit: None,
                subtypes: Vec::new(),
            },
        },
        DispatchBridgeRefusal::FamilyStepsExceeded(model)
        | DispatchBridgeRefusal::NotAQuery(model)
        | DispatchBridgeRefusal::UnknownCandidate(model) => model_cause(operation, model),
        DispatchBridgeRefusal::MissingClauseData { .. }
        | DispatchBridgeRefusal::MissingObjectKey { .. }
        | DispatchBridgeRefusal::DuplicateObjectKey { .. } => invariant(operation),
    }
}

fn model_cause(operation: DeclarationKey, model: &ModelRefusal) -> DispatchLinkCause {
    DispatchLinkCause {
        operation,
        code: model.code,
        catalog: model.catalog_code(),
        limit: model.cause.limit_exceeded().map(|limit| DispatchLinkLimit {
            setting: limit.setting(),
            bound: limit.configured_bound(),
            reached: limit.actual(),
        }),
        subtypes: Vec::new(),
    }
}

/// The checker's own input to the bridge was incomplete: a broken invariant
/// of the assembler, never a property of the source.
fn invariant(operation: DeclarationKey) -> DispatchLinkCause {
    DispatchLinkCause {
        operation,
        code: Code::RuntimeInvariant,
        catalog: CatalogCode::new("runtime_invariant", "established-invariant-broken"),
        limit: None,
        subtypes: Vec::new(),
    }
}
