// SPDX-License-Identifier: AGPL-3.0-or-later
//! The core object closure of an object environment over the kernel
//! [`ObjectReference`] (QSpec FR-143-AC-3 and AC-9; ADR-011 §6.2). Its
//! admission rules are FR-106-AC-10's.
//!
//! A reference is terminal: its identity is the snapshot-supplied
//! QSpec FR-009/FR-204 triple (universe, object-type declaration identity, object
//! identity), and equality never inspects the referenced state. No source
//! form creates one. Cycles between objects are representable only through
//! references resolved in an [`ObjectClosure`].
//!
//! The closure names only kernel ids and SV values, checked against SV's
//! [`TypeEnvironment`], so it sits in SV beside `containment`. FR-089's
//! `PopulationId` -> `PopulationBinding` correspondence is not here: the
//! binding is layer-3 `model`'s, so `model::object_environment` holds that
//! map beside the closure it builds.

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::declaration::{fill_slots, ConstructionRefusal, FieldRef, TypeEnvironment};
use quire_exact::{FieldValue, ObjectReference, UniverseId, Value};

/// Why an object closure does not close.
///
/// `object` is boxed: `ObjectReference` grew past a fixed-size 32-byte
/// `UniverseId` (ADR-013 §8 OQ-C ruling), which pushed this refusal's stack
/// size over `clippy::result_large_err`'s threshold on the cold refusal
/// path.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("object closure refused at {object:?}: {cause:?}")]
pub struct ObjectClosureRefusal {
    /// The object where the refusal originates.
    pub object: Box<ObjectReference>,
    /// The typed cause.
    pub cause: ObjectClosureCause,
}

/// The typed cause of an [`ObjectClosureRefusal`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectClosureCause {
    /// Two objects share one identity triple.
    DuplicateObject,
    /// The object type is not a model object type of the environment.
    UnknownObjectType,
    /// An attribute does not match its declaration.
    Attribute(ConstructionRefusal),
    /// A contained reference names no object of the closure.
    DanglingReference(Box<ObjectReference>),
}

/// A closed set of objects: every reference held by any attribute resolves
/// to an object of the closure.
#[derive(Clone, Debug, Default)]
pub struct ObjectClosure {
    objects: BTreeMap<ObjectReference, Box<[FieldValue]>>,
}

