// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-401: the std reference adapter. It implements `quire-eval`'s
//! [`CheckedClosure`] contract over [`CheckedPackage`] and
//! [`ObjectEnvironment`], so [`CheckedPackageEvaluation::call`] and
//! [`CheckedPackageEvaluation::evaluate`] drive one `quire-eval` session to
//! terminal, with the results and input refusals they had before.
//!
//! # Ownership
//!
//! [`PackageClosure`] is generic over how it holds the package and the
//! checked expression, through [`Deref`]: the synchronous entry points pass
//! plain references, and a caller that needs a registry to outlive the
//! package borrow passes an [`std::sync::Arc`] (a [`CheckedPackage`] is
//! `Arc`-shared by its dependency closure already). The suspended frame
//! holds no borrow of the package: it holds the admitted arguments and the
//! callable's identity, and re-reads the package when it runs.
//!
//! # Granularity
//!
//! The reference evaluator keeps its calls on its own explicit task stack
//! (`Machine`), and that stack borrows the checked graph, so it cannot be
//! suspended between two of its own steps without a self-referential
//! frame. One interpreted frame therefore runs to its terminal evaluation in
//! a single transition, and never returns a child request. A host body
//! registered for a callable is reached from the root application and from
//! a host body's child request. FR-401 requires an interpreted body's own
//! call to such a callable to dispatch to the host body too; the reference
//! evaluator does not yet, and that gap is carried by a code ticket. The
//! steppable `no_std` evaluator (IR-590) closes it.

use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::ops::Deref;

use qsl_semantics::check::CheckedExpression;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::supplied::{SuppliedLimits, SuppliedRun};
use quire_eval::{CheckedClosure, Evaluation, RequestRefusal, Signature, Transition};
use quire_exact::{Meter, NodeKey, Value, ValueType};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;

use super::evaluate::Machine;
use super::family::EvaluationEnv;
use super::s6a::S6aFamilyKind;
use super::{admit_arguments, evaluate_declaration, EvaluationTarget};
use qsl_package::CheckedPackage;
use quire_eval::CallFailure;

/// The reference [`CheckedClosure`] over one checked package and its
/// dependency closure.
///
/// The admitted callables are the functions of the package's checked graph
/// and of its dependency closure that are callable by name, each identified
/// by its minted [`NodeKey`], with the parameter and result types the
/// package records. An FR-151 synthesized dispatch candidate is not callable
/// by name and is not admitted.
///
/// `P` is how the package is held and `X` how a checked expression is
/// held, each through [`Deref`] (see the module documentation); `X` is a
/// cheap handle (a reference or an `Arc`), cloned into the frame.
pub struct PackageClosure<P, X> {
    package: P,
    /// Whether the caller already admitted its arguments (under its own
    /// supplied-value run), so `Begin` admits nothing more.
    pre_admitted: bool,
    expression: PhantomData<fn() -> X>,
}

impl<P, X> PackageClosure<P, X>
where
    P: Deref<Target = CheckedPackage>,
{
    /// A closure whose `Begin` admits arguments itself, under the default
    /// supplied-value limits.
    pub const fn new(package: P) -> Self {
        Self {
            package,
            pre_admitted: false,
            expression: PhantomData,
        }
    }

    /// A closure for a caller that admitted its arguments already, under its
    /// own supplied-value run (`call_supplied`): `Begin` admits nothing more,
    /// so no argument is admitted twice.
    pub(super) const fn pre_admitted(package: P) -> Self {
        Self {
            package,
            pre_admitted: true,
            expression: PhantomData,
        }
    }

    /// The package that declares `identity` as a function callable by name:
    /// this one, or the first of its dependency closure that does.
    fn declaring(&self, identity: NodeKey) -> Option<&CheckedPackage> {
        let mut pending = vec![&*self.package];
        let mut seen: BTreeSet<*const CheckedPackage> = BTreeSet::new();
        while let Some(package) = pending.pop() {
            if !seen.insert(std::ptr::from_ref(package)) {
                continue;
            }
            if package.graph().callable_by_identity(identity).is_some() {
                return Some(package);
            }
            pending.extend(package.dependencies().values().map(|d| &**d));
        }
        None
    }

    fn admit(
        &self,
        parameters: &[(String, ValueType)],
        arguments: &[Value],
        environment: &ObjectEnvironment,
        meter: &Meter,
    ) -> Result<(), CallFailure> {
        // The meter's installed handle is the session's: a handle cancelled
        // before `Begin` ends it as `Cancelled`, before any charge.
        let cancel = meter.cancel().cloned().unwrap_or_default();
        let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel).map_err(|stop| {
            CallFailure::from(quire_semantic_value::supplied_stop::SuppliedFailure::Stop(
                stop,
            ))
        })?;
        admit_arguments(
            parameters,
            arguments,
            environment,
            self.package.graph().scope().types(),
            &mut run,
        )
    }
}

/// The saved state of the reference adapter's one interpreted frame.
pub struct PackageTask<X>(Frame<X>);

enum Frame<X> {
    /// An admitted application of a callable.
    Call {
        callable: NodeKey,
        arguments: Vec<Value>,
    },
    /// An admitted checked expression.
    Expression {
        expression: X,
        parameters: Vec<Value>,
    },
    /// The frame ran; nothing remains to resume.
    Finished,
}

