// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed `Reference<T>` values and closed object environments (FR-143).
//!
//! A reference is terminal: equality compares its qualified object identity
//! and never inspects the referenced state. Cycles between objects are
//! representable only through references resolved in an [`ObjectEnvironment`].

use std::collections::BTreeMap;

use super::composite::{FieldValue, Value};
use super::node::{is_qualified_name, NodeKey};

/// A qualified object identity.
// SPEC-GAP(119-7): FR-149 names "qualified object identity" but no source form
// or canonical preimage for it. The identity here is a non-empty sequence of
// identifier segments.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectIdentity(Box<[String]>);

/// A malformed qualified object identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("an object identity is a non-empty sequence of identifiers")]
pub struct InvalidObjectIdentity;

impl ObjectIdentity {
    /// An identity from its qualified segments.
    pub fn new(segments: Vec<String>) -> Result<Self, InvalidObjectIdentity> {
        if is_qualified_name(&segments) {
            Ok(Self(segments.into_boxed_slice()))
        } else {
            Err(InvalidObjectIdentity)
        }
    }

    /// The qualified segments.
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

/// A `Reference<T>` value: object state declaration `T` and object identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ObjectReference {
    object_type: NodeKey,
    identity: ObjectIdentity,
}

impl ObjectReference {
    /// A reference to `identity` whose state has declaration `object_type`.
    pub fn new(object_type: NodeKey, identity: ObjectIdentity) -> Self {
        Self {
            object_type,
            identity,
        }
    }

    /// The referenced state declaration.
    pub fn object_type(&self) -> NodeKey {
        self.object_type
    }

    /// The qualified object identity.
    pub fn identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

/// Why an object environment is not closed.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("object environment refused at {identity:?}: {cause:?}")]
pub struct ObjectEnvironmentRefusal {
    /// The object where the refusal originates.
    pub identity: ObjectIdentity,
    /// The typed cause.
    pub cause: ObjectEnvironmentCause,
}

/// The typed cause of an [`ObjectEnvironmentRefusal`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ObjectEnvironmentCause {
    /// Two objects share one identity.
    DuplicateIdentity,
    /// The state is not a composite value of the object's declaration.
    StateTypeMismatch,
    /// A contained reference names no object of the environment.
    DanglingReference(ObjectIdentity),
    /// A contained reference's type differs from its target object's type.
    ReferenceTypeMismatch(ObjectIdentity),
}

/// A closed object environment: every reference contained in any object state
/// resolves to an object of the referenced declaration.
#[derive(Clone, Debug, Default)]
pub struct ObjectEnvironment {
    objects: BTreeMap<ObjectIdentity, (NodeKey, Value)>,
}

impl ObjectEnvironment {
    /// Admit `objects` as `(reference, state)` pairs.
    pub fn new(
        objects: impl IntoIterator<Item = (ObjectReference, Value)>,
    ) -> Result<Self, ObjectEnvironmentRefusal> {
        let mut admitted = BTreeMap::new();
        for (reference, state) in objects {
            let refuse = |cause| ObjectEnvironmentRefusal {
                identity: reference.identity.clone(),
                cause,
            };
            if !matches!(&state, Value::Composite(c) if c.declaration() == reference.object_type) {
                return Err(refuse(ObjectEnvironmentCause::StateTypeMismatch));
            }
            if admitted.contains_key(&reference.identity) {
                return Err(refuse(ObjectEnvironmentCause::DuplicateIdentity));
            }
            admitted.insert(reference.identity, (reference.object_type, state));
        }
        let environment = Self { objects: admitted };
        for (identity, (_, state)) in &environment.objects {
            environment.check_closed(identity, state)?;
        }
        Ok(environment)
    }

    /// The state of the referenced object.
    pub fn resolve(&self, reference: &ObjectReference) -> Option<&Value> {
        self.objects
            .get(&reference.identity)
            .filter(|(object_type, _)| *object_type == reference.object_type)
            .map(|(_, state)| state)
    }

    fn check_closed(
        &self,
        owner: &ObjectIdentity,
        state: &Value,
    ) -> Result<(), ObjectEnvironmentRefusal> {
        let mut pending = vec![state];
        while let Some(value) = pending.pop() {
            match value {
                Value::Reference(reference) => {
                    let cause = match self.objects.get(&reference.identity) {
                        None => ObjectEnvironmentCause::DanglingReference,
                        Some((object_type, _)) if *object_type != reference.object_type => {
                            ObjectEnvironmentCause::ReferenceTypeMismatch
                        }
                        Some(_) => continue,
                    };
                    return Err(ObjectEnvironmentRefusal {
                        identity: owner.clone(),
                        cause: cause(reference.identity.clone()),
                    });
                }
                Value::Option(option) => pending.extend(option.payload()),
                Value::Composite(composite) => {
                    pending.extend(composite.slots().iter().filter_map(|slot| match slot {
                        FieldValue::Present(value) => Some(value),
                        FieldValue::Absent | FieldValue::Null => None,
                    }));
                }
                Value::Collection(collection) => pending.extend(collection.elements()),
                Value::Boolean(_)
                | Value::Integer(_)
                | Value::Rational(_)
                | Value::Decimal(_)
                | Value::Quantity(_)
                | Value::Text(_)
                | Value::Enum(_) => {}
            }
        }
        Ok(())
    }
}
