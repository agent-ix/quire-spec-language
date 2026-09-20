// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 value expressions and total pure functions (FR-145, FR-146).
//!
//! [`PackageDeclarations::check`] resolves names, types every body and
//! measure, checks every definedness obligation on a reachable path and the
//! `decreases` obligations of every recursive component, all before any
//! charge. [`CheckedPackage::call`] and [`CheckedPackage::evaluate`] then run
//! checked code under a [`Meter`](super::Meter).

mod check;
mod evaluate;
mod facts;
mod family;
mod ir;
mod refusal;
mod termination;

use super::accounting::Meter;
use super::composite::{Value, ValueType};
use super::reference::ObjectEnvironment;
use crate::family::{FamilyContract, ReferenceEvaluation};
use crate::forms::{ClauseKind, Expression, FunctionDeclaration};
use check::{bind_parameters, Scope, Signature, Typer};
use evaluate::{Callable, Machine};
use facts::{CallSite, Definedness};

// `crate::model::conformance`'s FR-151 refinement obligation reuses this
// crate's own FR-146 fact-derivation primitive rather than a second
// implementation (see `established_field_fact`'s own doc); exposed
// crate-internal-only, the same `pub(crate) use` pattern
// `crate::value::mod`'s own `length_amount`/`Charge` re-export already uses
// for `model`<->`value` reuse.
pub(crate) use facts::{established_field_fact, Established};
pub(crate) use ir::{Connective, Node, NodeKind, OrderedKind};

pub use check::{
    CheckingLimits, DepthAboveMaximum, DispatchOperation, EnumBinding, PackageDeclarations,
    MAX_CHECKING_DEPTH,
};
pub use evaluate::{Evaluation, LocatedLoss, ValueLoss};
pub use family::{DecodeV2Error, InvalidQualifiedName, QualifiedName};
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

/// A package whose every function is admitted.
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

/// A checked standalone expression over named parameters.
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
}

/// A runtime input a call or evaluation refuses before any charge.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InputRefusal {
    /// No function of this name: `missing_declaration` / `missing-name`.
    #[error("no function named {0}")]
    UnknownFunction(String),
    /// The argument count differs from the parameter count:
    /// `invalid_runtime_input` / `wrong-value-kind`.
    #[error("{supplied} arguments for {declared} parameters")]
    Arity {
        /// Declared parameters.
        declared: usize,
        /// Supplied arguments.
        supplied: usize,
    },
    /// An argument is not a value of its parameter type:
    /// `invalid_runtime_input` / `wrong-value-kind`.
    #[error("argument {parameter} is not a value of its declared type")]
    WrongValueKind {
        /// The parameter index.
        parameter: usize,
    },
    /// An argument holds a reference with no object in the complete
    /// population: `dangling_reference` / `absent-target-in-complete-population`.
    #[error("argument {parameter} holds a reference with no target object")]
    DanglingReference {
        /// The parameter index.
        parameter: usize,
    },
}

impl InputRefusal {
    /// The refusal code.
    pub fn code(&self) -> crate::diagnostic::Code {
        use crate::diagnostic::Code;
        match self {
            Self::UnknownFunction(_) => Code::MissingDeclaration,
            Self::Arity { .. } | Self::WrongValueKind { .. } => Code::InvalidRuntimeInput,
            Self::DanglingReference { .. } => Code::DanglingReference,
        }
    }

