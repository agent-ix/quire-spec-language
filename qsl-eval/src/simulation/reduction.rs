// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 EI-4 (FR-152, FR-158, FR-159, FR-162): what a
//! [`TransitionSystem`](crate::simulation::TransitionSystem) offers a model
//! checker that reduces its search, and the values its reduction hooks
//! return.
//!
//! A symmetry group is a product of symmetric groups over classes of object
//! keys of a population universe (ADR-021 SYM-1). A [`Permutation`] of the
//! group renames keys class by class; it acts on states and transition
//! identities through the system's own hooks, since only the system knows
//! where its references are.

use std::collections::BTreeMap;

use qsl_semantics::model::key::DeclarationKey;
use quire_exact::ObjectReference;
use serde::Serialize;

use crate::temporal::UndefinedCause;

/// The reductions a transition system offers (FR-159): one flag per hook it
/// implements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfferedReductions {
    /// The system implements `canonical`.
    pub symmetry: bool,
    /// The system implements `footprint`.
    pub partial_order: bool,
    /// The system implements `holds_constraint`.
    pub constraint: bool,
}

impl OfferedReductions {
    /// A system that offers no reduction.
    pub const NONE: Self = Self {
        symmetry: false,
        partial_order: false,
        constraint: false,
    };
}

/// One class of interchangeable keys of a population's universe (FR-151):
/// two or more keys, in ascending identity-byte order.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SymmetryClass {
    /// The population whose universe holds the keys.
    #[serde(serialize_with = "population_node")]
    pub population: DeclarationKey,
    /// The keys, ascending.
    pub keys: Vec<String>,
}

fn population_node<S: serde::Serializer>(
    population: &DeclarationKey,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&population.node)
}

/// A symmetry group (FR-151): the product of the symmetric group of each
/// class. The empty group is the trivial group.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SymmetryGroup {
    /// Every class of two or more keys, in declaration order.
    pub classes: Vec<SymmetryClass>,
}

impl SymmetryGroup {
    /// Whether the group is trivial.
    pub fn is_trivial(&self) -> bool {
        self.classes.is_empty()
    }

    /// The stabiliser of `key` of `population`: the group with `key`
    /// removed from its class, a class left with one key dropping out
    /// (FR-151 "Instances").
    pub fn stabiliser(&self, population: &DeclarationKey, key: &str) -> Self {
        let classes = self
            .classes
            .iter()
            .map(|class| {
                let mut class = class.clone();
                if class.population == *population {
                    class.keys.retain(|held| held != key);
                }
                class
            })
            .filter(|class| class.keys.len() >= 2)
            .collect();
        Self { classes }
    }

    /// The class of `population` that holds `key`, if any.
    pub fn class_of(&self, population: &DeclarationKey, key: &str) -> Option<&SymmetryClass> {
        self.classes
            .iter()
            .find(|class| class.population == *population && class.keys.iter().any(|k| k == key))
    }
}

/// A permutation of a symmetry group (FR-152): for each population, the
/// image of every key it moves. A key it does not list is fixed, so the
/// identity lists none and two equal permutations have equal maps.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Permutation {
    maps: BTreeMap<DeclarationKey, BTreeMap<String, String>>,
}

impl Permutation {
    /// The identity.
    pub fn identity() -> Self {
        Self::default()
    }

    /// The permutation that maps each `(from, to)` of `population`; pairs
    /// with `from == to` are dropped. The caller supplies a bijection.
    pub fn of(
        population: &DeclarationKey,
        pairs: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let mut permutation = Self::identity();
        permutation.insert(population, pairs);
        permutation
    }

