// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-480 "Selections and imports": a `model` alias resolves only within the
//! unit that declares it. A unit may select one domain package under several
//! aliases, and two units may bind one alias to two packages. The package's
//! declarations are projected once, under the first alias that selects it
//! (its type environment names, operation table, relationships, effects and
//! populations), and every alias of the same identity, in any unit, names
//! the same declarations. Type and node identity stay those of the package
//! declaration, independent of the alias (QSpec FR-322-AC-71, AC-132 and
//! AC-142, FR-480-AC-3).
//!
//! [`ModelSpellings`] holds each selected package's selection and type
//! names by member `T`, and each unit's model aliases bound to their
//! packages. A model-qualified type name `N::T`, and a model alias `N`
//! itself, resolve through the alias `N` the writing unit binds, so the same
//! spelling in two units can name two packages, and a unit never reaches a
//! package through another unit's alias.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::source::provenance::RawSourceRef;
use quire_exact::{EffectiveId, ValueType};

use crate::model::domain_package::DomainPackageRef;

/// What one package-owned type name `M::T` names.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ModelName {
    /// Every type the name binds, in resolution order.
    pub(crate) types: Vec<ValueType>,
    /// The object type the name declares, by its effective identity.
    pub(crate) object: Option<EffectiveId>,
    /// The variant type the name declares, by its effective identity.
    pub(crate) variant: Option<EffectiveId>,
}

/// One selected package's type names, by member `T` of `M::T`.
pub(crate) type PackageNames = BTreeMap<String, ModelName>;

/// A package's projected type names, each under its first alias `first`, as
/// that package's names by member. A name not of the form `first::T` is not
/// the package's.
pub(crate) fn package_names<'n>(
    first: &str,
    declared: impl IntoIterator<Item = (&'n str, ModelName)>,
) -> PackageNames {
    let mut names = PackageNames::new();
    for (name, found) in declared {
        let Some(member) = name
            .strip_prefix(first)
            .and_then(|rest| rest.strip_prefix("::"))
        else {
            continue;
        };
        let entry = names.entry(member.to_owned()).or_default();
        entry.types.extend(found.types);
        entry.object = entry.object.take().or(found.object);
        entry.variant = entry.variant.take().or(found.variant);
    }
    names
}

/// How a unit's spelling of a qualified name reads.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Spelled<'a> {
    /// Its qualifier is a model alias of some unit: the name of the package
    /// the writing unit binds that alias to, or `None` when the writing unit
    /// binds no such alias or its package declares no such name.
    Model(Option<&'a ModelName>),
    /// Its qualifier is no model alias: a unit or library name.
    Other,
}

/// One selected package: its selection and its type names.
#[derive(Clone, Debug)]
struct Package {
    selection: DomainPackageRef,
    names: PackageNames,
}

/// Each selected package's type names and each source unit's model aliases.
#[derive(Clone, Debug, Default)]
pub(crate) struct ModelSpellings {
    /// The unit a lookup with no source unit is written in (the single-source
    /// assembly path).
    primary: Option<RawSourceRef>,
    /// Each unit's model aliases, to their package's position in `packages`.
    units: BTreeMap<RawSourceRef, BTreeMap<String, usize>>,
    /// Each unit's import aliases, which a model alias of another unit
    /// leaves to the unit.
    others: BTreeMap<RawSourceRef, BTreeSet<String>>,
    /// Every unit's model aliases.
    model_aliases: BTreeSet<String>,
    /// Each selected package.
    packages: Vec<Package>,
}

impl ModelSpellings {
    /// No packages yet, with `primary` the unit a lookup with no source unit
    /// is written in.
    pub(crate) fn new(primary: RawSourceRef) -> Self {
        Self {
            primary: Some(primary),
            ..Self::default()
        }
    }

