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
mod ir;
mod refusal;
mod syntax;
mod termination;

use super::accounting::Meter;
use super::composite::{Value, ValueType};
use super::reference::ObjectEnvironment;
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
pub use ir::{CollectionLoss, CollectionProperty, DispatchCandidate, DispatchTable};
pub use refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, DispatchFunctionRole,
    InvalidDispatchDeclaration, Location, MeasureObligation, Obligation, Origin, ProvedInterval,
    WrongSnapshotCause,
};
pub use syntax::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, Expression, FieldInitializer,
    FunctionDeclaration,
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
        for (index, function) in self.functions.into_iter().enumerate() {
            let location = body_location(index, &function.name);
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
                        // checked as (finding #172-6).
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
        Ok(CheckedPackage {
            scope,
            functions,
            dispatch_tables,
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
    /// binding contract (`crate::model::bundle`'s `PostconditionClause` is
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
    pub fn call(
        &self,
        function: &str,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InputRefusal> {
        let (_, checked) = self
            .function(function)
            .filter(|(_, checked)| checked.signature.callable_by_name)
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_owned()))?;
        Self::validate(&checked.signature.parameters, &arguments, objects)?;
        let callables = self.callables();
        Ok(Machine::new(
            &self.scope,
            &callables,
            objects,
            meter,
            &self.dispatch_tables,
        )
        .run(&checked.body, checked.slots, arguments, true))
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