    /// Add the moves `pairs` of `population` to this permutation.
    pub(crate) fn insert(
        &mut self,
        population: &DeclarationKey,
        pairs: impl IntoIterator<Item = (String, String)>,
    ) {
        let moved: BTreeMap<String, String> =
            pairs.into_iter().filter(|(from, to)| from != to).collect();
        if !moved.is_empty() {
            let map = self.maps.entry(population.clone()).or_default();
            debug_assert!(
                moved.keys().all(|key| !map.contains_key(key))
                    && moved
                        .values()
                        .all(|image| !map.values().any(|held| held == image)),
                "the moves of one population are disjoint from those it holds"
            );
            map.extend(moved);
        }
    }

    /// Whether this is the identity.
    pub fn is_identity(&self) -> bool {
        self.maps.is_empty()
    }

    /// The image of `key` of `population`.
    pub fn image<'k>(&'k self, population: &DeclarationKey, key: &'k str) -> &'k str {
        self.maps
            .get(population)
            .and_then(|map| map.get(key))
            .map_or(key, String::as_str)
    }

    /// The moved keys of each population, with their images.
    pub fn maps(&self) -> &BTreeMap<DeclarationKey, BTreeMap<String, String>> {
        &self.maps
    }

    /// `self ∘ other`: apply `other`, then `self`.
    #[must_use]
    pub fn compose(&self, other: &Self) -> Self {
        let mut result = Self::identity();
        let populations: std::collections::BTreeSet<&DeclarationKey> =
            self.maps.keys().chain(other.maps.keys()).collect();
        for population in populations {
            let keys = self
                .maps
                .get(population)
                .into_iter()
                .chain(other.maps.get(population))
                .flat_map(BTreeMap::keys);
            let pairs: Vec<(String, String)> = keys
                .map(|key| {
                    let image = self.image(population, other.image(population, key));
                    (key.clone(), image.to_owned())
                })
                .collect();
            result.insert(population, pairs);
        }
        result
    }

    /// The inverse.
    #[must_use]
    pub fn inverse(&self) -> Self {
        let mut result = Self::identity();
        for (population, map) in &self.maps {
            result.insert(
                population,
                map.iter().map(|(from, to)| (to.clone(), from.clone())),
            );
        }
        result
    }
}

/// How a state constraint reads at one state (FR-158).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstraintValue {
    /// The constraint holds: the state is expanded.
    Holds,
    /// The constraint is false: the state is a boundary state.
    Boundary,
    /// The constraint has no value at the state, for the first object in
    /// key order whose evaluation is undefined.
    Undefined {
        /// That object.
        object: ObjectReference,
        /// Why it has no value.
        cause: UndefinedCause,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn population() -> DeclarationKey {
        DeclarationKey {
            package: "test/sym".to_owned(),
            node: "ix://test/sym/counters".to_owned(),
        }
    }

    fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
            .collect()
    }

    /// A permutation composed with its inverse is the identity, and
    /// composition applies its right operand first.
    #[test]
    fn composition_applies_the_right_operand_first() {
        let p = population();
        let swap = Permutation::of(&p, pairs(&[("a", "b"), ("b", "a")]));
        let cycle = Permutation::of(&p, pairs(&[("a", "b"), ("b", "c"), ("c", "a")]));
        assert!(cycle.compose(&cycle.inverse()).is_identity());
        let both = swap.compose(&cycle);
        // cycle maps a to b, then swap maps b to a.
        assert_eq!(both.image(&p, "a"), "a");
        assert_eq!(both.image(&p, "b"), "c");
        assert_eq!(both.image(&p, "c"), "b");
    }

    /// The stabiliser drops the bound key, and a class left with one key.
    #[test]
    fn the_stabiliser_drops_the_bound_key() {
        let p = population();
        let group = SymmetryGroup {
            classes: vec![SymmetryClass {
                population: p.clone(),
                keys: vec!["a".into(), "b".into(), "c".into()],
            }],
        };
        let stabiliser = group.stabiliser(&p, "a");
        assert_eq!(stabiliser.classes[0].keys, ["b", "c"]);
        assert!(stabiliser.stabiliser(&p, "b").is_trivial());
    }
}
