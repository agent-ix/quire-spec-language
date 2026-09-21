// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §7.3 M-5 (QSL-139/FR-068): the layer-3 checking half of what was
//! `value::expression`. This module owns name resolution, typing, static
//! definedness and termination checking, and the checked-output types
//! ([`CheckedPackage`], [`CheckedExpression`], `CheckedFunction`) whose
//! constructors are private here (ADR-011 §4's private-constructor/
//! public-accessor mechanism). The evaluation half -- `evaluate.rs`,
//! `CheckedPackage::call`, `CheckedPackage::evaluate` and the
//! argument-admission logic -- stays at layer 5 in
//! `crate::value::expression` (a private module, not resolvable as an
//! intra-doc link) (S6a), reaching this module's state only
//! through the accessor methods below, never through a private field: the
//! two modules no longer share private state (US-009).
//!
//! # The interim `model` -> `check` edge (FR-068-AC-9, FR-068-CON-5)
//!
//! `model::checked_dispatch.rs` and `model::conformance.rs` import thirteen
//! names this module now defines
//! (`DispatchCandidate`/`DispatchOperation`/`DispatchTable`/
//! `PackageDeclarations` and
//! `established_field_fact`/`Connective`/`Established`/`Location`/`Node`/
//! `NodeKind`/`OrderedKind`/`Origin`/`ProvedInterval`), directly from
//! `crate::check`, not through `crate::value`'s aggregate re-export. Before
//! this move that dependency sat inside `value::expression`, a module not
//! yet cleanly assigned to one ADR-011 §6.1 layer; after this move it is a
//! legible `model` (layer-3-earlier) -> `check` (layer-3-later) reverse
//! edge, forbidden by §6.1's intra-layer-3 order until M-2 (QSL-7) moves
//! `model::checked_dispatch` and
//! `model::conformance::check_field_refinement_obligation` into `check`
//! itself. This requirement declares the edge rather than hiding it: see
//! FR-068's Behavior section, "The interim `model` -> `check` edge."
//!
//! # `family.rs`
//!
//! **Amended, PR #282 review F4:** FR-068's move surface now names
//! `family.rs` explicitly, alongside `check`, `evaluate`, `facts`, `ir`,
//! `mod` itself, `refusal` and `termination` -- an omission in the
//! requirement's original text, not a deliberate exclusion: `check.rs`'s
//! own `Typer::call` already called `family::mint_call_identity`/
//! `family::DEFAULT_PACKAGE_IDENTITY` directly before this move, so this
//! module carries a `family` submodule with exactly the identity-minting
//! content those existing calls need (see `family`'s own module doc).
//! Without it, `check`'s real import graph would reach back into
//! `value::expression::family`, which FR-068-AC-3 forbids.

// FR-068 itself names both the destination module (`check`) and the moved
// file (`check.rs`, holding `Typer`/`Scope`/`bind_parameters` -- name
// resolution and typing) -- the inception is the spec's own naming, not an
// accidental collision this file introduced.
#[allow(clippy::module_inception)]
mod check;
mod facts;
mod family;
mod ir;
mod refusal;
mod termination;

use check::{bind_parameters, Signature, Typer};
use facts::{CallSite, Definedness};

use crate::family::FamilyContract;
use crate::forms::{ClauseKind, Expression, FunctionDeclaration};
use crate::value::composite::ValueType;

