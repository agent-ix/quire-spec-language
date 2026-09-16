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
use ir::Node;

pub use check::{
    CheckingLimits, DepthAboveMaximum, EnumBinding, PackageDeclarations, MAX_CHECKING_DEPTH,
};
pub use evaluate::Evaluation;
pub use ir::{CollectionLoss, CollectionProperty};
pub use refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, Location, MeasureObligation,
    Obligation, Origin, ProvedInterval, UnsupportedForm,
};
pub use syntax::{
    Accumulation, BinaryOperator, BinderQuery, Expression, FieldInitializer, FunctionDeclaration,
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
        let scope = Scope {
            types: self.types,
            enums: self.enums,
            aliases: self.aliases,
            model_operations: self.model_operations,
            ieee_profile: self.ieee_profile,
        };
        let signatures: Vec<Signature> = self
            .functions
            .iter()
            .map(|function| Signature {
                name: function.name.clone(),
                parameters: function.parameters.clone(),
                result: function.result.clone(),
            })
            .collect();
        let mut nodes = 0_u64;
        let mut functions = Vec::with_capacity(self.functions.len());
        for (index, function) in self.functions.into_iter().enumerate() {
            let location = body_location(index, &function.name);
            let typed = (|| {
                let mut typer = Typer::new(&scope, &signatures, limits, &mut nodes);
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
                        let mut typer = Typer::new(&scope, &signatures, limits, &mut nodes);
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
            let mut body = Definedness::new(parameters);
            let checked = body
                .check(&function.body)
                .and_then(|()| match &function.measure {
                    Some(measure) => Definedness::new(parameters).check(measure),
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
        Ok(CheckedPackage { scope, functions })
    }
}

impl CheckedPackage {
    /// Check a standalone expression over `parameters`, against `expected`
    /// when given.
    pub fn check_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
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
        let mut typer = Typer::new(&self.scope, &signatures, limits, &mut nodes);
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
            Definedness::new(parameters.len()).check(&root)?;
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
                    | Value::Enum(_) => {}
                }
            }
        }
        Ok(())
    }

    /// Call the named function: `function.call`, then its body.
    pub fn call(
        &self,
        function: &str,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InputRefusal> {
        let (_, checked) = self
            .function(function)
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_owned()))?;
        Self::validate(&checked.signature.parameters, &arguments, objects)?;
        let callables = self.callables();
        Ok(Machine::new(&self.scope, &callables, objects, meter).run(
            &checked.body,
            checked.slots,
            arguments,
            true,
        ))
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
        Ok(Machine::new(&self.scope, &callables, objects, meter).run(
            &expression.root,
            expression.slots,
            arguments,
            false,
        ))
    }
}
