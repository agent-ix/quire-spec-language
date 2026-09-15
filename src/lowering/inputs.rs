// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-034: select primitive backend inputs from a completely validated native context.

use super::{NativeProjection, ProjectedRead, ProjectedReadOrigin};
use crate::linking::DeclarationKey;
use crate::runtime::{
    Invocation, ObjectIdentity, ObservationSelection, QualifiedName, RuntimeReference, Snapshot,
    ValidatedContext, ValueId, ValueNode,
};
use quire_contract_ir as ir;

/// Caller-lowered work budget for a fresh input materialization.
#[derive(Clone, Copy, Debug)]
pub struct InputProjectionLimits {
    /// Inspected reads, fields, parameters and arena values; at most 100,000.
    pub work: usize,
}

impl Default for InputProjectionLimits {
    fn default() -> Self {
        Self { work: 100_000 }
    }
}

/// Materialization stop reason; none of these represents predicate truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum InputProjectionCode {
    /// The context was validated against a different checked package.
    #[error("context belongs to another checked package")]
    ContextMismatch,
    /// A per-call work ceiling was reached.
    #[error("input projection work limit")]
    ResourceExhausted,
    /// The caller's poll requested cancellation.
    #[error("input projection cancelled")]
    Cancelled,
    /// Immutable checked/validated correspondence was unexpectedly unavailable.
    #[error("validated input correspondence invariant: {0}")]
    Invariant(InputProjectionInvariant),
}

/// A defensive failure of constructor-private checked or validated authority.
///
/// Missing observations, invocation parameters and model mismatches are refused
/// by native validation before it can produce a `ValidatedContext`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum InputProjectionInvariant {
    /// The read's checked observation has no selected snapshot.
    #[error("selected snapshot unavailable")]
    MissingSnapshot,
    /// The checked operation read has no selected invocation.
    #[error("selected invocation unavailable")]
    MissingInvocation,
    /// The selected self differs from the linked field's owner or record.
    #[error("selected self has a different model or record")]
    SelfIdentityMismatch,
    /// The validated complete population lacks the selected self.
    #[error("selected self object unavailable")]
    MissingSelfObject,
    /// The validated self record lacks the linked field.
    #[error("validated self field unavailable")]
    MissingSelfField,
    /// The validated invocation lacks the linked parameter.
    #[error("captured parameter unavailable")]
    MissingParameter,
    /// The validated snapshot lacks the required State root.
    #[error("validated state value unavailable")]
    MissingState,
    /// The projection's read origin and linked declaration kind disagree.
    #[error("projected read origin disagrees with linked declaration")]
    ReadOriginMismatch,
    /// An already validated arena address cannot be indexed on this host.
    #[error("validated arena address {value:?} exceeds host width")]
    ArenaIndexWidth {
        /// Original address within the selected artifact.
        value: ValueId,
    },
    /// An already validated address is absent from its selected arena.
    #[error("validated arena address {value:?} is unavailable")]
    MissingArenaValue {
        /// Original address within the selected artifact.
        value: ValueId,
    },
    /// An admitted primitive read selects a nonprimitive arena value.
    #[error("validated arena address {value:?} is not primitive")]
    NonPrimitiveValue {
        /// Original address within the selected artifact.
        value: ValueId,
    },
}

/// Classified atomic failure with no partial backend inputs.
#[derive(Debug, thiserror::Error)]
#[error("{code}")]
#[non_exhaustive]
pub struct InputProjectionError {
    /// Stable stop classification.
    pub code: InputProjectionCode,
    /// Exact original declaration and observations for an invariant failure.
    pub read: Option<Box<ProjectedRead>>,
    /// Work completed before stopping.
    pub work: usize,
}

/// A primitive copied without interpreting or evaluating a predicate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveValue {
    /// Exact native Boolean.
    Boolean(bool),
    /// Exact native bounded signed integer.
    Integer(i64),
}

/// One backend value and its exact original native location.
#[derive(Debug)]
pub struct PrimitiveInput<'a> {
    read: &'a ProjectedRead,
    value: PrimitiveValue,
    artifact: InputArtifact<'a>,
    object: Option<&'a ObjectIdentity>,
    arena_value: ValueId,
}