impl<P, X> CheckedClosure for PackageClosure<P, X>
where
    P: Deref<Target = CheckedPackage>,
    X: Deref<Target = CheckedExpression> + Clone + Send,
{
    type Expression = X;
    type Environment = ObjectEnvironment;
    type Task = PackageTask<X>;

    fn signature(&self, callable: &NodeKey) -> Option<Signature> {
        let package = self.declaring(*callable)?;
        let function = package.graph().callable_by_identity(*callable)?;
        Some(Signature::new(
            function
                .parameters
                .iter()
                .map(|(_, value_type)| value_type.clone())
                .collect::<Vec<_>>(),
            function.result.clone(),
        ))
    }

    fn admit_call(
        &self,
        callable: &NodeKey,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        meter: &Meter,
    ) -> Result<(), CallFailure> {
        let package = self
            .declaring(*callable)
            .ok_or_else(|| InputRefusal::UnknownFunction(identity_spelling(callable)))?;
        if self.pre_admitted {
            return Ok(());
        }
        let function = package
            .graph()
            .callable_by_identity(*callable)
            .ok_or_else(|| InputRefusal::UnknownFunction(identity_spelling(callable)))?;
        self.admit(function.parameters, arguments, environment, meter)
    }

    fn admit_request(
        &self,
        callable: &NodeKey,
        arguments: &[Value],
        environment: &ObjectEnvironment,
    ) -> Result<(), RequestRefusal> {
        let function = self
            .declaring(*callable)
            .and_then(|package| package.graph().callable_by_identity(*callable))
            .ok_or(RequestRefusal::NotAdmitted)?;
        if function.parameters.len() != arguments.len() {
            return Err(RequestRefusal::NotAdmitted);
        }
        let types = self
            .declaring(*callable)
            .ok_or(RequestRefusal::NotAdmitted)?
            .graph()
            .scope()
            .types();
        for (parameter, ((_, value_type), argument)) in
            function.parameters.iter().zip(arguments).enumerate()
        {
            let member = match (value_type, argument) {
                // `Population<T>[N]` is admitted by its resolved binding's
                // declared maximum (FR-089-AC-5), as `admit_arguments` does.
                (ValueType::Population(maximum), Value::Population(id)) => environment
                    .resolve_population(*id)
                    .is_some_and(|binding| binding.declared_maximum() == *maximum),
                _ => types
                    .admits(value_type, argument)
                    .map_err(|failure| RequestRefusal::Storage { parameter, failure })?,
            };
            if !member {
                return Err(RequestRefusal::NotAdmitted);
            }
        }
        Ok(())
    }

    fn admit_expression(
        &self,
        expression: &X,
        parameters: &[Value],
        environment: &ObjectEnvironment,
        meter: &Meter,
    ) -> Result<(), CallFailure> {
        if self.pre_admitted {
            return Ok(());
        }
        self.admit(expression.parameters(), parameters, environment, meter)
    }

    fn bind_call(
        &self,
        callable: &NodeKey,
        arguments: Vec<Value>,
        _environment: &ObjectEnvironment,
    ) -> Result<Self::Task, InternalFault> {
        Ok(PackageTask(Frame::Call {
            callable: *callable,
            arguments,
        }))
    }

    fn bind_expression(
        &self,
        expression: &X,
        parameters: Vec<Value>,
        _environment: &ObjectEnvironment,
    ) -> Result<Self::Task, InternalFault> {
        Ok(PackageTask(Frame::Expression {
            expression: expression.clone(),
            parameters,
        }))
    }

    fn advance(
        &self,
        task: &mut Self::Task,
        delivered: Option<Evaluation>,
        environment: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Transition, InternalFault> {
        if delivered.is_some() {
            return Err(InternalFault::new(
                "S6a",
                "reference-adapter-frame-never-requests-a-child",
            ));
        }
        match std::mem::replace(&mut task.0, Frame::Finished) {
            Frame::Call {
                callable,
                arguments,
            } => {
                let package = self.declaring(callable).ok_or_else(|| {
                    InternalFault::new("S6a", "checked-identity-not-resolved-by-package")
                })?;
                // The scheduler charged this application's `function.call`.
                let mut env = EvaluationEnv::charged(package, environment, arguments);
                evaluate_declaration(
                    S6aFamilyKind::Value,
                    &callable,
                    EvaluationTarget::Value(&mut env),
                    meter,
                )
                .map(Transition::Done)
            }
            Frame::Expression {
                expression,
                parameters,
            } => {
                let graph = self.package.graph();
                Machine::new(
                    graph.scope(),
                    graph,
                    environment,
                    meter,
                    graph.dispatch_tables(),
                )
                .run(expression.root(), expression.slots(), parameters)
                .map(Transition::Done)
            }
            Frame::Finished => Err(InternalFault::new(
                "S6a",
                "reference-adapter-frame-resumed-after-it-ran",
            )),
        }
    }
}

/// The hex spelling of a callable identity, for the refusal that names a
/// callable no checked closure declares.
fn identity_spelling(identity: &NodeKey) -> String {
    identity
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