// `crate::model::conformance`'s FR-151 refinement obligation reuses this
// crate's own FR-146 fact-derivation primitive rather than a second
// implementation (see `established_field_fact`'s own doc); exposed
// crate-internal-only, the same `pub(crate) use` pattern
// `crate::value::mod`'s own `length_amount`/`Charge` re-export already uses
// for `model`<->`value` reuse. `model/conformance.rs` imports these
// directly from here (FR-068-CON-5), not through `crate::value`.
pub(crate) use check::Scope;
pub(crate) use facts::{established_field_fact, Established};
pub(crate) use family::{ValueFunctionFamily, SCALAR_LIMITS_UNLIMITED};
// `mint_declaration_identity`, `OccurrenceMap` and `DEFAULT_PACKAGE_IDENTITY`
// are consumed only by `value::expression::family`'s `#[cfg(test)]` modules
// (layer 5 depending on layer 3 is permitted), so this re-export is itself
// `#[cfg(test)]`-gated rather than plain: a plain `pub(crate) use` here is
// genuinely unused in a non-test build (`cargo check`/`cargo build`/`cargo
// clippy` without `--all-targets`), and `-D warnings` promotes that to a
// hard compile error before cargo ever reaches the test binaries where it
// would be used -- gating on `cfg(test)` keeps both builds clean instead of
// papering over the non-test one with `#[allow(unused_imports)]`.
#[cfg(test)]
pub(crate) use family::{mint_declaration_identity, OccurrenceMap, DEFAULT_PACKAGE_IDENTITY};
pub(crate) use ir::{Arithmetic, Connective, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit};

pub use check::{
    CheckingLimits, DepthAboveMaximum, DispatchOperation, EnumBinding, PackageDeclarations,
    MAX_CHECKING_DEPTH,
};
pub use ir::{CollectionLoss, CollectionProperty, DispatchCandidate, DispatchTable};
pub use refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, DispatchFunctionRole,
    InvalidDispatchDeclaration, Location, MeasureObligation, Obligation, Origin, ProvedInterval,
    WrongSnapshotCause,
};

/// How a standalone expression is checked.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckMode {
    /// Linking: typing and every static definedness obligation.
    Linked,
    /// Direct kernel evaluation over supplied values: typing only, so an
    /// empty `reduce` or `value(none)` is a located undefined outcome.
    Kernel,
}

/// A checked function.
#[derive(Debug)]
struct CheckedFunction {
    /// FR-062/FR-065: content-addressed identity, minted once at check from
    /// the declaration's own parsed structure (`family::
    /// mint_declaration_identity`), independent of this function's position
    /// in the package's function list.
    identity: quire_exact::NodeKey,
    signature: Signature,
    body: Node,
    measure: Option<Node>,
    slots: usize,
}

/// A package whose every function is admitted. Its constructor and every
/// field are private to this module (ADR-011 §4): `value::expression`
/// reaches this state only through the accessor methods below.
#[derive(Debug)]
pub struct CheckedPackage {
    scope: Scope,
    functions: Vec<CheckedFunction>,
    dispatch_tables: Vec<DispatchTable>,
    /// FR-062-AC-2/FR-065-AC-3: the occurrence-keyed source map (identity,
    /// role, ordinal) -> source [`Location`], for every function
    /// declaration and function-application occurrence this package
    /// checked.
    occurrences: family::OccurrenceMap<Location>,
}

/// A checked standalone expression over named parameters. Its constructor
/// and every field are private to this module (ADR-011 §4).
#[derive(Debug)]
pub struct CheckedExpression {
    parameters: Vec<(String, ValueType)>,
    root: Node,
    slots: usize,
}

impl CheckedExpression {
    /// The result type.
    pub fn value_type(&self) -> &ValueType {
        &self.root.value_type
    }

    /// Every `convert` loss in pre-order.
    pub fn losses(&self) -> Vec<CollectionLoss> {
        self.root.losses()
    }

    /// Every `deref(r).f` location, whose target existence is a runtime input
    /// requirement.
    pub fn dereferences(&self) -> Vec<Location> {
        self.root.dereferences()
    }

    /// The declared parameters, in evaluation-slot order -- the accessor
    /// [`crate::value::expression::CheckedPackage::evaluate`] reads to
    /// validate its caller's arguments (ADR-011 §4: evaluation never reads
    /// this type's fields directly). Crate-internal only: `Node` (see
    /// [`Self::root`]) is `pub(crate)`, so this whole accessor surface stays
    /// no more public than that.
    pub(crate) fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked expression tree evaluation runs.
    pub(crate) fn root(&self) -> &Node {
        &self.root
    }

    /// The evaluation slot count evaluation allocates.
    pub(crate) fn slots(&self) -> usize {
        self.slots
    }
}