#[derive(Debug)]
enum InputArtifact<'a> {
    Snapshot(&'a Snapshot),
    Invocation(&'a Invocation),
}

impl PrimitiveInput<'_> {
    /// Linked declaration, model and checked native/IR observations.
    pub fn read(&self) -> &ProjectedRead {
        self.read
    }
    /// Actual primitive selected from the validated artifact.
    pub fn value(&self) -> PrimitiveValue {
        self.value
    }
    /// Exact selected snapshot or captured invocation identity and byte digest.
    pub fn artifact(&self) -> RuntimeReference {
        match self.artifact {
            InputArtifact::Snapshot(snapshot) => RuntimeReference::Snapshot(snapshot.reference()),
            InputArtifact::Invocation(invocation) => {
                RuntimeReference::Invocation(invocation.reference())
            }
        }
    }
    /// Exact self identity when the input came from a context field.
    pub fn object(&self) -> Option<&ObjectIdentity> {
        self.object
    }
    /// Primitive node's original artifact-local index.
    pub fn arena_value(&self) -> ValueId {
        self.arena_value
    }
}

/// Complete inputs for one selected clause, retaining their native validation.
#[derive(Debug)]
pub struct InputProjection<'a, 'checked, 'model> {
    context: &'a ValidatedContext<'checked, 'model>,
    bound: &'a ir::BoundPackage,
    inputs: Vec<PrimitiveInput<'a>>,
    work: usize,
}

impl<'a, 'checked, 'model> InputProjection<'a, 'checked, 'model> {
    /// The original exact package, selected request and offered artifacts.
    pub fn context(&self) -> &'a ValidatedContext<'checked, 'model> {
        self.context
    }
    /// Exact bound projection whose selected clause consumes these inputs.
    pub fn bound(&self) -> &'a ir::BoundPackage {
        self.bound
    }
    /// Selected clause inputs in projection read order.
    pub fn inputs(&self) -> &[PrimitiveInput<'a>] {
        &self.inputs
    }
    /// Actual inspected entries in this fresh call.
    pub fn work(&self) -> usize {
        self.work
    }
}

struct Budget<P> {
    maximum: usize,
    work: usize,
    poll: P,
}

impl<P: FnMut() -> bool> Budget<P> {
    fn error(&self, code: InputProjectionCode) -> InputProjectionError {
        InputProjectionError {
            code,
            read: None,
            work: self.work,
        }
    }
    fn invariant(
        &self,
        read: &ProjectedRead,
        cause: InputProjectionInvariant,
    ) -> InputProjectionError {
        InputProjectionError {
            code: InputProjectionCode::Invariant(cause),
            read: Some(Box::new(read.clone())),
            work: self.work,
        }
    }
    fn check_cancelled(&mut self) -> Result<(), InputProjectionError> {
        if (self.poll)() {
            return Err(self.error(InputProjectionCode::Cancelled));
        }
        Ok(())
    }
    fn step(&mut self) -> Result<(), InputProjectionError> {
        self.check_cancelled()?;
        if self.work >= self.maximum {
            return Err(self.error(InputProjectionCode::ResourceExhausted));
        }
        self.work += 1;
        Ok(())
    }
}

impl<'model> NativeProjection<'_, 'model> {
    /// Materialize actual primitive reads after complete native runtime validation.
    ///
    /// This does not evaluate a clause or authenticate a backend result. The exact
    /// in-memory checked package must match; an independently reconstructed context
    /// must be validated again against `self.native().checked()` before use here.
    /// As in native validation/evaluation, a true poll means cancel; a poll panic unwinds.
    pub fn inputs<'a, 'checked>(
        &'a self,
        context: &'a ValidatedContext<'checked, 'model>,
        limits: InputProjectionLimits,
        poll: impl FnMut() -> bool,
    ) -> Result<InputProjection<'a, 'checked, 'model>, InputProjectionError> {
        let mut budget = Budget {
            maximum: limits.work.min(InputProjectionLimits::default().work),
            work: 0,
            poll,
        };
        if !std::ptr::eq(context.checked(), self.native().checked()) {
            return Err(budget.error(InputProjectionCode::ContextMismatch));
        }
        budget.check_cancelled()?;
        let selection = context.selection();
        let mut inputs = Vec::new();
        // Charge even unselected entries: a large package cannot hide scan work.
        for read in self.reads() {
            budget.step()?;
            if read.clause.requirement() != &selection.requirement
                || read.clause.clause() != &selection.clause
            {
                continue;
            }
            inputs.push(materialize(context, read, &mut budget)?);
        }
        Ok(InputProjection {
            context,
            bound: self.bound(),
            inputs,
            work: budget.work,
        })
    }
}

