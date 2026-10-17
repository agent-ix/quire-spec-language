// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-367: [`bind_scalar_operands`], the join of one admitted [`Call`]'s
//! named values to the operands of one scalar application occurrence.
//!
//! A driver that runs a generated scalar oracle needs the occurrence's
//! operands as an ordered `&[i64]` and the argument that fed each. The call
//! is admitted as [`super::run`] admits it (FR-100); the occurrence is
//! checked as FR-357's operator entry checks a claim's scalar node; the
//! operand structure comes from Contract IR's `scalar_application_operands`
//! and `parameter_declaration`. Nothing here evaluates the function.

use qsl_foundation::diagnostic::{Code, InternalFault};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::{checked_node_id, checked_occurrence, CheckedPackage};
use qsl_semantics::check::NodeTag;
use qsl_semantics::library::PackageId;
use qsl_semantics::model::key::hex;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_contract_model::{
    read_checked_package, CheckedNodeId, CheckedPackageDispatchResult, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedParameterDeclarationError, CheckedScalarOperand,
    CheckedScalarOperandChild, CheckedScalarOperandError, CheckedScalarOperandRange,
};
use quire_exact::{Cancel, ChargePoint, Incomplete, Integer, LimitKind, Value, ValueType};
use quire_semantic_value::location::Origin as BodyOrigin;

use super::call::{admit_model_arguments, bind_arguments, convert_call_failure, select};
use super::{AdmittedModels, Call, CallIncomplete, CheckedUnit, EmittedUnit, RunRefusal};
use crate::execute::callable_parameter_keys;
use crate::execute::scalar_site::{applies, in_body};
use crate::scalar::ScalarOperator;

/// The scalar claim's location (FR-357 Inputs, operator level).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarOccurrence {
    /// The enclosing function's declared name; it must equal
    /// [`Call::function`].
    pub function: String,
    /// The scalar application node.
    pub application: WireNodeId,
    /// The occurrence key of that node.
    pub occurrence: OccurrenceKey,
}

/// The compiled and emitted package a call runs against, as [`super::run`]
/// holds it after its compile and package stages, with Contract IR's reading
/// of the emitted bytes.
#[derive(Debug)]
pub struct BindPackage<'a> {
    models: &'a AdmittedModels,
    checked: &'a CheckedUnit,
    package_id: PackageId,
    ir: Box<CheckedPackageV2>,
}

impl<'a> BindPackage<'a> {
    /// Read `emitted` back through Contract IR. The bytes were just written
    /// by the emitter, so a refusal is an internal failure.
    pub fn read(
        models: &'a AdmittedModels,
        checked: &'a CheckedUnit,
        emitted: &'a EmittedUnit,
    ) -> Result<Self, Box<RunRefusal>> {
        let mut evidence = emitted.evidence().clone();
        for (digest, document) in models.packages() {
            evidence
                .insert_domain_package_document(hex(digest), document.as_slice())
                .map_err(|_| fault("domain-package-documents-do-not-conflict"))?;
        }
        let CheckedPackageDispatchResult::AdmittedV2(ir) = read_checked_package(
            emitted.package().bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        ) else {
            return Err(fault("emitted-package-is-readable-by-contract-ir"));
        };
        Ok(Self {
            models,
            checked,
            package_id: emitted.package().package_id(),
            ir,
        })
    }

    /// The emitted package's `package_id`.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }
}

/// What [`bind_scalar_operands`] settled.
#[derive(Debug)]
pub enum BindOutcome {
    /// Every operand bound.
    Bound(BoundScalarOperands),
    /// The occurrence or an operand refused; no operand list.
    Refused(BindRefusal),
    /// The argument conversion reached an accounting limit; no binding.
    Incomplete(CallIncomplete),
}

/// The operands of one scalar application occurrence, bound to a call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundScalarOperands {
    package_id: PackageId,
    occurrence: ScalarOccurrence,
    operands: Vec<BoundOperand>,
}

impl BoundScalarOperands {
    /// The package's `package_id`.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// The occurrence as supplied.
    pub fn occurrence(&self) -> &ScalarOccurrence {
        &self.occurrence
    }

