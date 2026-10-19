// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 SYM-1 and SYM-7 (FR-151, FR-152): how a permutation of object
//! keys acts on a model state and a transition identity, and the sort
//! canonicaliser.
//!
//! A permutation renames every reference whose universe and key it moves,
//! in every field, collection and record, and moves each object of a
//! permuted population to its key's image; a set or bag is re-sorted into
//! its canonical-key order after the renaming. A transition identity is
//! renamed in its receiver and its reference arguments.

use std::collections::BTreeMap;

use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::state::{ModelState, StateObject};
use quire_canonical::Limits;
use quire_exact::{
    compare_keys, from_admitted, from_admitted_slots, CollectionKind, FieldValue, ObjectId,
    ObjectReference, OptionValue, UnionValue, UniverseId, Value,
};
use quire_semantic_value::model_transition::ModelTransition;

use super::encode::{self, declaration_text, Enc};
use super::walk::stop;
use super::{identity_limits, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::reduction::{Permutation, SymmetryGroup};

/// Where a permutation's keys live: each moved population's universes and
/// its key map.
struct Renaming<'p> {
    moved: Vec<(
        Vec<UniverseId>,
        &'p DeclarationKey,
        &'p BTreeMap<String, String>,
    )>,
}

impl Renaming<'_> {
    /// The reference `reference` is renamed to, when the permutation moves
    /// its key.
    fn reference(&self, reference: &ObjectReference) -> Option<ObjectReference> {
        self.moved.iter().find_map(|(universes, _, map)| {
            if !universes.contains(&reference.universe()) {
                return None;
            }
            let image = map.get(reference.object().as_str())?;
            let object = ObjectId::new(image.clone()).ok()?;
            Some(ObjectReference::new(
                reference.universe(),
                reference.object_type(),
                object,
            ))
        })
    }

    /// `value` with every reference renamed; a set or bag is re-sorted into
    /// its canonical-key order.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when two elements
    /// of a renamed set or bag have no canonical-key order.
    fn value(&self, value: &Value) -> Result<Value, ExpansionStop> {
        Ok(match value {
            Value::Reference(reference) => self
                .reference(reference)
                .map_or_else(|| value.clone(), Value::Reference),
            Value::Option(option) => OptionValue::from_admitted(
                option.payload_type().clone(),
                option
                    .payload()
                    .map(|payload| self.value(payload))
                    .transpose()?,
            ),
            Value::Collection(collection) => {
                let mut elements = collection
                    .elements()
                    .iter()
                    .map(|element| self.value(element))
                    .collect::<Result<Vec<Value>, _>>()?;
                if matches!(
                    collection.collection_type().kind(),
                    CollectionKind::Set | CollectionKind::Bag
                ) {
                    let mut unordered = false;
                    elements.sort_by(|left, right| {
                        compare_keys(left, right).unwrap_or_else(|| {
                            unordered = true;
                            std::cmp::Ordering::Equal
                        })
                    });
                    if unordered {
                        return Err(stop(BROKEN));
                    }
                }
                from_admitted(collection.collection_type().clone(), elements)
            }
            Value::Composite(composite) => {
                let slots = composite
                    .slots()
                    .iter()
                    .map(|slot| {
                        Ok(match slot {
                            FieldValue::Present(value) => FieldValue::Present(self.value(value)?),
                            FieldValue::Absent => FieldValue::Absent,
                            FieldValue::Null => FieldValue::Null,
                        })
                    })
                    .collect::<Result<Vec<FieldValue>, ExpansionStop>>()?;
                from_admitted_slots(composite.declaration(), slots.into_boxed_slice())
            }
            Value::Union(union) => UnionValue::from_admitted(
                union.member().clone(),
                union
                    .payload()
                    .iter()
                    .map(|value| self.value(value))
                    .collect::<Result<Vec<Value>, _>>()?,
            ),
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Float(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::String(_)
            | Value::Uuid(_)
            | Value::Timestamp(_)
            | Value::Enum(_)
            | Value::Population(_) => value.clone(),
        })
    }

    fn object(&self, object: &StateObject) -> Result<StateObject, ExpansionStop> {
        Ok(StateObject {
            type_identity: object.type_identity.clone(),
            fields: object
                .fields
                .iter()
                .map(|(field, value)| Ok((field.clone(), self.value(value)?)))
                .collect::<Result<_, ExpansionStop>>()?,
        })
    }

    fn key<'k>(&'k self, population: &DeclarationKey, key: &'k str) -> &'k str {
        self.moved
            .iter()
            .find(|(_, moved, _)| *moved == population)
            .and_then(|(_, _, map)| map.get(key))
            .map_or(key, String::as_str)
    }
}