fn materialize<'a>(
    context: &'a ValidatedContext<'_, '_>,
    read: &'a ProjectedRead,
    budget: &mut Budget<impl FnMut() -> bool>,
) -> Result<PrimitiveInput<'a>, InputProjectionError> {
    let owner = &read.declaration.identity.owner;
    let snapshot = || {
        context
            .snapshot(read.native_observation)
            .ok_or_else(|| budget.invariant(read, InputProjectionInvariant::MissingSnapshot))
    };
    let (arena, arena_value, artifact, object) = match (read.origin, &read.declaration.identity.key)
    {
        (ProjectedReadOrigin::SelfField, DeclarationKey::Field { record, field }) => {
            let identity = match &context.selection().observation {
                ObservationSelection::Current { self_object, .. } => self_object,
                ObservationSelection::Invocation { .. } => {
                    &context
                        .invocation()
                        .ok_or_else(|| {
                            budget.invariant(read, InputProjectionInvariant::MissingInvocation)
                        })?
                        .draft()
                        .self_object
                }
            };
            if &identity.model != owner || &identity.record != record {
                return Err(budget.invariant(read, InputProjectionInvariant::SelfIdentityMismatch));
            }
            let snapshot = snapshot()?;
            let object = context
                .object(read.native_observation, identity)
                .ok_or_else(|| {
                    budget.invariant(read, InputProjectionInvariant::MissingSelfObject)
                })?;
            let mut value = None;
            for binding in &object.fields {
                budget.step()?;
                if &binding.name == field {
                    value = Some(binding.value);
                    break;
                }
            }
            (
                &snapshot.draft().arena,
                value.ok_or_else(|| {
                    budget.invariant(read, InputProjectionInvariant::MissingSelfField)
                })?,
                InputArtifact::Snapshot(snapshot),
                Some(identity),
            )
        }
        (ProjectedReadOrigin::Value(kind), DeclarationKey::Value(name)) => match kind {
            ir::ValueDeclarationKind::Input => {
                let invocation = context.invocation().ok_or_else(|| {
                    budget.invariant(read, InputProjectionInvariant::MissingInvocation)
                })?;
                let mut value = None;
                for binding in &invocation.draft().parameters {
                    budget.step()?;
                    if &binding.declaration.model == owner && &binding.declaration.name == name {
                        value = Some(binding.value);
                        break;
                    }
                }
                (
                    &invocation.draft().arena,
                    value.ok_or_else(|| {
                        budget.invariant(read, InputProjectionInvariant::MissingParameter)
                    })?,
                    InputArtifact::Invocation(invocation),
                    None,
                )
            }
            ir::ValueDeclarationKind::State => {
                let snapshot = snapshot()?;
                let value = context
                    .state(
                        read.native_observation,
                        &QualifiedName {
                            model: owner.clone(),
                            name: name.clone(),
                        },
                    )
                    .ok_or_else(|| {
                        budget.invariant(read, InputProjectionInvariant::MissingState)
                    })?;
                (
                    &snapshot.draft().arena,
                    value,
                    InputArtifact::Snapshot(snapshot),
                    None,
                )
            }
        },
        _ => return Err(budget.invariant(read, InputProjectionInvariant::ReadOriginMismatch)),
    };
    budget.step()?;
    let index = usize::try_from(arena_value.index()).map_err(|_| {
        budget.invariant(
            read,
            InputProjectionInvariant::ArenaIndexWidth { value: arena_value },
        )
    })?;
    let value = match arena.get(index) {
        Some(ValueNode::Boolean { value }) => PrimitiveValue::Boolean(*value),
        Some(ValueNode::Integer { value }) => PrimitiveValue::Integer(*value),
        Some(_) => {
            return Err(budget.invariant(
                read,
                InputProjectionInvariant::NonPrimitiveValue { value: arena_value },
            ))
        }
        None => {
            return Err(budget.invariant(
                read,
                InputProjectionInvariant::MissingArenaValue { value: arena_value },
            ))
        }
    };
    Ok(PrimitiveInput {
        read,
        value,
        artifact,
        object,
        arena_value,
    })
}