    /// One operand per application argument, in the application's wire
    /// argument order.
    pub fn operands(&self) -> &[BoundOperand] {
        &self.operands
    }

    /// Each operand's value in operand order: the generated oracle's
    /// argument order.
    pub fn values(&self) -> Vec<i64> {
        self.operands.iter().map(|operand| operand.value).collect()
    }
}

/// One bound operand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundOperand {
    /// Zero-based position in the application's arguments.
    pub ordinal: u64,
    /// The operand's concrete value.
    pub value: i64,
    /// The operand's inclusive range as Contract IR returns it.
    pub range: CheckedScalarOperandRange,
    /// What supplied the value.
    pub source: OperandSource,
}

/// What supplied a bound operand's value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperandSource {
    /// A parameter of the called function, supplied by a [`Call`] argument.
    Argument {
        /// The index of the supplying argument in [`Call::arguments`].
        argument: usize,
        /// The parameter's declared name.
        parameter: String,
        /// The parameter node the operand references.
        node: WireNodeId,
        /// The parameter node's declared level, as decimal text.
        level: String,
    },
    /// An integer literal; no argument fed the operand.
    InlineLiteral {
        /// The `value`/`literal` graph node the operand references, or
        /// `None` for an inline literal term of the application.
        node: Option<WireNodeId>,
    },
}

/// Why the occurrence is not the claim's scalar application (FR-357's
/// causes, and the function name this requirement adds).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindIdentityCause {
    /// [`ScalarOccurrence::function`] differs from [`Call::function`].
    Function {
        /// The occurrence's function name.
        claimed: String,
        /// The call's function name.
        called: String,
    },
    /// The application is no node of the package, or lies outside the
    /// called function's body.
    Node {
        /// The claimed application node.
        node: WireNodeId,
    },
    /// The node is no application of `add`, `sub`, `mul` or `negate`.
    Operator {
        /// The claimed application node.
        node: WireNodeId,
    },
    /// The occurrence is not one of the node's occurrences in the called
    /// function's body.
    Occurrence {
        /// The claimed occurrence.
        occurrence: OccurrenceKey,
    },
}

/// Why an occurrence or an operand did not bind.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BindRefusal {
    /// The occurrence is not the claim's scalar application.
    #[error("the occurrence does not name a scalar application of the called function: {0:?}")]
    ScalarIdentity(BindIdentityCause),
    /// Contract IR cannot read the application's operands.
    #[error("the application's operands cannot be read: {0}")]
    Operands(CheckedScalarOperandError),
    /// Operand `ordinal` is neither a literal nor a parameter.
    #[error("operand {ordinal} is neither a literal nor a parameter")]
    OperandNotParameter {
        /// The operand's position.
        ordinal: u64,
    },
    /// Operand `ordinal` references a parameter node that is no parameter of
    /// the called function, such as a `let` or quantifier binder.
    #[error(
        "operand {ordinal} references {parameter} at level {level}, which no argument supplies"
    )]
    OperandNotArgument {
        /// The operand's position.
        ordinal: u64,
        /// The parameter node's declared name.
        parameter: String,
        /// The parameter node's declared level, as decimal text.
        level: String,
    },
    /// Operand `ordinal`'s value is not representable as `i64`.
    #[error("operand {ordinal} has the value {value}, which is not an i64")]
    OperandNotNative {
        /// The operand's position.
        ordinal: u64,
        /// The value's decimal text.
        value: String,
    },
}

impl BindRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ScalarIdentity(_) => Code::StaleDependency,
            Self::Operands(_)
            | Self::OperandNotParameter { .. }
            | Self::OperandNotArgument { .. } => Code::UnsupportedConstruct,
            Self::OperandNotNative { .. } => Code::InvalidRuntimeInput,
        }
    }

    /// The catalog cause, where the code has one here.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::ScalarIdentity(_) => Some("content-mismatch"),
            Self::Operands(_)
            | Self::OperandNotParameter { .. }
            | Self::OperandNotArgument { .. } => Some("expression-form"),
            Self::OperandNotNative { .. } => None,
        }
    }
}