    /// Add the selected package `selection` with no names yet, returning
    /// its position.
    pub(crate) fn add_package(&mut self, selection: DomainPackageRef) -> usize {
        self.packages.push(Package {
            selection,
            names: PackageNames::new(),
        });
        self.packages.len() - 1
    }

    /// Give the package at `package` its type names.
    pub(crate) fn set_names(&mut self, package: usize, names: PackageNames) {
        if let Some(slot) = self.packages.get_mut(package) {
            slot.names = names;
        }
    }

    /// The package `unit` (the primary unit when `None`) binds its model
    /// alias `alias` to; `None` when the unit binds no admitted model alias
    /// `alias`.
    fn package(&self, unit: Option<&RawSourceRef>, alias: &str) -> Option<&Package> {
        let package = self
            .units
            .get(unit.or(self.primary.as_ref())?)?
            .get(alias)?;
        self.packages.get(*package)
    }

    /// The selected package `unit` (the primary unit when `None`) binds its
    /// model alias `alias` to.
    pub(crate) fn selection(
        &self,
        unit: Option<&RawSourceRef>,
        alias: &str,
    ) -> Option<&DomainPackageRef> {
        self.package(unit, alias).map(|package| &package.selection)
    }

    /// `unit` selects a model under `alias`, bound to the package at
    /// `package`, or to no package when its selection did not admit.
    pub(crate) fn bind_model(&mut self, unit: &RawSourceRef, alias: &str, package: Option<usize>) {
        self.model_aliases.insert(alias.to_owned());
        if let Some(package) = package {
            self.units
                .entry(unit.clone())
                .or_default()
                .entry(alias.to_owned())
                .or_insert(package);
        }
    }

    /// `unit` declares the import alias `alias`.
    pub(crate) fn bind_other(&mut self, unit: &RawSourceRef, alias: &str) {
        self.others
            .entry(unit.clone())
            .or_default()
            .insert(alias.to_owned());
    }