impl ObjectClosure {
    /// Admit `objects` as `(reference, attributes)` pairs against the model
    /// object types of `types`. An omitted `?` attribute is `absent`.
    ///
    /// Every reference any attribute holds must name an object of the
    /// closure, except a reference in `tolerated_dangling`: the exact
    /// targets a caller has already decided may dangle. FR-106 admission
    /// passes the references its check 8 skipped, because they name an
    /// incomplete population nothing requires; every other caller passes
    /// `&[]`.
    pub fn new<'n>(
        types: &TypeEnvironment,
        objects: impl IntoIterator<Item = (ObjectReference, Vec<(&'n str, FieldValue)>)>,
        tolerated_dangling: &[ObjectReference],
    ) -> Result<Self, ObjectClosureRefusal> {
        let mut admitted = BTreeMap::new();
        for (reference, attributes) in objects {
            let refuse = |cause| ObjectClosureRefusal {
                object: Box::new(reference.clone()),
                cause,
            };
            let Some(declared) = types.attributes(reference.object_type()) else {
                return Err(refuse(ObjectClosureCause::UnknownObjectType));
            };
            let slots = fill_slots(types, declared, attributes)
                .map_err(|refusal| refuse(ObjectClosureCause::Attribute(refusal)))?;
            if admitted.contains_key(&reference) {
                return Err(refuse(ObjectClosureCause::DuplicateObject));
            }
            admitted.insert(reference, slots);
        }
        let closure = Self { objects: admitted };
        let tolerated: BTreeSet<&ObjectReference> = tolerated_dangling.iter().collect();
        for (owner, slots) in &closure.objects {
            closure.check_closed(owner, slots, &tolerated)?;
        }
        Ok(closure)
    }

    /// The closure's own reference whose universe is `universe` and
    /// declared key is `object`, whatever its most-specific type is (FR-109:
    /// a `Function` selection's object argument names an object by
    /// population and key alone, with no declared type of its own to
    /// narrow the search). `None` when no admitted object matches, or more
    /// than one does (an object identity is unique within one universe, so
    /// more than one match is a broken admission invariant, not a real
    /// ambiguity).
    pub fn find(&self, universe: UniverseId, object: &str) -> Option<&ObjectReference> {
        let mut found = self.objects.keys().filter(|reference| {
            reference.universe() == universe && reference.object().as_str() == object
        });
        let first = found.next()?;
        match found.next() {
            None => Some(first),
            Some(_) => None,
        }
    }

    /// Whether `reference` names an object of the closure.
    pub fn contains(&self, reference: &ObjectReference) -> bool {
        self.objects.contains_key(reference)
    }

    /// The referenced object's slot for `field`, the field `deref(r).f`
    /// resolved to in `r`'s static type. The object's own type
    /// conforms to that static type, so its effective attribute set has
    /// exactly one attribute standing for `field`: `field` itself when
    /// inherited unchanged, or the field that redefines it.
    pub fn attribute(
        &self,
        types: &TypeEnvironment,
        reference: &ObjectReference,
        field: &FieldRef,
    ) -> Option<&FieldValue> {
        let position = types
            .attributes(reference.object_type())?
            .iter()
            .position(|attribute| attribute.stands_for(field))?;
        self.objects.get(reference)?.get(position)
    }

    fn check_closed(
        &self,
        owner: &ObjectReference,
        slots: &[FieldValue],
        tolerated: &BTreeSet<&ObjectReference>,
    ) -> Result<(), ObjectClosureRefusal> {
        let mut pending: Vec<&Value> = present(slots).collect();
        while let Some(value) = pending.pop() {
            match value {
                Value::Reference(reference)
                    if !self.objects.contains_key(reference) && !tolerated.contains(reference) =>
                {
                    return Err(ObjectClosureRefusal {
                        object: Box::new(owner.clone()),
                        cause: ObjectClosureCause::DanglingReference(Box::new(reference.clone())),
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
                | Value::Enum(_)
                | Value::Population(_) => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::{FieldDeclaration, ObjectTypeDeclaration};
    use alloc::vec;
    use ix_trace_rs::trace;
    use quire_exact::{EffectiveId, ObjectId, Presence, ValueType};

    fn object_type(byte: u8) -> EffectiveId {
        EffectiveId::from_digest([byte; 32])
    }

    const A: u8 = 1;
    const B: u8 = 2;
    const UNDECLARED: u8 = 3;

    /// Model object types `A` and `B`, each with one required `Int`
    /// attribute `x`.
    fn types() -> TypeEnvironment {
        let declare = |byte, name| {
            ObjectTypeDeclaration::new(
                object_type(byte),
                name,
                vec![FieldDeclaration::new(
                    "x",
                    ValueType::Integer,
                    Presence::Required,
                )],
            )
        };
        TypeEnvironment::new([], [declare(A, "A"), declare(B, "B")])
            .expect("two object types admit")
    }

    fn reference(type_byte: u8, key: &str) -> ObjectReference {
        ObjectReference::new(
            UniverseId::from_digest([9; 32]),
            object_type(type_byte),
            ObjectId::new(key).expect("non-empty key"),
        )
    }

    fn object(reference: &ObjectReference) -> (ObjectReference, Vec<(&'static str, FieldValue)>) {
        (
            reference.clone(),
            vec![("x", FieldValue::Present(Value::Integer(1_i64.into())))],
        )
    }

    /// TC-904 row 1: a second object with an admitted identity triple
    /// refuses `DuplicateObject`, naming that triple.
    #[trace("TC-904", "FR-106-AC-10")]
    #[test]
    fn a_duplicate_identity_triple_refuses() {
        let a = reference(A, "a");
        assert_eq!(
            ObjectClosure::new(&types(), [object(&a), object(&a)], &[]).unwrap_err(),
            ObjectClosureRefusal {
                object: Box::new(a),
                cause: ObjectClosureCause::DuplicateObject,
            }
        );
    }

    /// TC-904 row 2: an object whose type names no model object type of the
    /// environment refuses `UnknownObjectType`.
    #[trace("TC-904", "FR-106-AC-10")]
    #[test]
    fn a_non_model_object_type_refuses() {
        let stray = reference(UNDECLARED, "s");
        assert_eq!(
            ObjectClosure::new(&types(), [object(&stray)], &[]).unwrap_err(),
            ObjectClosureRefusal {
                object: Box::new(stray),
                cause: ObjectClosureCause::UnknownObjectType,
            }
        );
    }

    /// TC-904 rows 3 and 4: `find` returns the one object a universe and
    /// key name, and none when two objects of different types share them.
    #[trace("TC-904", "FR-106-AC-10")]
    #[test]
    fn find_returns_none_for_an_ambiguous_key() {
        let universe = UniverseId::from_digest([9; 32]);
        let a = reference(A, "k");
        let single = ObjectClosure::new(&types(), [object(&a)], &[]).unwrap();
        assert_eq!(single.find(universe, "k"), Some(&a));

        let b = reference(B, "k");
        let both = ObjectClosure::new(&types(), [object(&a), object(&b)], &[]).unwrap();
        assert!(both.contains(&a) && both.contains(&b));
        assert_eq!(both.find(universe, "k"), None);
    }
}
