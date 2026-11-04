// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-480 "Selections and imports": a unit may select one domain package
//! under several `model` aliases. The package's declarations are projected
//! once, under the first alias that selects it (its type environment names,
//! operation table, relationships, effects and populations), and every
//! further alias is a second spelling of the same declarations. Type and
//! node identity stay those of the package declaration, independent of the
//! alias (QSpec FR-322-AC-71, AC-132 and AC-142, FR-480-AC-3).
//!
//! [`ModelSpellings`] holds, for each package-owned type name `M::T` of the
//! first alias `M`, its spelling `N::T` under each further alias `N` of the
//! same package identity. A name table indexed by declared name gains each
//! spelling as another key for the same entry.

use std::collections::BTreeMap;

/// Each further-alias spelling of a package-owned type name, to the name it
/// spells (the declared name under the package's first alias).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ModelSpellings(BTreeMap<String, String>);

impl ModelSpellings {
    /// The spellings of `declared` names under each of `aliases`, where
    /// `declared` are the package-owned type names projected under the
    /// package's first alias `first`. A name not of the form `first::T`
    /// gains no spelling.
    pub(crate) fn add<'n>(
        &mut self,
        first: &str,
        aliases: &[&str],
        declared: impl IntoIterator<Item = &'n str>,
    ) {
        for name in declared {
            let Some(member) = name
                .strip_prefix(first)
                .and_then(|rest| rest.strip_prefix("::"))
            else {
                continue;
            };
            for alias in aliases {
                self.0
                    .entry(format!("{alias}::{member}"))
                    .or_insert_with(|| name.to_owned());
            }
        }
    }

    /// Each spelling and the declared name it spells, in spelling order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(spelled, declared)| (spelled.as_str(), declared.as_str()))
    }

    /// `table` with each spelling bound to its declared name's entry. A
    /// spelling that already names an entry keeps it: the first binding of a
    /// name wins, as it does for the declared names themselves.
    pub(crate) fn bind_first<V: Clone>(&self, table: &mut BTreeMap<String, V>) {
        for (spelled, declared) in self.iter() {
            if let Some(value) = table.get(declared).cloned() {
                table.entry(spelled.to_owned()).or_insert(value);
            }
        }
    }

    /// `table` with each spelling also binding every candidate its declared
    /// name binds. A spelling that already binds candidates keeps them and
    /// gains these, so a name two declarations reach stays ambiguous.
    pub(crate) fn bind_all<V: Clone>(&self, table: &mut BTreeMap<String, Vec<V>>) {
        for (spelled, declared) in self.iter() {
            if let Some(candidates) = table.get(declared).cloned() {
                table
                    .entry(spelled.to_owned())
                    .or_default()
                    .extend(candidates);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spellings() -> ModelSpellings {
        let mut spellings = ModelSpellings::default();
        spellings.add(
            "Config",
            &["History", "Audit"],
            ["Config::ConfigVersion", "Config::VersionNumber", "Other::X"],
        );
        spellings
    }

    #[test]
    fn each_further_alias_spells_each_first_alias_name() {
        let found: Vec<_> = spellings()
            .iter()
            .map(|(s, d)| (s.to_owned(), d.to_owned()))
            .collect();
        assert_eq!(
            found,
            [
                ("Audit::ConfigVersion", "Config::ConfigVersion"),
                ("Audit::VersionNumber", "Config::VersionNumber"),
                ("History::ConfigVersion", "Config::ConfigVersion"),
                ("History::VersionNumber", "Config::VersionNumber"),
            ]
            .map(|(s, d)| (s.to_owned(), d.to_owned()))
        );
    }

    #[test]
    fn a_bound_spelling_keeps_its_first_entry_and_candidates_accumulate() {
        let spellings = spellings();
        let mut first = BTreeMap::from([
            ("Config::ConfigVersion".to_owned(), 1),
            ("History::ConfigVersion".to_owned(), 2),
        ]);
        spellings.bind_first(&mut first);
        assert_eq!(first["History::ConfigVersion"], 2);
        assert_eq!(first["Audit::ConfigVersion"], 1);
        assert!(!first.contains_key("Audit::VersionNumber"));

        let mut all = BTreeMap::from([
            ("Config::ConfigVersion".to_owned(), vec![1]),
            ("History::ConfigVersion".to_owned(), vec![2]),
        ]);
        spellings.bind_all(&mut all);
        assert_eq!(all["History::ConfigVersion"], [2, 1]);
        assert_eq!(all["Audit::ConfigVersion"], [1]);
    }
}