/// One admitted function's evaluation-visible state: exactly what
/// `value::expression`'s evaluator needs (name, checked body, slot count --
/// FR-068's own Description names these as the accessor surface), without
/// exposing `CheckedFunction`'s private representation. Crate-internal only:
/// `body`'s `Node` type is `pub(crate)`.
pub(crate) struct FunctionState<'a> {
    /// The declared name.
    pub(crate) name: &'a str,
    /// The checked body.
    pub(crate) body: &'a Node,
    /// The evaluation slot count.
    pub(crate) slots: usize,
}

/// One callable function's evaluation-visible identity and signature --
/// what [`crate::value::expression::CheckedPackage::call`] needs to route a
/// runtime `QualifiedName` lookup through
/// [`crate::family::ReferenceEvaluation::evaluate`] and validate its
/// caller's arguments, without a direct field read.
pub(crate) struct CallableFunction<'a> {
    /// The minted identity [`crate::family::ReferenceEvaluation::evaluate`]
    /// resolves against.
    pub(crate) identity: quire_exact::NodeKey,
    /// The declared parameters, for argument admission.
    pub(crate) parameters: &'a [(String, ValueType)],
}

fn root(origin: Origin) -> Location {
    Location {
        origin,
        path: Vec::new(),
    }
}

fn invalid_dispatch(location: Location, detail: InvalidDispatchDeclaration) -> CheckRefusal {
    CheckRefusal {
        location,
        cause: CheckCause::InvalidDispatchDeclaration(detail),
    }
}

/// One dispatch-table function index against the dispatch operation it must
/// conform to: `expected_arity` is the receiver plus every declared
/// argument, `result` is `Boolean` for a precondition (clause or combinator)
/// and the operation's own declared result for a body -- validates every
/// dispatch table/candidate index and signature arity/type upfront, rather
/// than trusting a caller-supplied table at evaluation time. A function that
/// exists (valid `index`) is located at its own real
/// [`Origin::Body`]; an out-of-range `index` names no real declaration to
/// point at, so it is located at [`Origin::Expression`] instead.
fn validate_dispatch_function(
    functions: &[FunctionDeclaration],
    index: usize,
    call_parameters: &[ValueType],
    result: &ValueType,
    role: DispatchFunctionRole,
) -> Result<(), CheckRefusal> {
    let Some(function) = functions.get(index) else {
        return Err(invalid_dispatch(
            root(Origin::Expression),
            InvalidDispatchDeclaration::FunctionOutOfRange { role, index },
        ));
    };
    let location = root(Origin::Body {
        function: function.name.clone(),
        index,
    });
    let expected_arity = call_parameters.len() + 1;
    if function.parameters.len() != expected_arity {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::Arity {
                role,
                index,
                declared: function.parameters.len(),
                expected: expected_arity,
            },
        ));
    }
    let mismatched = function
        .parameters
        .iter()
        .skip(1)
        .map(|(_, value_type)| value_type)
        .zip(call_parameters)
        .any(|(declared, expected)| declared != expected);
    if mismatched {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::ParameterType { role, index },
        ));
    }
    if &function.result != result {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::ResultType { role, index },
        ));
    }
    Ok(())
}

