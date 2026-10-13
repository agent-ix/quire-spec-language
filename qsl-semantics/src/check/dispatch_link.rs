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
use super::refusal::{DispatchLinkCause, DispatchSubtypeFailure};
use super::{CheckCause, CheckRefusal, PackageDeclarations};
use crate::model::accounting::{Meter, ModelNormalizationLimits};
use crate::model::dispatch::{DispatchLinkOutcome, GeneralizationClosure, LinkCheckOutcome};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::intake::member_identity_name;
use crate::model::key::DeclarationKey;
use crate::model::normalize::{EffectiveView, ModelRefusal};

/// One family a called member name may name: the alias of the selected model
/// that declares it, the key of its root operation (an operation that
/// redefines none) and the member name.
struct CalledFamily {
    alias: String,
    root: DeclarationKey,
    member: String,
}

/// One family that did not link: the member name it answers to, the object
/// types a call to it may have as receiver, and its refusal.
#[derive(Debug)]
struct PendingFamily {
    member: String,
    receivers: BTreeSet<EffectiveId>,
    refusal: CheckRefusal,
}

/// The families that did not link, held until the unit is checked: a family
/// matched by member name alone may be one no call reaches, and that must not
/// refuse the unit (FR-302).
#[derive(Debug, Default)]
pub struct PendingLinks(Vec<PendingFamily>);

impl PendingLinks {
    /// The refusals of a unit whose check refused with `refusals`. A call
    /// that found no dispatch operation, and whose receiver's static type is
    /// one a failed family answers for under that member, is refused with
    /// that family's refusal, at the call; the family is reported once, at
    /// its first call. Every other refusal is kept as it is, and a failed
    /// family no call needed is dropped.
    pub fn report(self, refusals: Vec<CheckRefusal>) -> Vec<CheckRefusal> {
        let mut reported = BTreeSet::new();
        let mut kept = Vec::with_capacity(refusals.len());
        for refusal in refusals {
            let Some((member, receiver)) = unresolved_call(&refusal) else {
                kept.push(refusal);
                continue;
            };
            let family = self
                .0
                .iter()
                .position(|family| family.member == member && family.receivers.contains(&receiver));
            match family {
                None => kept.push(refusal),
                Some(index) if reported.insert(index) => {
                    let mut family = self.0[index].refusal.clone();
                    family.location = refusal.location;
                    kept.push(family);
                }
                Some(_) => {}
            }
        }
        kept
    }
}

/// The member and receiver type of a call the check found no dispatch
/// operation for.
fn unresolved_call(refusal: &CheckRefusal) -> Option<(&str, EffectiveId)> {
    match &refusal.cause {
        CheckCause::StateModel(model) => match (&model.cause, model.receiver) {
            (CheckCause::MissingName(name), Some(receiver)) => Some((name, receiver)),
            _ => None,
        },
        _ => None,
    }
}

