// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-363: the relationship ends a receiver type navigates.
//!
//! A relationship is read under the role of the end nearest the receiver: the
//! source end's role names the forward end, and the target end's role, when it
//! declares one, names the inverse end.

use std::collections::BTreeMap;

use quire_exact::EffectiveId;
use quire_semantic_value::declaration::TypeEnvironment;

use crate::model::domain_package::{Multiplicity, RelationshipDirection};
use crate::model::key::DeclarationKey;

/// One end of a relationship.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EndSide {
    /// The source end.
    Source,
    /// The target end.
    Target,
}

/// One declared end: the object type it names, its role and its multiplicity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndDeclaration {
    /// The object type the end names, by effective identity.
    pub object: EffectiveId,
    /// The end's declared role.
    pub role: Option<String>,
    /// The end's declared multiplicity; `None` when it declares none.
    pub multiplicity: Option<Multiplicity>,
}

/// One relationship between two object types of an admitted model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipDeclaration {
    /// The relationship's own declaration key.
    pub key: DeclarationKey,
    /// The declared traversal direction.
    pub direction: RelationshipDirection,
    /// The source end.
    pub source: EndDeclaration,
    /// The target end.
    pub target: EndDeclaration,
}

impl RelationshipDeclaration {
    /// The end on `side`.
    pub fn end(&self, side: EndSide) -> &EndDeclaration {
        match side {
            EndSide::Source => &self.source,
            EndSide::Target => &self.target,
        }
    }
}

/// A relationship end navigable from a receiver type, nearest end first.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Navigable<'a> {
    pub(crate) relationship: &'a RelationshipDeclaration,
    /// The end nearest the receiver, whose role names the navigation.
    pub(crate) near: EndSide,
    /// The near end's role: the member name the source wrote.
    pub(crate) role: &'a str,
}

impl<'a> Navigable<'a> {
    /// The end the navigation reaches, which its multiplicity types.
    pub(crate) fn destination(&self) -> &'a EndDeclaration {
        match self.near {
            EndSide::Source => &self.relationship.target,
            EndSide::Target => &self.relationship.source,
        }
    }

    /// Whether the relationship's direction admits this traversal: forward
    /// along `source-to-target`, `bidirectional` or `undirected`, inverse
    /// along `target-to-source`, `bidirectional` or `undirected`.
    pub(crate) fn direction_admitted(&self) -> bool {
        match (self.near, self.relationship.direction) {
            (
                EndSide::Source,
                RelationshipDirection::SourceToTarget
                | RelationshipDirection::Bidirectional
                | RelationshipDirection::Undirected,
            )
            | (
                EndSide::Target,
                RelationshipDirection::TargetToSource
                | RelationshipDirection::Bidirectional
                | RelationshipDirection::Undirected,
            ) => true,
            (EndSide::Source, RelationshipDirection::TargetToSource)
            | (EndSide::Target, RelationshipDirection::SourceToTarget) => false,
        }
    }
}

/// Every relationship of the admitted models, indexed by near-end role.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Relationships {
    declared: Vec<RelationshipDeclaration>,
    /// A role to each (relationship, near end) that role names.
    by_role: BTreeMap<String, Vec<(usize, EndSide)>>,
}

impl Relationships {
    /// Every end of `declared` indexed by its role.
    pub fn new(declared: Vec<RelationshipDeclaration>) -> Self {
        let mut by_role: BTreeMap<String, Vec<(usize, EndSide)>> = BTreeMap::new();
        for (position, relationship) in declared.iter().enumerate() {
            for side in [EndSide::Source, EndSide::Target] {
                if let Some(role) = &relationship.end(side).role {
                    by_role
                        .entry(role.clone())
                        .or_default()
                        .push((position, side));
                }
            }
        }
        Self { declared, by_role }
    }

    /// The object type the end on `side` of the relationship `key` names.
    pub(crate) fn end_object(&self, key: &DeclarationKey, side: EndSide) -> Option<EffectiveId> {
        self.declared
            .iter()
            .find(|relationship| &relationship.key == key)
            .map(|relationship| relationship.end(side).object)
    }

    /// The relationship ends `name` names that a value of the object type
    /// `receiver` navigates: those whose near end's type `receiver` conforms
    /// to.
    pub(crate) fn navigable<'a>(
        &'a self,
        types: &'a TypeEnvironment,
        receiver: EffectiveId,
        name: &str,
    ) -> impl Iterator<Item = Navigable<'a>> {
        self.by_role
            .get_key_value(name)
            .into_iter()
            .flat_map(|(role, ends)| ends.iter().map(move |end| (role.as_str(), end)))
            .filter_map(move |(role, &(position, near))| {
                let relationship = self.declared.get(position)?;
                types
                    .conforms(receiver, relationship.end(near).object)
                    .then_some(Navigable {
                        relationship,
                        near,
                        role,
                    })
            })
    }
}
