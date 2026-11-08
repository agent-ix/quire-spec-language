// SPDX-License-Identifier: AGPL-3.0-or-later
//! The declaration forms of the extensions a unit selects (FR-354; QSpec
//! FR-467 "Production").
//!
//! S1 reads a selected extension's leading keywords and clause names from
//! these values alone. The caller builds them from definitions the catalog
//! validated and held; this layer-1 module names no catalog type.

/// One production of a selected extension: a leading keyword and its clause
/// names in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionProduction {
    keyword: String,
    clauses: Vec<String>,
}

impl ExtensionProduction {
    /// A production with leading keyword `keyword` and the given clause names.
    pub fn new<C>(keyword: impl Into<String>, clauses: C) -> Self
    where
        C: IntoIterator,
        C::Item: Into<String>,
    {
        Self {
            keyword: keyword.into(),
            clauses: clauses.into_iter().map(Into::into).collect(),
        }
    }

    /// The leading keyword.
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// The clause names in the production's source order.
    pub fn clauses(&self) -> &[String] {
        &self.clauses
    }
}

/// The productions of every extension a unit selects. The default selects
/// none, so no word is an extension keyword.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtensionForms {
    productions: Vec<ExtensionProduction>,
}

impl ExtensionForms {
    /// The forms of the given productions.
    pub fn new(productions: impl IntoIterator<Item = ExtensionProduction>) -> Self {
        Self {
            productions: productions.into_iter().collect(),
        }
    }

    /// The productions in selection order.
    pub fn productions(&self) -> &[ExtensionProduction] {
        &self.productions
    }
}