/// Everything that stops a binding short of `Bound`.
#[derive(Debug)]
enum Halt {
    Run(Box<RunRefusal>),
    Refused(BindRefusal),
    Incomplete(Box<CallIncomplete>),
}

impl From<Box<RunRefusal>> for Halt {
    fn from(refusal: Box<RunRefusal>) -> Self {
        Self::Run(refusal)
    }
}

impl From<BindRefusal> for Halt {
    fn from(refusal: BindRefusal) -> Self {
        Self::Refused(refusal)
    }
}

fn fault(invariant: &'static str) -> Box<RunRefusal> {
    Box::new(RunRefusal::Fault(InternalFault::new("call", invariant)))
}

/// Bind `call`'s named values to the operands of `occurrence` (FR-367).
///
/// `Err` is [`super::run`]'s own refusal for the call's admission, and its
/// internal failure. `cancel` is polled as [`super::run`] polls it.
pub fn bind_scalar_operands(
    package: &BindPackage<'_>,
    call: &Call,
    occurrence: &ScalarOccurrence,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<BindOutcome, Box<RunRefusal>> {
    match bind(package, call, occurrence, objects, cancel) {
        Ok(bound) => Ok(BindOutcome::Bound(bound)),
        Err(Halt::Refused(refusal)) => Ok(BindOutcome::Refused(refusal)),
        Err(Halt::Incomplete(incomplete)) => Ok(BindOutcome::Incomplete(*incomplete)),
        Err(Halt::Run(refusal)) => Err(refusal),
    }
}

fn bind(
    package: &BindPackage<'_>,
    call: &Call,
    occurrence: &ScalarOccurrence,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<BoundScalarOperands, Halt> {
    let checked = package.checked.package();
    let selected = select(checked, &call.function)?;
    let arguments = bind_arguments(selected.parameters, &call.arguments)?;
    let types = checked.graph().scope().types();
    admit_model_arguments(
        package.models.models(),
        types,
        selected.parameters,
        &arguments,
        objects,
    )?;
    admit_values(
        selected.parameters,
        &arguments,
        call,
        objects,
        types,
        cancel,
    )?;
    if let Some(incomplete) = value_occurrences_exceeded(call, &arguments) {
        return Err(Halt::Incomplete(Box::new(incomplete)));
    }
    let (application, origin) = locate(checked, call, occurrence)?;
    let operands = package
        .ir
        .scalar_application_operands(&application, &origin)
        .map_err(BindRefusal::Operands)?;
    let resolver = Resolver::new(package, call, selected.parameters, &arguments)?;
    let operands = resolver.bind(&operands)?;
    Ok(BoundScalarOperands {
        package_id: package.package_id,
        occurrence: occurrence.clone(),
        operands,
    })
}

/// The admission [`super::run`]'s S6a call makes before it evaluates.
fn admit_values(
    parameters: &[(String, ValueType)],
    arguments: &[Value],
    call: &Call,
    objects: &ObjectEnvironment,
    types: &quire_semantic_value::declaration::TypeEnvironment,
    cancel: &Cancel,
) -> Result<(), Box<RunRefusal>> {
    let cancelled = |cause| Box::new(RunRefusal::Cancelled(cause));
    if let Some(cause) = cancel.cause() {
        return Err(cancelled(cause));
    }
    let mut run = qsl_semantics::value::supplied::SuppliedRun::new(call.supplied, cancel.clone())
        .map_err(|stop| Box::new(RunRefusal::Supplied(stop)))?;
    qsl_eval::value::admit_arguments(parameters, arguments, objects, types, &mut run).map_err(
        |failure| match failure {
            qsl_eval::value::CallFailure::Cancelled(cause) => cancelled(cause),
            other => convert_call_failure(other),
        },
    )?;
    match cancel.tripped() {
        Some(cause) => Err(cancelled(cause)),
        None => Ok(()),
    }
}

/// FR-098's occurrence count of each converted argument against the call's
/// `value_occurrences`. Nothing is charged to a meter.
fn value_occurrences_exceeded(call: &Call, arguments: &[Value]) -> Option<CallIncomplete> {
    let limit = call.accounting.value_occurrences;
    arguments.iter().find_map(|argument| {
        let occurrences = argument.occ();
        (occurrences > Integer::from(limit)).then(|| CallIncomplete {
            record: Incomplete {
                limit_kind: LimitKind::ValueOccurrences,
                limit,
                consumed: 0,
                next_charge: occurrences,
                charge_point: ChargePoint::FunctionCall,
            },
            location: None,
        })
    })
}

/// For each declared parameter, the index of its argument in
/// [`Call::arguments`]. The join was admitted, so each parameter has one.
fn supplied_positions(parameters: &[(String, ValueType)], call: &Call) -> Vec<usize> {
    parameters
        .iter()
        .filter_map(|(name, _)| {
            call.arguments
                .iter()
                .position(|argument| argument.parameter == *name)
        })
        .collect()
}

/// Check `occurrence` as FR-357's operator entry checks a claim's scalar
/// node, in the order FR-367 Behavior 2 states.
#[qsl_attrs::string_edge]
fn locate(
    checked: &CheckedPackage,
    call: &Call,
    occurrence: &ScalarOccurrence,
) -> Result<(CheckedNodeId, quire_contract_model::CheckedOccurrence), BindRefusal> {
    let identity = |cause| BindRefusal::ScalarIdentity(cause);
    if occurrence.function != call.function {
        return Err(identity(BindIdentityCause::Function {
            claimed: occurrence.function.clone(),
            called: call.function.clone(),
        }));
    }
    let node = occurrence.application;
    let not_a_node = || identity(BindIdentityCause::Node { node });
    let graph = checked.graph().semantic_graph();
    let semantic = graph
        .resolve_wire(node)
        .and_then(|key| graph.node(key))
        .ok_or_else(not_a_node)?;
    let callable = checked
        .graph()
        .callable(&call.function)
        .ok_or_else(not_a_node)?;
    if !in_body(graph, callable.identity, semantic.key()) {
        return Err(not_a_node());
    }
    if !ScalarOperator::ALL
        .into_iter()
        .any(|operator| applies(semantic, operator))
    {
        return Err(identity(BindIdentityCause::Operator { node }));
    }
    let absent = || {
        identity(BindIdentityCause::Occurrence {
            occurrence: occurrence.occurrence.clone(),
        })
    };
    if occurrence.occurrence.node() != node {
        return Err(absent());
    }
    let origin = occurrence.occurrence.origin();
    checked
        .graph()
        .occurrence(semantic.key(), origin)
        .filter(|location| {
            matches!(&location.origin, BodyOrigin::Body { function, .. }
                if *function == call.function)
        })
        .ok_or_else(absent)?;
    let checked_origin = checked_occurrence(origin).ok_or_else(absent)?;
    Ok((checked_node_id(node), checked_origin))
}

/// A bound operand before the `i64` check.
struct Resolved {
    ordinal: u64,
    value: Integer,
    range: CheckedScalarOperandRange,
    source: OperandSource,
}

/// Decides each operand's source and value (FR-367 Behavior 4 and 5).
struct Resolver<'a> {
    ir: &'a CheckedPackageV2,
    checked: &'a CheckedPackage,
    /// The called function's parameters in declared order.
    parameters: &'a [(String, ValueType)],
    /// Their node ids, in the same order.
    parameter_nodes: Vec<WireNodeId>,
    /// The admitted argument values, in declared parameter order.
    arguments: &'a [Value],
    /// Their indices in `Call.arguments`, in declared parameter order.
    supplied_at: Vec<usize>,
}

impl<'a> Resolver<'a> {
    fn new(
        package: &'a BindPackage<'_>,
        call: &Call,
        parameters: &'a [(String, ValueType)],
        arguments: &'a [Value],
    ) -> Result<Self, Box<RunRefusal>> {
        let checked = package.checked.package();
        let callable = checked
            .graph()
            .callable(&call.function)
            .ok_or_else(|| fault("selected-function-is-callable"))?;
        let parameter_nodes = callable_parameter_keys(checked, &callable)
            .map_err(|fault| Box::new(RunRefusal::Fault(fault)))?
            .into_iter()
            .map(|key| WireNodeId::from_digest(*key.as_bytes()))
            .collect();
        Ok(Self {
            ir: &package.ir,
            checked,
            parameters,
            parameter_nodes,
            arguments,
            supplied_at: supplied_positions(parameters, call),
        })
    }

    /// Resolve every operand's source in ordinal order, then check each
    /// value for `i64`, lowest ordinal first.
    fn bind(&self, operands: &[CheckedScalarOperand]) -> Result<Vec<BoundOperand>, Halt> {
        let resolved = operands
            .iter()
            .map(|operand| self.resolve(operand))
            .collect::<Result<Vec<_>, _>>()?;
        resolved
            .into_iter()
            .map(|operand| {
                let text = operand.value.to_string();
                let value = text
                    .parse::<i64>()
                    .map_err(|_| BindRefusal::OperandNotNative {
                        ordinal: operand.ordinal,
                        value: text,
                    })?;
                Ok(BoundOperand {
                    ordinal: operand.ordinal,
                    value,
                    range: operand.range,
                    source: operand.source,
                })
            })
            .collect()
    }

    fn resolve(&self, operand: &CheckedScalarOperand) -> Result<Resolved, Halt> {
        let ordinal = operand.ordinal;
        let resolved = |value, source| Resolved {
            ordinal,
            value,
            range: operand.range,
            source,
        };
        let id = match &operand.child {
            CheckedScalarOperandChild::InlineLiteral { .. } => {
                return Ok(resolved(
                    Integer::from(operand.range.lower),
                    OperandSource::InlineLiteral { node: None },
                ));
            }
            CheckedScalarOperandChild::GraphChild(id) => id,
        };
        let node = WireNodeId::from_hex(&id.digest)
            .ok_or_else(|| Halt::Run(fault("operand-child-id-is-a-wire-node-id")))?;
        if let Some(value) = self.literal_value(node) {
            return Ok(resolved(
                value,
                OperandSource::InlineLiteral { node: Some(node) },
            ));
        }
        let declaration = self
            .ir
            .parameter_declaration(id)
            .map_err(|error| match error {
                CheckedParameterDeclarationError::NotParameter => {
                    Halt::Refused(BindRefusal::OperandNotParameter { ordinal })
                }
                CheckedParameterDeclarationError::UnknownNode
                | CheckedParameterDeclarationError::MalformedBody => {
                    Halt::Run(fault("operand-parameter-node-is-well-formed"))
                }
            })?;
        let level = declaration.level.to_string();
        let Some(position) = self
            .parameter_nodes
            .iter()
            .position(|node_id| *node_id == node)
        else {
            return Err(Halt::Refused(BindRefusal::OperandNotArgument {
                ordinal,
                parameter: declaration.name.to_string(),
                level,
            }));
        };
        let (name, _) = self
            .parameters
            .get(position)
            .ok_or_else(|| Halt::Run(fault("parameter-node-has-a-declared-parameter")))?;
        if **name != *declaration.name {
            return Err(Halt::Run(fault("parameter-node-name-is-the-declared-name")));
        }
        let (Some(Value::Integer(value)), Some(argument)) =
            (self.arguments.get(position), self.supplied_at.get(position))
        else {
            return Err(Halt::Run(fault("integer-operand-has-an-integer-argument")));
        };
        Ok(resolved(
            value.clone(),
            OperandSource::Argument {
                argument: *argument,
                parameter: name.clone(),
                node,
                level,
            },
        ))
    }

    /// The value of `node` when the checked package holds it as a
    /// `value`/`literal` node of an integer.
    #[qsl_attrs::string_edge]
    fn literal_value(&self, node: WireNodeId) -> Option<Integer> {
        let graph = self.checked.graph().semantic_graph();
        let semantic = graph.resolve_wire(node).and_then(|key| graph.node(key))?;
        if semantic.node_tag() != NodeTag::Value || semantic.semantic_form() != "literal" {
            return None;
        }
        let preimage = serde_json::from_slice::<serde_json::Value>(semantic.preimage()).ok()?;
        let body = &preimage["body"];
        if body["value_kind"] != "integer" {
            return None;
        }
        body["value"].as_str()?.parse().ok()
    }
}

#[cfg(test)]
mod tests;