impl PackageDeclarations {
    /// Check every function: duplicate names, declared types, typing, static
    /// definedness and termination, in that order. Every refusal is made
    /// before any charge; a reached checking limit is `resource_exhausted`
    /// and yields no admission verdict.
    pub fn check(self, limits: CheckingLimits) -> Result<CheckedPackage, Vec<CheckRefusal>> {
        let body_location = |index: usize, name: &str| {
            root(Origin::Body {
                function: name.to_owned(),
                index,
            })
        };
        let mut refusals = Vec::new();
        for (index, function) in self.functions.iter().enumerate() {
            let loci: Vec<Location> = self
                .functions
                .iter()
                .enumerate()
                .filter(|(_, other)| other.name == function.name)
                .map(|(other, declaration)| body_location(other, &declaration.name))
                .collect();
            if loci.len() > 1 {
                refusals.push(CheckRefusal {
                    location: body_location(index, &function.name),
                    cause: CheckCause::AmbiguousName {
                        name: function.name.clone(),
                        loci,
                    },
                });
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let mut seen_operations = std::collections::BTreeSet::new();
        for operation in &self.dispatch_operations {
            if !seen_operations.insert((operation.receiver_type, operation.member.clone())) {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::DuplicateOperation {
                        receiver_type: operation.receiver_type,
                        member: operation.member.clone(),
                    },
                ));
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        for operation in &self.dispatch_operations {
            let Some(table) = self.dispatch_tables.get(operation.table) else {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::TableOutOfRange {
                        member: operation.member.clone(),
                        table: operation.table,
                    },
                ));
                continue;
            };
            for (_, candidate) in table.entries() {
                if let Err(refusal) = validate_dispatch_function(
                    &self.functions,
                    candidate.body,
                    &operation.parameters,
                    &operation.result,
                    DispatchFunctionRole::Body,
                ) {
                    refusals.push(refusal);
                }
                if let Some(precondition) = candidate.precondition {
                    if let Err(refusal) = validate_dispatch_function(
                        &self.functions,
                        precondition,
                        &operation.parameters,
                        &ValueType::Boolean,
                        DispatchFunctionRole::Precondition,
                    ) {
                        refusals.push(refusal);
                    }
                }
                for &clause in &candidate.precondition_clauses {
                    if let Err(refusal) = validate_dispatch_function(
                        &self.functions,
                        clause,
                        &operation.parameters,
                        &ValueType::Boolean,
                        DispatchFunctionRole::PreconditionClause,
                    ) {
                        refusals.push(refusal);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let scope = Scope {
            types: self.types,
            enums: self.enums,
            aliases: self.aliases,
            model_operations: self.model_operations,
            ieee_profile: self.ieee_profile,
            dispatch_operations: self.dispatch_operations,
        };
        let dispatch_tables = self.dispatch_tables;
        let signatures: Vec<Signature> = self
            .functions
            .iter()
            .map(|function| Signature {
                name: function.name.clone(),
                parameters: function.parameters.clone(),
                result: function.result.clone(),
                callable_by_name: function.callable_by_name,
            })
            .collect();
        let mut nodes = 0_u64;
        let mut functions = Vec::with_capacity(self.functions.len());
        // FR-062/FR-065, corrected per PR #262 review (the headline
        // question): identity is minted, and one diagnostic logged, through
        // the checked-family contract's own `check` hook
        // (`family::ValueFunctionFamily::check`) -- that part is real and
        // exclusive to the contract. It is NOT what decides whether this
        // declaration is admitted: `check` only ever refuses on the
        // nesting-depth limit below, and admits unconditionally otherwise.
        // The typing, definedness and termination verdict is still made
        // entirely by the unchanged `Typer`, invoked immediately below, for
        // every function, unconditionally -- FR-065's own claim that this
        // migration makes the form "check... exclusively through" the
        // contract is accurate only for identity/provenance minting, not
        // for the checking decision itself.
        let package_identity = family::DEFAULT_PACKAGE_IDENTITY.to_owned();
        // PR #262 review, finding F4: this used to hardcode
        // `MAX_CHECKING_DEPTH` here regardless of what `limits` (this
        // method's own caller-supplied `CheckingLimits`) declared, and a
        // fresh `CheckContext` is built inside the per-declaration loop
        // below (`depth` starts at 0 every time) -- so `enter_nesting`'s
        // `0 >= nesting_depth` never held for any real caller, and the
        // `StageFailure::Limit` arm below was dead through this, the only
        // production entry point. Reading `limits.depth()` here (the same
        // `CheckingLimits` the unchanged `Typer` below already honors)
        // makes the contract's own resource bound live.
        let contract_limits = crate::family::StageLimits {
            nesting_depth: limits.depth(),
        };
        let mut contract_meter = quire_exact::Meter::new(family::SCALAR_LIMITS_UNLIMITED);
        let mut contract_diagnostics = crate::family::DiagnosticSink::default();
        let mut contract_scopes = crate::family::ScopeStack::default();
        for (index, function) in self.functions.into_iter().enumerate() {
            let location = body_location(index, &function.name);
            let mut contract_cx = crate::family::CheckContext::new(
                &package_identity,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            let identity = match family::ValueFunctionFamily::check(&function, &mut contract_cx) {
                Ok(staged) => staged.value,
                Err(crate::family::StageFailure::Limit(limit)) => {
                    // PR #262 review (coordinator round 3, finding 4):
                    // `limit.kind` is matched, not read past into a
                    // hardcoded `CheckingLimitKind::Depth` -- `StageLimitKind`
                    // has exactly one variant today, but this exhaustive
                    // match (not a `_` catch-all) is what forces a real
                    // decision here, not a guess, the day a second
                    // `StageLimitKind` variant is added.
                    let kind = match limit.kind {
                        crate::family::StageLimitKind::NestingDepth => CheckingLimitKind::Depth,
                    };
                    refusals.push(CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::ResourceExhausted {
                            stage: CheckingStage::Typing,
                            kind,
                            limit: limit.configured_bound,
                        },
                    });
                    continue;
                }
            };
            let typed = (|| {
                let mut typer = Typer::new(
                    &scope,
                    &signatures,
                    limits,
                    &mut nodes,
                    function.clause_kind,
                );
                bind_parameters(&mut typer, &function.parameters, &location)?;
                typer.check_declared_type(&function.result, &location)?;
                let body = typer.check_as(&function.body, &function.result, &location)?;
                let slots = typer.slots();
                let measure = match &function.measure {
                    Some(measure) => {
                        let at = root(Origin::Measure {
                            function: function.name.clone(),
                            index,
                        });
                        // A `decreases` measure is always checked as
                        // `ClauseKind::Body` (`syntax.rs`'s own doc: "A
                        // function body, an operation body, or a `decreases`
                        // measure"), never `function.clause_kind`: FR-151's
                        // dispatch-call restriction gates on the *body's*
                        // context, and a measure is its own, always-Body
                        // context regardless of what the body itself is
                        // checked as.
                        let mut typer =
                            Typer::new(&scope, &signatures, limits, &mut nodes, ClauseKind::Body);
                        bind_parameters(&mut typer, &function.parameters, &at)?;
                        Some(typer.infer(measure, None, &at)?)
                    }
                    None => None,
                };
                Ok::<_, CheckRefusal>((body, measure, slots))
            })();
            match typed {
                Ok((body, measure, slots)) => functions.push(CheckedFunction {
                    identity,
                    signature: Signature {
                        name: function.name,
                        parameters: function.parameters,
                        result: function.result,
                        callable_by_name: function.callable_by_name,
                    },
                    body,
                    measure,
                    slots,
                }),
                Err(refusal) => {
                    let exhausted = matches!(refusal.cause, CheckCause::ResourceExhausted { .. });
                    refusals.push(refusal);
                    if exhausted {
                        return Err(refusals);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let mut calls: Vec<Vec<CallSite>> = Vec::with_capacity(functions.len());
        for function in &functions {
            let parameters = function.signature.parameters.len();
            let mut body =
                Definedness::new(parameters, &dispatch_tables, &scope.dispatch_operations);
            let checked = body
                .check(&function.body)
                .and_then(|()| match &function.measure {
                    Some(measure) => {
                        Definedness::new(parameters, &dispatch_tables, &scope.dispatch_operations)
                            .check(measure)
                    }
                    None => Ok(()),
                });
            if let Err(refusal) = checked {
                refusals.push(refusal);
            }
            calls.push(body.calls);
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let members: Vec<termination::Member<'_>> = functions
            .iter()
            .zip(&calls)
            .map(|(function, calls)| termination::Member {
                name: &function.signature.name,
                parameters: &function.signature.parameters,
                measure: function.measure.as_ref(),
                calls,
            })
            .collect();
        let refusals = termination::check(&members);
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // FR-062-AC-2/FR-065-AC-3: the occurrence-keyed source map, built
        // from each function's own minted declaration identity and every
        // `NodeKind::Call` identity its checked body (and measure, if any)
        // already carries -- reads locations `Typer` already recorded, mints
        // no new identity or span here.
        let mut occurrences = family::OccurrenceMap::default();
        for (index, function) in functions.iter().enumerate() {
            occurrences.record(
                function.identity,
                "declaration",
                body_location(index, &function.signature.name),
            );
            for (identity, location) in function.body.call_occurrences() {
                occurrences.record(identity, "reference", location);
            }
            if let Some(measure) = &function.measure {
                for (identity, location) in measure.call_occurrences() {
                    occurrences.record(identity, "reference", location);
                }
            }
        }
        Ok(CheckedPackage {
            scope,
            functions,
            dispatch_tables,
            occurrences,
        })
    }
}

impl CheckedPackage {
    /// Check a standalone expression over `parameters`, against `expected`
    /// when given, as a function or operation body (`ClauseKind::Body`).
    /// `pre(...)` refuses `wrong_snapshot`/`wrong-anchor` here: this is not
    /// an operation's postcondition, the only clause FR-153's anchor table
    /// admits it in. Use [`Self::check_postcondition_expression`] to check a
    /// real postcondition, where `pre(...)` is legal, or
    /// [`Self::check_clause_expression`] for any other [`ClauseKind`].
    pub fn check_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        self.check_clause_expression(
            parameters,
            expression,
            expected,
            ClauseKind::Body,
            mode,
            limits,
        )
    }

    /// Check a standalone expression as an operation's postcondition
    /// (`ClauseKind::Postcondition`): identical to [`Self::check_expression`],
    /// except `pre(...)` is legal (FR-153's own anchor table;
    /// shared-grammar.md's caller-side anchor operations), subject to its
    /// own eligible-operand rule (FR-042's Behavior clause, `Typer`'s
    /// `Expression::Pre` arm).
    pub fn check_postcondition_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        self.check_clause_expression(
            parameters,
            expression,
            expected,
            ClauseKind::Postcondition,
            mode,
            limits,
        )
    }

    /// Check a standalone expression over `parameters` as `clause_kind`,
    /// against `expected` when given. FR-151's dispatch-call restriction
    /// (TC-196 D06/D07) gates on `clause_kind`, not on syntax alone; `pre(...)`
    /// is legal exactly when `clause_kind` is [`ClauseKind::Postcondition`]
    /// (`Typer`'s `Expression::Pre` arm).
    pub fn check_clause_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        clause_kind: ClauseKind,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        let location = root(Origin::Expression);
        let signatures: Vec<Signature> = self
            .functions
            .iter()
            .map(|function| function.signature.clone())
            .collect();
        let mut nodes = 0_u64;
        let mut typer = Typer::new(&self.scope, &signatures, limits, &mut nodes, clause_kind);
        bind_parameters(&mut typer, &parameters, &location)?;
        let root = match expected {
            Some(expected) => {
                typer.check_declared_type(expected, &location)?;
                typer.check_as(expression, expected, &location)?
            }
            None => typer.infer(expression, None, &location)?,
        };
        let slots = typer.slots();
        if mode == CheckMode::Linked {
            Definedness::new(
                parameters.len(),
                &self.dispatch_tables,
                &self.scope.dispatch_operations,
            )
            .check(&root)?;
        }
        Ok(CheckedExpression {
            parameters,
            root,
            slots,
        })
    }

    fn function(&self, name: &str) -> Option<(usize, &CheckedFunction)> {
        self.functions
            .iter()
            .enumerate()
            .find(|(_, function)| function.signature.name == name)
    }

    fn function_by_identity_raw(&self, identity: quire_exact::NodeKey) -> Option<&CheckedFunction> {
        self.functions
            .iter()
            .find(|function| function.identity == identity)
    }

    /// The `deref(r).f` locations of a function body, or `None` for an
    /// undeclared name.
    pub fn dereferences(&self, function: &str) -> Option<Vec<Location>> {
        self.function(function)
            .map(|(_, function)| function.body.dereferences())
    }

    /// The `convert` losses of a function body, or `None` for an undeclared
    /// name.
    pub fn losses(&self, function: &str) -> Option<Vec<CollectionLoss>> {
        self.function(function)
            .map(|(_, function)| function.body.losses())
    }

    /// FR-065-AC-2: `name`'s checked identity, minted once at `check` and
    /// unchanged by anything else in the package. `None` for an undeclared
    /// name.
    pub fn function_identity(&self, name: &str) -> Option<quire_exact::NodeKey> {
        self.function(name).map(|(_, function)| function.identity)
    }

    /// FR-062-AC-2/FR-065-AC-3: the source location recorded for one
    /// occurrence (identity, role, ordinal) of a checked function
    /// declaration or function-application call, if `check` recorded one.
    pub fn occurrence(
        &self,
        identity: quire_exact::NodeKey,
        origin: &quire_exact::Origin,
    ) -> Option<&Location> {
        self.occurrences.resolve(identity, origin)
    }

    /// Every admitted function's own name, checked body and evaluation slot
    /// count -- the accessor surface
    /// [`crate::value::expression::CheckedPackage::evaluate`] reads to build
    /// its own `Callable` list, since `Callable` is a layer-5 type this
    /// module must not construct itself (that would be a `check` ->
    /// `value::expression` edge, forbidden by FR-068-AC-3).
    pub(crate) fn function_states(&self) -> impl Iterator<Item = FunctionState<'_>> + '_ {
        self.functions.iter().map(|function| FunctionState {
            name: &function.signature.name,
            body: &function.body,
            slots: function.slots,
        })
    }

    /// One admitted function's evaluation-visible state, by its minted
    /// identity -- the accessor
    /// [`crate::family::ReferenceEvaluation::evaluate`]'s `Value` family
    /// implementation (`value::expression::family`) resolves a checked call
    /// against.
    pub(crate) fn function_by_identity(
        &self,
        identity: quire_exact::NodeKey,
    ) -> Option<FunctionState<'_>> {
        self.function_by_identity_raw(identity)
            .map(|function| FunctionState {
                name: &function.signature.name,
                body: &function.body,
                slots: function.slots,
            })
    }

    /// `name`'s identity and declared parameters, filtered to functions a
    /// plain named call may resolve to (`callable_by_name`) -- the accessor
    /// [`crate::value::expression::CheckedPackage::call`] uses to resolve a
    /// runtime `QualifiedName` lookup and validate its caller's arguments,
    /// without a direct field read (TC-196 D07's bypass: a crate-internal
    /// FR-151 synthesized dispatch candidate is never reachable this way).
    pub(crate) fn callable(&self, name: &str) -> Option<CallableFunction<'_>> {
        self.function(name)
            .filter(|(_, function)| function.signature.callable_by_name)
            .map(|(_, function)| CallableFunction {
                identity: function.identity,
                parameters: &function.signature.parameters,
            })
    }

    /// Every admitted function's own name and minted identity -- the
    /// accessor
    /// [`crate::value::expression::CheckedPackage::emit_function_package_v2`]
    /// reads to build its v2 entries, since the v2 codec and `QualifiedName`
    /// are `value::expression::family` types this module must not import
    /// (FR-068-AC-3).
    pub(crate) fn function_identities(
        &self,
    ) -> impl Iterator<Item = (&str, quire_exact::NodeKey)> + '_ {
        self.functions
            .iter()
            .map(|function| (function.signature.name.as_str(), function.identity))
    }

    /// The scope every declared name resolves against -- the accessor
    /// [`crate::value::expression::CheckedPackage::evaluate`] passes through
    /// to the evaluator.
    pub(crate) fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The checked dispatch tables `scope`'s dispatch operations index into
    /// -- the accessor the evaluator reads directly, alongside `scope`.
    pub(crate) fn dispatch_tables(&self) -> &[DispatchTable] {
        &self.dispatch_tables
    }
}