    /// How `name`, written in `unit` (the primary unit when `None`), reads.
    pub(crate) fn spelled(&self, unit: Option<&RawSourceRef>, name: &str) -> Spelled<'_> {
        let Some((alias, member)) = name.split_once("::") else {
            return Spelled::Other;
        };
        if !self.model_aliases.contains(alias) {
            return Spelled::Other;
        }
        if unit
            .or(self.primary.as_ref())
            .and_then(|unit| self.others.get(unit))
            .is_some_and(|others| others.contains(alias))
        {
            return Spelled::Other;
        }
        Spelled::Model(
            self.package(unit, alias)
                .and_then(|package| package.names.get(member)),
        )
    }

    /// Every type `name`, written in `unit`, binds: through the unit's model
    /// alias, or `table`'s entry when its qualifier is no model alias.
    pub(crate) fn types<'a>(
        &'a self,
        unit: Option<&RawSourceRef>,
        name: &str,
        table: &'a BTreeMap<String, Vec<ValueType>>,
    ) -> &'a [ValueType] {
        match self.spelled(unit, name) {
            Spelled::Model(found) => found.map_or(&[], |found| found.types.as_slice()),
            Spelled::Other => table.get(name).map_or(&[], Vec::as_slice),
        }
    }

    /// The object type `name`, written in `unit`, names through the unit's
    /// model alias. Every object type is package-owned, so a name whose
    /// qualifier is no model alias names none.
    pub(crate) fn object_type(
        &self,
        unit: Option<&RawSourceRef>,
        name: &str,
    ) -> Option<EffectiveId> {
        match self.spelled(unit, name) {
            Spelled::Model(found) => found.and_then(|found| found.object.clone()),
            Spelled::Other => None,
        }
    }

    /// Whether `name`, written in `unit`, names a package-owned type through
    /// the unit's model alias.
    pub(crate) fn declares(&self, unit: Option<&RawSourceRef>, name: &str) -> bool {
        matches!(self.spelled(unit, name), Spelled::Model(Some(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(identity: &str) -> RawSourceRef {
        RawSourceRef::new("repo".to_owned(), identity.to_owned()).expect("a source reference")
    }

    fn selection(identity: &str) -> DomainPackageRef {
        DomainPackageRef {
            identity: identity.to_owned(),
        }
    }

    fn scalar(value_type: ValueType) -> ModelName {
        ModelName {
            types: vec![value_type],
            ..ModelName::default()
        }
    }

    /// Unit `a` selects package 0 as `M` and `N`, unit `b` binds `N` to
    /// package 1, and both declare `T`.
    fn spellings() -> ModelSpellings {
        let mut spellings = ModelSpellings::new(source("a"));
        let (x, y) = (
            spellings.add_package(selection("x")),
            spellings.add_package(selection("y")),
        );
        spellings.set_names(
            x,
            package_names(
                "M",
                [
                    ("M::T", scalar(ValueType::String)),
                    ("Q::U", scalar(ValueType::Timestamp)),
                ],
            ),
        );
        spellings.set_names(y, package_names("N", [("N::T", scalar(ValueType::Uuid))]));
        spellings.bind_model(&source("a"), "M", Some(x));
        spellings.bind_model(&source("a"), "N", Some(x));
        spellings.bind_model(&source("b"), "N", Some(y));
        spellings.bind_other(&source("b"), "M");
        spellings
    }

    /// FR-480-AC-14, AC-15, AC-16: one spelling names each unit's own package.
    #[test]
    fn each_unit_reads_a_spelling_through_its_own_alias() {
        let spellings = spellings();
        let read = |unit: &str, name: &str| match spellings.spelled(Some(&source(unit)), name) {
            Spelled::Model(found) => found.cloned(),
            Spelled::Other => panic!("{name} is a model spelling"),
        };
        assert_eq!(read("a", "N::T"), Some(scalar(ValueType::String)));
        assert_eq!(read("a", "M::T"), Some(scalar(ValueType::String)));
        assert_eq!(read("b", "N::T"), Some(scalar(ValueType::Uuid)));
        assert_eq!(read("a", "M::U"), None, "a name of no first alias is none");
        // No unit reaches a package through another unit's alias.
        assert_eq!(read("c", "N::T"), None);
        // The primary unit reads a spelling with no source unit.
        assert_eq!(
            spellings.spelled(None, "N::T"),
            Spelled::Model(Some(&scalar(ValueType::String)))
        );
    }

    /// FR-480 "Selections and imports": a model alias names the package its
    /// writing unit binds it to, never another unit's.
    #[test]
    fn each_unit_reads_a_model_alias_as_its_own_package() {
        let spellings = spellings();
        let read = |unit: Option<&str>, alias: &str| {
            spellings
                .selection(unit.map(source).as_ref(), alias)
                .map(|selection| selection.identity.clone())
        };
        assert_eq!(read(Some("a"), "N").as_deref(), Some("x"));
        assert_eq!(read(Some("b"), "N").as_deref(), Some("y"));
        assert_eq!(read(None, "N").as_deref(), Some("x"), "the primary unit's");
        assert_eq!(read(Some("b"), "M"), None, "b's M is an import alias");
        assert_eq!(read(Some("c"), "N"), None);
    }

    /// A unit's own non-model alias, and a qualifier no unit binds as a
    /// model alias, read the unit's table.
    #[test]
    fn a_non_model_qualifier_reads_the_table() {
        let spellings = spellings();
        let table = BTreeMap::from([("M::T".to_owned(), vec![ValueType::Boolean])]);
        assert_eq!(
            spellings.types(Some(&source("b")), "M::T", &table),
            [ValueType::Boolean]
        );
        assert_eq!(
            spellings.spelled(Some(&source("a")), "E::m"),
            Spelled::Other
        );
        assert_eq!(spellings.spelled(Some(&source("a")), "T"), Spelled::Other);
        assert!(spellings
            .types(Some(&source("c")), "M::T", &table)
            .is_empty());
    }
}
