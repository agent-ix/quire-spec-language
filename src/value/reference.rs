// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed `Reference<T>` values and closed object environments (FR-143).
//!
//! A reference is terminal: its identity is the snapshot-supplied FR-009/FR-204
//! triple (universe, object-type declaration identity, object identity), and
//! equality never inspects the referenced state. No source form creates one.
//! Cycles between objects are representable only through references resolved
//! in an [`ObjectEnvironment`].

use std::collections::BTreeMap;

use super::composite::{
    fill_slots, ConstructionRefusal, FieldValue, ObjectTypeDeclaration, TypeEnvironment, Value,
};
use super::node::NodeKey;

/// A universe identity in its canonical identity bytes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UniverseIdentity(Box<[u8]>);

/// A declared object identity in its canonical identity bytes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectIdentity(Box<[u8]>);

/// An identity component with no canonical identity bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("an identity component has at least one canonical identity byte")]
pub struct InvalidObjectIdentity;

fn identity_bytes(bytes: &[u8]) -> Result<Box<[u8]>, InvalidObjectIdentity> {
    if bytes.is_empty() {
        return Err(InvalidObjectIdentity);
    }
    Ok(bytes.into())
}

impl UniverseIdentity {
    /// A universe from its canonical identity bytes.
    pub fn new(bytes: &[u8]) -> Result<Self, InvalidObjectIdentity> {
        identity_bytes(bytes).map(Self)
    }

    /// The canonical identity bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl ObjectIdentity {
    /// An object identity from its canonical identity bytes.
    pub fn new(bytes: &[u8]) -> Result<Self, InvalidObjectIdentity> {
        identity_bytes(bytes).map(Self)
    }

    /// The canonical identity bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A `Reference<T>` value: its identity triple.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectReference {
    universe: UniverseIdentity,
    object_type: NodeKey,
    identity: ObjectIdentity,
}

impl ObjectReference {
    /// The reference `(universe, object_type, identity)` supplied by a bound
    /// model snapshot.
    pub fn new(universe: UniverseIdentity, object_type: NodeKey, identity: ObjectIdentity) -> Self {
        Self {
            universe,
            object_type,
            identity,
        }
    }

    /// The universe identity.
    pub fn universe(&self) -> &UniverseIdentity {
        &self.universe
    }

    /// The object-type declaration identity.
    pub fn object_type(&self) -> NodeKey {
        self.object_type
    }

    /// The declared object identity.
    pub fn identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

/// Why an object environment is not closed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("object environment refused at {object:?}: {cause:?}")]
pub struct ObjectEnvironmentRefusal {
    /// The object where the refusal originates.
    pub object: ObjectReference,
    /// The typed cause.
    pub cause: ObjectEnvironmentCause,
}

/// The typed cause of an [`ObjectEnvironmentRefusal`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectEnvironmentCause {
    /// Two objects share one identity triple.
    DuplicateObject,
    /// The object type is not a model object type of the environment.
    UnknownObjectType,
    /// An attribute does not match its declaration.
    Attribute(ConstructionRefusal),
    /// A contained reference names no object of the environment.
    DanglingReference(Box<ObjectReference>),
}

/// A closed object environment: every reference held by any attribute
/// resolves to an object of the environment.
#[derive(Clone, Debug, Default)]
pub struct ObjectEnvironment {
    objects: BTreeMap<ObjectReference, Box<[FieldValue]>>,
}

impl ObjectEnvironment {
    /// Admit `objects` as `(reference, attributes)` pairs against the model
    /// object types of `types`. An omitted `?` attribute is `absent`.
    pub fn new<'n>(
        types: &TypeEnvironment,
        objects: impl IntoIterator<Item = (ObjectReference, Vec<(&'n str, FieldValue)>)>,
    ) -> Result<Self, ObjectEnvironmentRefusal> {
        let mut admitted = BTreeMap::new();
        for (reference, attributes) in objects {
            let refuse = |cause| ObjectEnvironmentRefusal {
                object: reference.clone(),
                cause,
            };
            let Some(declaration) = types.object_type(reference.object_type) else {
                return Err(refuse(ObjectEnvironmentCause::UnknownObjectType));
            };
            let slots = fill_slots(declaration.attributes(), attributes)
                .map_err(|refusal| refuse(ObjectEnvironmentCause::Attribute(refusal)))?;
            if admitted.contains_key(&reference) {
                return Err(refuse(ObjectEnvironmentCause::DuplicateObject));
            }
            admitted.insert(reference, slots);
        }
        let environment = Self { objects: admitted };
        for (owner, slots) in &environment.objects {
            environment.check_closed(owner, slots)?;
        }
        Ok(environment)
    }

    /// Whether the referenced object is in the environment.
    pub fn contains(&self, reference: &ObjectReference) -> bool {
        self.objects.contains_key(reference)
    }

    /// The named attribute slot of the referenced object.
    pub fn attribute(
        &self,
        types: &TypeEnvironment,
        reference: &ObjectReference,
        name: &str,
    ) -> Option<&FieldValue> {
        let declaration: &ObjectTypeDeclaration = types.object_type(reference.object_type)?;
        let position = declaration
            .attributes()
            .iter()
            .position(|attribute| attribute.name() == name)?;
        self.objects.get(reference)?.get(position)
    }

    fn check_closed(
        &self,
        owner: &ObjectReference,
        slots: &[FieldValue],
    ) -> Result<(), ObjectEnvironmentRefusal> {
        let mut pending: Vec<&Value> = present(slots).collect();
        while let Some(value) = pending.pop() {
            match value {
                Value::Reference(reference) if !self.objects.contains_key(reference) => {
                    return Err(ObjectEnvironmentRefusal {
                        object: owner.clone(),
                        cause: ObjectEnvironmentCause::DanglingReference(Box::new(
                            reference.clone(),
                        )),
                    });
                }
                Value::Option(option) => pending.extend(option.payload()),
                Value::Composite(composite) => pending.extend(present(composite.slots())),
                Value::Collection(collection) => pending.extend(collection.elements()),
                Value::Reference(_)
                | Value::Boolean(_)
                | Value::Integer(_)
                | Value::Rational(_)
                | Value::Decimal(_)
                | Value::Float(_)
                | Value::Quantity(_)
                | Value::Text(_)
                | Value::Enum(_) => {}
            }
        }
        Ok(())
    }
}

fn present(slots: &[FieldValue]) -> impl Iterator<Item = &Value> {
    slots.iter().filter_map(|slot| match slot {
        FieldValue::Present(value) => Some(value),
        FieldValue::Absent | FieldValue::Null => None,
    })
}