impl ModelSystem<'_> {
    /// Whether the population `population` is declared `symmetric`
    /// (FR-150); `None` when no selected model declares it.
    pub fn is_symmetric(&self, population: &DeclarationKey) -> Option<bool> {
        self.populations
            .iter()
            .find(|declared| declared.key == *population)
            .map(|declared| declared.symmetric)
    }

    /// The keys of `population`'s universe, in the order the subject lists
    /// them; `None` when the subject has no universe for it.
    pub fn universe(&self, population: &DeclarationKey) -> Option<&[String]> {
        self.universes.get(population).map(Vec::as_slice)
    }

    /// The universes the objects of `population` are referenced in: one per
    /// declared member type.
    fn universes_of(&self, population: &DeclarationKey) -> Vec<UniverseId> {
        let mut universes = Vec::new();
        let Some(declared) = self
            .populations
            .iter()
            .find(|declared| declared.key == *population)
        else {
            return universes;
        };
        for member in &declared.member_types {
            // A reference's universe is its type's; any key names it.
            if let Ok(reference) = self.model.object_reference(member, "k") {
                if !universes.contains(&reference.universe()) {
                    universes.push(reference.universe());
                }
            }
        }
        universes
    }

    fn renaming<'p>(&self, permutation: &'p Permutation) -> Renaming<'p> {
        Renaming {
            moved: permutation
                .maps()
                .iter()
                .map(|(population, map)| (self.universes_of(population), population, map))
                .collect(),
        }
    }

    /// ADR-021 SYM-1: `permutation` applied to `state`: every object of a
    /// moved population moves to its key's image, and every reference whose
    /// key it moves is renamed, in every field, collection, record and
    /// relationship extent. The result has no reaching step.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when a renamed set
    /// or bag has no canonical-key order.
    pub fn permute(
        &self,
        state: &ModelSimState,
        permutation: &Permutation,
    ) -> Result<ModelSimState, ExpansionStop> {
        if permutation.is_identity() {
            return Ok(ModelSimState {
                state: state.state.clone(),
                anchor: state.anchor.clone(),
                reached_by: None,
            });
        }
        let renaming = self.renaming(permutation);
        let populations = state
            .state
            .populations
            .iter()
            .map(|(population, objects)| {
                let objects = objects
                    .iter()
                    .map(|(key, object)| {
                        Ok((
                            renaming.key(population, key).to_owned(),
                            renaming.object(object)?,
                        ))
                    })
                    .collect::<Result<_, ExpansionStop>>()?;
                Ok((population.clone(), objects))
            })
            .collect::<Result<_, ExpansionStop>>()?;
        let extents = state
            .state
            .extents
            .iter()
            .map(|extent| {
                let mut extent = extent.clone();
                for (source, target) in &mut extent.instances {
                    if let Some(renamed) = renaming.reference(source) {
                        *source = renamed;
                    }
                    if let Some(renamed) = renaming.reference(target) {
                        *target = renamed;
                    }
                }
                extent
            })
            .collect();
        Ok(ModelSimState {
            state: ModelState {
                populations,
                extents,
            },
            anchor: state.anchor.clone(),
            reached_by: None,
        })
    }

    /// ADR-021 SYM-1: `permutation` applied to `transition`: its receiver
    /// and each argument that is a reference, or an option of one, renamed.
    /// An argument that holds a reference inside another value is left as it
    /// is; a caller that needs the image to be a transition of the subject
    /// checks it against an expansion.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when a renamed
    /// identity does not encode.
    pub fn permute_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop> {
        if permutation.is_identity() {
            return Ok(transition.clone());
        }
        let renaming = self.renaming(permutation);
        let limits = Limits::new(identity_limits().input_bytes);
        let text = |enc: Enc| enc.canonical_text(limits).map_err(|_| stop(BROKEN));
        let rename = |argument: &str| -> Result<String, ExpansionStop> {
            if let Some(reference) = encode::parse_reference(argument) {
                return match renaming.reference(&reference) {
                    Some(renamed) => text(encode::reference_enc(&renamed)),
                    None => Ok(argument.to_owned()),
                };
            }
            match encode::parse_optional_reference(argument) {
                Some(Some(reference)) => match renaming.reference(&reference) {
                    Some(renamed) => text(Enc::tagged(
                        "option",
                        vec![("value", encode::reference_enc(&renamed))],
                    )),
                    None => Ok(argument.to_owned()),
                },
                Some(None) | None => Ok(argument.to_owned()),
            }
        };
        let receiver = transition.receiver().map(rename).transpose()?;
        let arguments = transition
            .arguments()
            .iter()
            .map(|argument| rename(argument))
            .collect::<Result<Vec<_>, _>>()?;
        ModelTransition::new(
            transition.operation().to_owned(),
            receiver,
            arguments,
            limits,
        )
        .map_err(|_| stop(BROKEN))
    }

    /// ADR-021 SYM-7 (FR-152): the sort canonicaliser. For each class of
    /// `group`, each key's sort key is the JCS bytes of the key-free
    /// encoding of the object it names (empty when the state holds none);
    /// the keys sorted ascending by sort key, ties in key order, are mapped
    /// in turn to the class's keys in key order. The canonical form is that
    /// permutation applied to `state`.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when an object's
    /// encoding has no canonical bytes.
    pub fn canonicalise(
        &self,
        state: &ModelSimState,
        group: &SymmetryGroup,
    ) -> Result<(ModelSimState, Permutation), ExpansionStop> {
        let mut permutation = Permutation::identity();
        if !group.is_trivial() {
            let limits = Limits::new(identity_limits().input_bytes);
            let mut placeholders: Vec<(Vec<UniverseId>, &DeclarationKey, &[String], Enc)> =
                Vec::new();
            for class in &group.classes {
                let text = declaration_text(&class.population, limits).map_err(|_| stop(BROKEN))?;
                placeholders.push((
                    self.universes_of(&class.population),
                    &class.population,
                    &class.keys,
                    Enc::Object(vec![
                        ("type", Enc::text("symmetric")),
                        ("population", Enc::text(text)),
                    ]),
                ));
            }
            let placeholder = |reference: &ObjectReference| {
                placeholders
                    .iter()
                    .find(|(universes, _, keys, _)| {
                        universes.contains(&reference.universe())
                            && keys.iter().any(|key| key == reference.object().as_str())
                    })
                    .map(|(_, _, _, enc)| enc.clone())
            };
            for class in &group.classes {
                let objects = state.state.populations.get(&class.population);
                let mut sorted = Vec::with_capacity(class.keys.len());
                for key in &class.keys {
                    let bytes = match objects.and_then(|objects| objects.get(key)) {
                        Some(object) => quire_canonical::to_vec(
                            &encode::object_enc(&self.model, object, &placeholder),
                            limits,
                        )
                        .map_err(|_| stop(BROKEN))?,
                        None => Vec::new(),
                    };
                    sorted.push((bytes, key));
                }
                // A stable sort keeps equal sort keys in key order.
                sorted.sort_by(|left, right| left.0.cmp(&right.0));
                permutation.insert(
                    &class.population,
                    sorted
                        .into_iter()
                        .zip(&class.keys)
                        .map(|((_, from), to)| (from.clone(), to.clone())),
                );
            }
        }
        Ok((self.permute(state, &permutation)?, permutation))
    }
}