impl PackageDeclarations {
    /// FR-302: links the dispatch table of each family a dispatched call's
    /// member name may name, under `limits`' `family_steps` ceiling and the
    /// other model normalization counters, one meter for the whole unit, and
    /// merges the linked tables into the package.
    ///
    /// A family that does not link is returned as [`PendingLinks`] rather
    /// than refused, because the member name alone does not say that a call
    /// reaches it; the caller reports it through [`PendingLinks::report`] if
    /// the check refuses a call for want of it. A body whose parameter count
    /// differs from its operation's refuses `ill_typed`/`type-mismatch`
    /// located at the body, when linking first reads it, and a failure to
    /// build or merge what the bridge needs refuses as a broken invariant.
    pub fn link_dispatch(
        &mut self,
        limits: ModelNormalizationLimits,
    ) -> Result<PendingLinks, Vec<CheckRefusal>> {
        let called = self.called_members();
        if called.is_empty() {
            return Ok(PendingLinks::default());
        }
        let mut meter = Meter::new(limits);
        let mut pending = PendingLinks::default();
        let mut linked = Vec::new();
        let mut clauses: BTreeMap<String, Option<OperationClauses>> = BTreeMap::new();
        for family in self.called_families(&called) {
            let Some(view) = self.selected_views.get(&family.alias) else {
                continue;
            };
            let clauses = clauses
                .entry(family.alias.clone())
                .or_insert_with(|| self.view_clauses(view));
            let cause = match clauses {
                None => Err(invariant(family.root.clone())),
                Some(clauses) => {
                    let root = DispatchRoot {
                        key: family.root.clone(),
                        closure: GeneralizationClosure::Closed,
                    };
                    checked_dispatch_operation(
                        view,
                        &root,
                        clauses,
                        self.source.clone(),
                        &mut meter,
                        self.identity,
                    )
                    .map(|declarations| (declarations, CallSite::of(&family.root, clauses)))
                    .map_err(|refusal| link_cause(family.root.clone(), &refusal))
                }
            };
            match cause {
                Ok((declarations, call)) => linked.push((family.alias, declarations, call)),
                Err(cause) => {
                    let receivers = clauses
                        .as_ref()
                        .and_then(|clauses| CallSite::of(&family.root, clauses))
                        .map(|call| self.receiver_types(view, call.owner))
                        .unwrap_or_default();
                    pending.0.push(PendingFamily {
                        member: family.member,
                        receivers: receivers.into_iter().collect(),
                        refusal: dispatch_link(cause),
                    });
                }
            }
        }
        let first_new = self.functions.len();
        let mut refusals = Vec::new();
        for (alias, declarations, call) in linked {
            if let Err(cause) = self.merge_linked(&alias, declarations, call) {
                refusals.push(dispatch_link(*cause));
            }
        }
        refusals.extend(self.body_arity_refusals(first_new));
        if refusals.is_empty() {
            Ok(pending)
        } else {
            Err(refusals)
        }
    }