    /// The closed cause tag.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownFunction(_) => "missing-name",
            Self::Arity { .. } | Self::WrongValueKind { .. } => "wrong-value-kind",
            Self::DanglingReference { .. } => "absent-target-in-complete-population",
        }
    }
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
/// and the operation's own declared result for a body — validates every
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
        // for the checking decision itself. Both the contract's `check` and
        // the `Typer` run for every declaration; see FR-065's own amended
        // Status section for why this is recorded as a real limitation
        // rather than resolved by this round's fixes, and why it is not the
        // ADR-011 §7.3 M-6e side-by-side hazard in the narrow sense that
        // rule targets (there is no separate, deprecated *old* admission
        // path for declarations this ticket left running by oversight --
        // the `Typer` is not "old" in that sense, it is the only checker
        // that has ever existed for this form, and the contract's `check`
        // was never built to replace it).
        let package_identity = family::DEFAULT_PACKAGE_IDENTITY.to_owned();
        let contract_limits = crate::family::StageLimits {
            nesting_depth: MAX_CHECKING_DEPTH,
        };
        let mut contract_meter = quire_exact::Meter::new(quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
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
        // PR #262 review (coordinator round 3): an earlier version of this
        // function asserted `contract_diagnostics.entries().len() ==
        // contract_checked` here. `contract_checked` is incremented exactly
        // once per successful `ValueFunctionFamily::check` call above, and
        // `check` itself records exactly one diagnostic on every successful
        // path (its own doc) -- the two counts are equal by construction,
        // not because anything downstream was checked. `contract_checked`
        // is deleted along with it; FR-062-AC-3's real coverage is the
        // `#[cfg(test)]` assertions in `value/expression/family.rs` that
        // exercise `check` directly and inspect its diagnostic sink.
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
        // PR #262 review (coordinator round 3): an earlier version of this
        // function asserted `occurrences.entries().len() >=
        // functions.len()` here. The loop above unconditionally calls
        // `occurrences.record(function.identity, "declaration", ...)` once
        // per function in `functions`, before any "reference" entry --
        // `entries().len()` is at least `functions.len()` by construction,
        // not because anything downstream was checked. `OccurrenceMap::
        // entries`, that assertion's only reader anywhere in this crate, is
        // deleted with it.
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
    ///
    /// `self`/`result`, shared-grammar.md's other two caller-side anchor
    /// operations (line 500/604, alongside `pre(...)`), are out of scope
    /// here: this method takes no declared operation (no result type, no
    /// receiver type) to bind either one to, [`Expression`] itself
    /// (`syntax.rs`) has no `Self_`/`Result` variant to even lower a
    /// reference to either into, and nothing in this crate's own value
    /// layer defines a checked "operation" declaration with a result-type
    /// binding contract (`crate::model::domain_package`'s `PostconditionClause` is
    /// a different, model-layer structure, never lowered through this
    /// `Expression`/`Typer`/`Node` pipeline). Binding `result` needs that
    /// missing declaration shape first; this method is deliberately silent
    /// on it rather than guessing one.
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

    fn function(&self, name: &str) -> Option<(usize, &CheckedFunction)> {
        self.functions
            .iter()
            .enumerate()
            .find(|(_, function)| function.signature.name == name)
    }

    /// The checked function whose minted identity is `identity`, if this
    /// package admitted one (the identity-keyed lookup [`Self::call`]'s
    /// `QualifiedName`-keyed lookup cannot serve, and evaluation-by-identity
    /// needs).
    fn function_by_identity(&self, identity: quire_exact::NodeKey) -> Option<&CheckedFunction> {
        self.functions
            .iter()
            .find(|function| function.identity == identity)
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

    /// FR-062/FR-065: this package's `quire.checked-function-package/v2`
    /// bytes -- the checked-package producer's own public entry point.
    /// S4-links each identity first (`family::link_function_identity`),
    /// then emits directly through `family::emit_v2` (PR #262 review,
    /// findings F1/F2: an earlier version routed this through
    /// `FamilyContract::package` into a scratch buffer that was never read
    /// back, which that trait method has since been deleted for -- see its
    /// own doc). Reads no CST, no source text, only each function's
    /// already-checked identity.
    pub fn emit_function_package_v2(&self) -> Vec<u8> {
        let entries = self
            .functions
            .iter()
            .map(|function| {
                let linked = family::link_function_identity(function.identity);
                (function.signature.name.clone(), linked)
            })
            .collect::<Vec<_>>();
        family::emit_v2(&entries)
    }

    /// Decode `quire.checked-function-package/v2` bytes emitted by
    /// [`Self::emit_function_package_v2`] back into (qualified name,
    /// identity) pairs, for a caller verifying identity survived the round
    /// trip (FR-065-AC-2).
    pub fn decode_function_package_v2(
        bytes: &[u8],
    ) -> Result<Vec<(String, quire_exact::NodeKey)>, family::DecodeV2Error> {
        family::decode_v2(bytes)
    }

    fn callables(&self) -> Vec<Callable<'_>> {
        self.functions
            .iter()
            .map(|function| Callable {
                body: &function.body,
                slots: function.slots,
                name: &function.signature.name,
            })
            .collect()
    }

    fn validate(
        parameters: &[(String, ValueType)],
        arguments: &[Value],
        objects: &ObjectEnvironment,
    ) -> Result<(), InputRefusal> {
        if parameters.len() != arguments.len() {
            return Err(InputRefusal::Arity {
                declared: parameters.len(),
                supplied: arguments.len(),
            });
        }
        for (parameter, ((_, value_type), argument)) in parameters.iter().zip(arguments).enumerate()
        {
            if !value_type.admits(argument) {
                return Err(InputRefusal::WrongValueKind { parameter });
            }
            let mut pending = vec![argument];
            while let Some(value) = pending.pop() {
                match value {
                    Value::Reference(reference) => {
                        if !objects.contains(reference) {
                            return Err(InputRefusal::DanglingReference { parameter });
                        }
                    }
                    Value::Option(option) => pending.extend(option.payload()),
                    Value::Composite(composite) => {
                        pending.extend(composite.slots().iter().filter_map(|slot| match slot {
                            super::composite::FieldValue::Present(value) => Some(value),
                            super::composite::FieldValue::Absent
                            | super::composite::FieldValue::Null => None,
                        }));
                    }
                    Value::Collection(collection) => pending.extend(collection.elements()),
                    Value::Boolean(_)
                    | Value::Integer(_)
                    | Value::Rational(_)
                    | Value::Decimal(_)
                    | Value::Float(_)
                    | Value::Quantity(_)
                    | Value::Text(_)
                    | Value::Enum(_)
                    | Value::Population(_) => {}
                }
            }
        }
        Ok(())
    }

    /// Call the named function: `function.call`, then its body. Refused
    /// `InputRefusal::UnknownFunction` for a name [`Self::function`] finds
    /// but whose `callable_by_name` is `false` — the same refusal an
    /// undeclared name gets, not a distinct one — so this public runtime
    /// entry point cannot reach a crate-internal FR-151 synthesized dispatch
    /// candidate body or effective precondition by name any more than an
    /// ordinary checked `Expression::Call` can (`check.rs`'s own
    /// `callable_by_name` gate, TC-196 D07's bypass this closes at the other
    /// entry point).
    ///
    /// FR-065-AC-6/ADR-013 O-11: `function` is a typed [`QualifiedName`],
    /// never a bare `&str` — this is the layer-6 `replay` facade's executor
    /// entry for `Value`'s function family (the `mod.rs:635` bare-`&str`
    /// lookup this requirement replaces). A name this package's
    /// declarations do not resolve refuses with `UnknownFunction`, naming
    /// it; it never falls back to a display-name string comparison.
    pub fn call(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InputRefusal> {
        let name = function
            .as_unqualified()
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        let (_, checked) = self
            .function(name)
            .filter(|(_, checked)| checked.signature.callable_by_name)
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        Self::validate(&checked.signature.parameters, &arguments, objects)?;
        // FR-062/FR-065: this family's own `evaluate` hook
        // (`crate::family::ReferenceEvaluation`) is the one path that runs
        // checked function-application code, not a second, parallel
        // `Machine` call beside it.
        let identity = checked.identity;
        let mut contract_meter = quire_exact::Meter::new(quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
        let mut env = family::EvaluationEnv {
            package: self,
            objects,
            arguments: Some(arguments),
            local_meter: meter,
        };
        family::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter).map_err(
            |refusal| match refusal {
                crate::family::EvaluateRefusal::Refused(reason) => {
                    InputRefusal::UnknownFunction(reason)
                }
            },
        )
    }

    /// Evaluate a checked expression with `arguments` for its parameters.
    pub fn evaluate(
        &self,
        expression: &CheckedExpression,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InputRefusal> {
        Self::validate(&expression.parameters, &arguments, objects)?;
        let callables = self.callables();
        Ok(Machine::new(
            &self.scope,
            &callables,
            objects,
            meter,
            &self.dispatch_tables,
        )
        .run(&expression.root, expression.slots, arguments, false))
    }
}