    /// The member names of the dispatched calls in the unit's state clauses
    /// and in the kept operation clauses: the only places a dispatched call
    /// checks (FR-151, TC-196 D07). A call in an operation body refuses
    /// `ill_typed`/`operator-ineligible` and links nothing.
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
            let mut roots: Vec<(&DeclarationKey, &str)> = view
                .domain_package()
                .records
                .iter()
                .filter_map(|record| match record {
                    DomainPackageRecord::OperationMember(operation)
                        if operation.redefines.is_none() =>
                    {
                        member_identity_name(&operation.owner.node, &operation.key.node)
                            .filter(|name| called.contains(*name))
                            .map(|name| (&operation.key, name))
                    }
                    _ => None,
                })
                .collect();
            roots.sort();
            families.extend(roots.into_iter().map(|(root, member)| CalledFamily {
                alias: alias.clone(),
                root: root.clone(),
                member: member.to_owned(),
            }));
        }
        families
    }

    /// What the bridge reads of the operations of `view`: each one's
    /// signature from the operation table, body from the unit's
    /// `operation-body` declaration and precondition from the unit's `pre`
    /// clauses, keyed by its `DeclarationKey`. The bridge reads only the
    /// entries of the family it walks, under `family_steps`. `None` when a
    /// retained expression cannot be rewritten.
    fn view_clauses(&self, view: &EffectiveView) -> Option<OperationClauses> {
        let mut clauses = OperationClauses::default();
        for record in &view.domain_package().records {
            let DomainPackageRecord::OperationMember(operation) = record else {
                continue;
            };
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
            clauses.member.insert(key.clone(), name.to_owned());
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
                    rename_names(&receiver_as_parameter(&body.body)?, &rename),
                );
            }
            let mut precondition: Option<Expression> = None;
            for clause in self
                .unit_clauses_for(key)
                .into_iter()
                .filter(|clause| clause.kind == StateClauseKind::Precondition)
            {
                let term = receiver_as_parameter(&clause.body)?;
                precondition = Some(match precondition {
                    None => term,
                    Some(before) => Expression::binary(BinaryOperator::And, before, term),
                });
            }
            if let Some(precondition) = precondition {
                clauses.own_precondition.insert(key.clone(), precondition);
            }
            clauses.parameters.insert(key.clone(), signature);
        }
        Some(clauses)
    }

    /// Appends what the bridge built for one family of the model `alias`: its
    /// functions after the package's own (a source function keeps its index),
    /// its call sites and its table, with every function index and table
    /// index the bridge numbered from zero moved to its place in the
    /// package. A body function carries the spans of the `operation-body` it
    /// came from, so a refusal at the body resolves to its source region
    /// (FR-096). A call through any object type that conforms to the root's
    /// declaring type, abstract or not, names the family's table, not only a
    /// type whose own member is the root.
    fn merge_linked(
        &mut self,
        alias: &str,
        linked: PackageDeclarations,
        call: Option<CallSite>,
    ) -> Result<(), Box<DispatchLinkCause>> {
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
            let body = clause
                .filter(|clause| clause.kind == DeclaredClauseKind::Body)
                .and_then(|clause| {
                    self.operation_body(&clause.declaration)
                        .map(|body| (clause, body))
                });
            let function = match body {
                Some((clause, body)) => function
                    .with_spans(body.spans.clone())
                    .map_err(|_| Box::new(invariant(clause.declaration.clone())))?,
                None => function,
            };
            // A precondition function the bridge synthesized from the unit's
            // `pre` clauses (one clause, their conjunction, or the
            // disjunction with a redefined operation's) has no tree of spans
            // to match its shape: it is located at the first of those
            // clauses.
            if let Some(span) = clause
                .filter(|clause| clause.kind == DeclaredClauseKind::Precondition)
                .and_then(|clause| self.precondition_span(&clause.declaration))
            {
                self.synthesized_spans.insert(index, span);
            }
            self.functions.push(function);
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
        if let (Some(call), Some(view)) = (&call, self.selected_views.get(alias)) {
            let receivers = self.receiver_types(view, call.owner);
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
        for table in dispatch_tables {
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
        Ok(())
    }

    /// Every object type of `view` that conforms to `owner`, the declaring
    /// type of a family's root, abstract or not: the static types of the
    /// receivers whose calls name the family.
    fn receiver_types(&self, view: &EffectiveView, owner: EffectiveId) -> Vec<EffectiveId> {
        view.type_identities()
            .values()
            .copied()
            .filter(|candidate| {
                self.types.object_type(*candidate).is_some()
                    && self.types.conforms(*candidate, owner)
            })
            .collect()
    }

    /// The span of the first unit `pre` clause that resolved to `key`.
    fn precondition_span(&self, key: &DeclarationKey) -> Option<Span> {
        self.unit_clauses_for(key)
            .into_iter()
            .find(|clause| clause.kind == StateClauseKind::Precondition)
            .map(|clause| clause.spans.declaration)
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

fn dispatch_link(cause: DispatchLinkCause) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::DispatchLink(Box::new(cause)),
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
/// functions bind it as; no other node changes. `None` if the rewrite
/// changed the tree's shape, which it cannot: it changes a leaf's payload.
fn receiver_as_parameter(expression: &Expression) -> Option<Expression> {
    expression
        .respelled(|_, node| {
            if matches!(node, ExprNode::SelfRef) {
                *node = ExprNode::Name("self".to_owned());
            }
        })
        .ok()
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
                    denial: None,
                    subtypes,
                }
            }
            LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(_)) => invariant(operation),
            LinkCheckOutcome::Refused(model) => model_cause(operation, model),
            LinkCheckOutcome::Incomplete(incomplete) => DispatchLinkCause {
                operation,
                code: Code::ResourceExhausted,
                catalog: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                denial: Some(incomplete.denial()),
                subtypes: Vec::new(),
            },
            LinkCheckOutcome::OpenClosure(open) => DispatchLinkCause {
                operation,
                code: open.code,
                catalog: open.cause.catalog_code(),
                denial: None,
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
        denial: model.cause.denial(),
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
        denial: None,
        subtypes: Vec::new(),
    }
}
