// SPDX-License-Identifier: AGPL-3.0-or-later
//! Extension definitions held by a catalog (FR-354 "Definition validation";
//! QSpec FR-467 "Members" and "Validation").
//!
//! [`DefinitionCatalog::hold_extensions`] reads the `grammar_schema` and
//! `typed_node_schema` members of each selected extension definition from the
//! definition's exact bytes, validates them by FR-467's three rules in order,
//! and returns only definitions that passed as [`HeldExtensions`]. Later
//! stages read held definitions only. Like `link_bundle`, it names no
//! `check`, `package`, `model`, emitter or backend type (FR-111-CON-1): the
//! core keywords and core node kinds the rules compare against are supplied
//! by the caller as a [`CoreVocabulary`].

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::Code;
use quire_contract_model::CheckedExtensionDefinition;
use serde_json::Value;

use super::bundle::{Definition, DefinitionCatalog};

/// The core grammar's keywords and the checked package's core node kinds, the
/// names an extension's leading keywords and node kinds must not equal
/// (FR-467 rule 2). The caller selects them for the grammar edition in force.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreVocabulary {
    keywords: BTreeSet<String>,
    node_kinds: BTreeSet<String>,
}

impl CoreVocabulary {
    /// A vocabulary of the given core keywords and core node kinds.
    pub fn new<K, N>(keywords: K, node_kinds: N) -> Self
    where
        K: IntoIterator,
        K::Item: Into<String>,
        N: IntoIterator,
        N::Item: Into<String>,
    {
        Self {
            keywords: keywords.into_iter().map(Into::into).collect(),
            node_kinds: node_kinds.into_iter().map(Into::into).collect(),
        }
    }
}

impl CoreVocabulary {
    /// Whether `word` is a core keyword.
    pub fn contains_keyword(&self, word: &str) -> bool {
        self.keywords.contains(word)
    }

    /// Whether `kind` is a core node kind.
    pub fn contains_node_kind(&self, kind: &str) -> bool {
        self.node_kinds.contains(kind)
    }
}

/// One clause of a production: its name and the spelling of the core
/// `type-ref` or node kind its value checks against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clause {
    /// The clause name.
    pub name: String,
    /// The type or node kind spelling from the typed-node schema entry.
    pub type_ref: String,
}

/// One declaration form of an extension, its grammar production joined to its
/// typed-node schema entry by leading keyword.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Production {
    /// The leading keyword.
    pub keyword: String,
    /// The node kind of the typed-node schema entry.
    pub node_kind: String,
    /// The clauses in the production's source order.
    pub clauses: Vec<Clause>,
}

/// An extension definition that passed validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldExtension {
    definition: DefinitionRef,
    productions: Vec<Production>,
    /// The `keyword` of each node entry in `typed_node_schema` order; rule 1
    /// guarantees every entry has one.
    entry_keywords: Vec<String>,
    /// The two FR-467 members, read strictly from the definition's bytes: the
    /// evidence IR's reader joins an `extension` node against.
    checked: CheckedExtensionDefinition,
}

impl HeldExtension {
    /// The definition as IR's reader holds it.
    pub fn checked_definition(&self) -> &CheckedExtensionDefinition {
        &self.checked
    }

    /// The definition's authority and identity.
    pub fn definition(&self) -> &DefinitionRef {
        &self.definition
    }

    /// The definition's productions in `grammar_schema` order.
    pub fn productions(&self) -> &[Production] {
        &self.productions
    }
}

/// The validated extension definitions of a unit's selections, by definition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HeldExtensions {
    held: BTreeMap<DefinitionRef, HeldExtension>,
}

impl HeldExtensions {
    /// The held extension `definition` names, if it was selected and passed
    /// validation.
    pub fn get(&self, definition: &DefinitionRef) -> Option<&HeldExtension> {
        self.held.get(definition)
    }

    /// The held extensions in definition order.
    pub fn iter(&self) -> impl Iterator<Item = &HeldExtension> {
        self.held.values()
    }

    /// The number of held extensions.
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether no extension is held.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}

/// The member of an extension definition FR-467 rule 1 finds missing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissingMember {
    /// No `grammar_schema`.
    GrammarSchema,
    /// No `typed_node_schema`.
    TypedNodeSchema,
    /// The grammar schema has no production.
    Production,
    /// A production at `index` has no leading keyword.
    ProductionKeyword {
        /// The production's index in `grammar_schema`.
        index: usize,
    },
    /// A `grammar_schema` production clause has no name.
    ProductionClauseName {
        /// The production's leading keyword.
        keyword: String,
        /// The clause's index in the production's `clauses`.
        index: usize,
    },
    /// A `typed_node_schema` entry clause has no name.
    EntryClauseName {
        /// The entry's keyword.
        keyword: String,
        /// The clause's index in the entry's `clauses`.
        index: usize,
    },
    /// A node entry at `index` has no `keyword`.
    EntryKeyword {
        /// The entry's index in `typed_node_schema`.
        index: usize,
    },
    /// A production with no typed-node schema entry.
    NodeEntry {
        /// The production's leading keyword.
        keyword: String,
    },
    /// A node entry with no `node_kind`.
    NodeKind {
        /// The entry's keyword.
        keyword: String,
    },
    /// A production clause with no entry in its node entry.
    ClauseEntry {
        /// The production's leading keyword.
        keyword: String,
        /// The clause name.
        clause: String,
    },
    /// A clause entry with no `type`.
    ClauseType {
        /// The production's leading keyword.
        keyword: String,
        /// The clause name.
        clause: String,
    },
    /// A node-entry clause the production does not list.
    UnlistedClause {
        /// The production's leading keyword.
        keyword: String,
        /// The clause name.
        clause: String,
    },
    /// A node entry whose `keyword` matches no production; the missing member
    /// is that production.
    ProductionForEntry {
        /// The entry's keyword.
        keyword: String,
    },
    /// A schema member that is not exactly the FR-467 shape: a member the
    /// shape lacks or does not name.
    SchemaShape(String),
}

/// What a FR-467 rule 2 collision is over.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictKind {
    /// A production's leading keyword.
    Keyword,
    /// A node entry's node kind.
    NodeKind,
    /// A node entry's `keyword`, held by two entries of one definition.
    EntryKeyword,
}

/// What the colliding name equals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConflictWith {
    /// A core keyword or core node kind.
    CoreGrammar,
    /// A production or entry of another selected extension.
    Definition(DefinitionRef),
    /// Another production or entry of the same definition, by index.
    SameDefinition {
        /// The earlier index.
        first: usize,
        /// The later index.
        second: usize,
    },
}

/// The closed cause tag of an extension refusal, under
/// `quire.native.diagnostics/v1`. Selection resolution uses FR-111's codes;
/// every other extension refusal is `invalid_package`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExtensionCause {
    /// `invalid_package`: FR-467 rule 1.
    MissingMember,
    /// `invalid_package`: FR-467 rule 2.
    ConflictingDefinition,
    /// `invalid_package`: FR-467 rule 3.
    DefinitionCycle,
    /// `unknown_profile`: a selected extension is in no catalog definition.
    UnsupportedSelection,
    /// `missing_import`: a dependency edge names a definition the catalog
    /// does not hold.
    MissingSelection,
    /// `ambiguous_declaration`: one identity is selected under two
    /// authorities.
    ConflictingAuthority,
}

impl ExtensionCause {
    /// The stable catalog tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingMember => "missing-member",
            Self::ConflictingDefinition => "conflicting-definition",
            Self::DefinitionCycle => "definition-cycle",
            Self::UnsupportedSelection => "unsupported-selection",
            Self::MissingSelection => "missing-selection",
            Self::ConflictingAuthority => "conflicting-authority",
        }
    }

    /// The code the cause is catalogued under.
    pub fn code(self) -> Code {
        match self {
            Self::MissingMember | Self::ConflictingDefinition | Self::DefinitionCycle => {
                Code::InvalidPackage
            }
            Self::UnsupportedSelection => Code::UnknownProfile,
            Self::MissingSelection => Code::MissingImport,
            Self::ConflictingAuthority => Code::AmbiguousDeclaration,
        }
    }
}

/// The first violated FR-467 rule over the selected extension definitions.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ExtensionRefusal {
    /// A selected extension is in no catalog definition (FR-111).
    #[error("no catalog definition for the selected extension {0:?}")]
    UnsupportedSelection(DefinitionRef),
    /// A dependency edge of `dependent` names a definition the catalog does
    /// not hold (FR-111).
    #[error("extension {dependent:?} depends on {dependency:?}, which the catalog does not hold")]
    MissingDependency {
        /// The definition holding the edge.
        dependent: DefinitionRef,
        /// The absent definition.
        dependency: DefinitionRef,
    },
    /// One identity is selected under two authorities (FR-111).
    #[error("conflicting authorities for one identity: {first:?} and {second:?}")]
    ConflictingAuthority {
        /// The earlier selection.
        first: DefinitionRef,
        /// The later selection.
        second: DefinitionRef,
    },
    /// Rule 1: a member is missing.
    #[error("extension {definition:?} is missing {member:?}")]
    MissingMember {
        /// The definition.
        definition: DefinitionRef,
        /// What is missing.
        member: MissingMember,
    },
    /// Rule 2: a leading keyword or node kind collides.
    #[error("extension {definition:?} {kind:?} `{name}` collides with {with:?}")]
    ConflictingDefinition {
        /// The definition holding the later name.
        definition: DefinitionRef,
        /// What collides.
        kind: ConflictKind,
        /// The keyword or node kind.
        name: String,
        /// What it collides with.
        with: ConflictWith,
    },
    /// Rule 3: the dependency edges contain a cycle.
    #[error("extension dependency cycle {0:?}")]
    DefinitionCycle(Vec<DefinitionRef>),
}

impl ExtensionRefusal {
    /// The cause tag.
    pub fn cause(&self) -> ExtensionCause {
        match self {
            Self::UnsupportedSelection(_) => ExtensionCause::UnsupportedSelection,
            Self::MissingDependency { .. } => ExtensionCause::MissingSelection,
            Self::ConflictingAuthority { .. } => ExtensionCause::ConflictingAuthority,
            Self::MissingMember { .. } => ExtensionCause::MissingMember,
            Self::ConflictingDefinition { .. } => ExtensionCause::ConflictingDefinition,
            Self::DefinitionCycle(_) => ExtensionCause::DefinitionCycle,
        }
    }

    /// The refusal code.
    pub fn code(&self) -> Code {
        self.cause().code()
    }
}

impl DefinitionCatalog {
    /// Validate the extension definitions `selected` names and hold those
    /// that pass (FR-354; QSpec FR-467 "Validation").
    ///
    /// Reports the first violated rule over all selected definitions and no
    /// later one: a missing member (rule 1), then a keyword or node-kind
    /// collision (rule 2), then a dependency cycle (rule 3).
    pub fn hold_extensions(
        &self,
        selected: &[DefinitionRef],
        core: &CoreVocabulary,
    ) -> Result<HeldExtensions, ExtensionRefusal> {
        let definitions = self.resolve_selected(selected)?;
        let mut held = Vec::with_capacity(definitions.len());
        for definition in &definitions {
            let (productions, entry_keywords) =
                read_productions(definition).map_err(|member| ExtensionRefusal::MissingMember {
                    definition: definition.exact().clone(),
                    member,
                })?;
            let checked =
                strict_definition(definition).map_err(|error| ExtensionRefusal::MissingMember {
                    definition: definition.exact().clone(),
                    member: MissingMember::SchemaShape(error.to_string()),
                })?;
            held.push(HeldExtension {
                definition: definition.exact().clone(),
                productions,
                entry_keywords,
                checked,
            });
        }
        check_collisions(&held, core)?;
        self.check_acyclic(&definitions)?;
        Ok(HeldExtensions {
            held: held
                .into_iter()
                .map(|extension| (extension.definition.clone(), extension))
                .collect(),
        })
    }

    /// FR-111 resolution of the selections and their dependency closure:
    /// two selections of one identity under different authorities refuse
    /// `conflicting-authority` (among the roots, then anywhere in the
    /// closure); a root absent from the catalog refuses
    /// `unsupported-selection`; a dependency edge to an absent definition
    /// refuses `missing-selection`. After this, every edge the cycle walk
    /// follows resolves. Returns the distinct selected definitions in
    /// selection order.
    fn resolve_selected(
        &self,
        selected: &[DefinitionRef],
    ) -> Result<Vec<&Definition>, ExtensionRefusal> {
        let mut by_identity: BTreeMap<&str, &DefinitionRef> = BTreeMap::new();
        for reference in selected {
            if let Some(first) = by_identity.insert(reference.identity(), reference) {
                if first != reference {
                    return Err(ExtensionRefusal::ConflictingAuthority {
                        first: first.clone(),
                        second: reference.clone(),
                    });
                }
            }
        }
        let mut definitions: Vec<&Definition> = Vec::new();
        let mut seen = BTreeSet::new();
        for reference in selected {
            if !seen.insert(reference) {
                continue;
            }
            let definition = self
                .resolve(reference)
                .ok_or_else(|| ExtensionRefusal::UnsupportedSelection(reference.clone()))?;
            definitions.push(definition);
        }
        let mut closure: BTreeMap<&str, &DefinitionRef> = by_identity;
        let mut visited: BTreeSet<&DefinitionRef> = BTreeSet::new();
        let mut work: Vec<&Definition> = definitions.clone();
        while let Some(definition) = work.pop() {
            if !visited.insert(definition.exact()) {
                continue;
            }
            for dependency in definition.dependencies() {
                if let Some(first) = closure.insert(dependency.identity(), dependency) {
                    if first != dependency {
                        return Err(ExtensionRefusal::ConflictingAuthority {
                            first: first.clone(),
                            second: dependency.clone(),
                        });
                    }
                }
                let resolved = self.resolve(dependency).ok_or_else(|| {
                    ExtensionRefusal::MissingDependency {
                        dependent: definition.exact().clone(),
                        dependency: dependency.clone(),
                    }
                })?;
                work.push(resolved);
            }
        }
        Ok(definitions)
    }

    /// Rule 3: the dependency edges of the selected definitions contain no
    /// cycle. A depth-first walk over an explicit stack; the cycle is named
    /// in path order, first and last equal.
    fn check_acyclic(&self, selected: &[&Definition]) -> Result<(), ExtensionRefusal> {
        let mut done = BTreeSet::new();
        for root in selected {
            let mut active: Vec<&DefinitionRef> = Vec::new();
            let mut work = vec![(root.exact(), false)];
            while let Some((reference, expanded)) = work.pop() {
                if expanded {
                    active.pop();
                    done.insert(reference);
                    continue;
                }
                if done.contains(reference) {
                    continue;
                }
                if let Some(start) = active.iter().position(|held| *held == reference) {
                    return Err(ExtensionRefusal::DefinitionCycle(
                        active[start..]
                            .iter()
                            .map(|held| (*held).clone())
                            .chain([reference.clone()])
                            .collect(),
                    ));
                }
                // `resolve_selected` already refused any absent definition.
                let Some(definition) = self.resolve(reference) else {
                    continue;
                };
                active.push(reference);
                work.push((reference, true));
                for dependency in definition.dependencies().iter().rev() {
                    work.push((dependency, false));
                }
            }
        }
        Ok(())
    }
}

/// Rule 2 over every held definition, in selection order: each production's
/// keyword and each node entry's keyword and node kind against the core
/// vocabulary, the same definition's earlier names, then other definitions'.
fn check_collisions(held: &[HeldExtension], core: &CoreVocabulary) -> Result<(), ExtensionRefusal> {
    let mut keywords: BTreeMap<&str, &DefinitionRef> = BTreeMap::new();
    let mut node_kinds: BTreeMap<&str, &DefinitionRef> = BTreeMap::new();
    for extension in held {
        let conflict = |kind, name: &str, with| ExtensionRefusal::ConflictingDefinition {
            definition: extension.definition.clone(),
            kind,
            name: name.to_owned(),
            with,
        };
        let mut own_keywords: BTreeMap<&str, usize> = BTreeMap::new();
        let mut own_kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, production) in extension.productions.iter().enumerate() {
            let name = production.keyword.as_str();
            if core.keywords.contains(name) {
                return Err(conflict(
                    ConflictKind::Keyword,
                    name,
                    ConflictWith::CoreGrammar,
                ));
            }
            if let Some(first) = own_keywords.insert(name, index) {
                return Err(conflict(
                    ConflictKind::Keyword,
                    name,
                    ConflictWith::SameDefinition {
                        first,
                        second: index,
                    },
                ));
            }
            if let Some(other) = keywords.get(name) {
                return Err(conflict(
                    ConflictKind::Keyword,
                    name,
                    ConflictWith::Definition((*other).clone()),
                ));
            }
        }
        let mut own_entries: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, keyword) in extension.entry_keywords.iter().enumerate() {
            if let Some(first) = own_entries.insert(keyword.as_str(), index) {
                return Err(conflict(
                    ConflictKind::EntryKeyword,
                    keyword,
                    ConflictWith::SameDefinition {
                        first,
                        second: index,
                    },
                ));
            }
        }
        for (index, production) in extension.productions.iter().enumerate() {
            let name = production.node_kind.as_str();
            if core.node_kinds.contains(name) {
                return Err(conflict(
                    ConflictKind::NodeKind,
                    name,
                    ConflictWith::CoreGrammar,
                ));
            }
            if let Some(first) = own_kinds.insert(name, index) {
                return Err(conflict(
                    ConflictKind::NodeKind,
                    name,
                    ConflictWith::SameDefinition {
                        first,
                        second: index,
                    },
                ));
            }
            if let Some(other) = node_kinds.get(name) {
                return Err(conflict(
                    ConflictKind::NodeKind,
                    name,
                    ConflictWith::Definition((*other).clone()),
                ));
            }
        }
        for production in &extension.productions {
            keywords.insert(production.keyword.as_str(), &extension.definition);
            node_kinds.insert(production.node_kind.as_str(), &extension.definition);
        }
    }
    Ok(())
}

/// The two exact FR-467 members of `definition`'s bytes, deserialized with
/// unknown members refused.
fn strict_definition(
    definition: &Definition,
) -> Result<CheckedExtensionDefinition, serde_json::Error> {
    let root = root(definition).unwrap_or(Value::Null);
    let members = serde_json::json!({
        "grammar_schema": root.get("grammar_schema"),
        "typed_node_schema": root.get("typed_node_schema"),
    });
    serde_json::from_value(members)
}

fn root(definition: &Definition) -> Option<Value> {
    serde_json::from_slice(definition.exact_bytes()).ok()
}

/// The array at `path` under `root`, empty when absent or not an array.
fn array_at<'a>(root: &'a Value, path: &[&str]) -> &'a [Value] {
    path.iter()
        .try_fold(root, |value, key| value.get(*key))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn string_member(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Rule 1: read the productions joined to their node entries and the
/// entries' keywords, or the first missing member. Every node entry is
/// checked, so a duplicate entry (a rule 2 collision) that also lacks a
/// member is a rule 1 refusal.
fn read_productions(
    definition: &Definition,
) -> Result<(Vec<Production>, Vec<String>), MissingMember> {
    let root = root(definition).ok_or(MissingMember::GrammarSchema)?;
    let grammar = root
        .get("grammar_schema")
        .ok_or(MissingMember::GrammarSchema)?;
    let typed = root
        .get("typed_node_schema")
        .ok_or(MissingMember::TypedNodeSchema)?;
    let raw_productions = array_at(grammar, &["productions"]);
    if raw_productions.is_empty() {
        return Err(MissingMember::Production);
    }
    let entries = array_at(typed, &["nodes"]);

    let mut productions = Vec::with_capacity(raw_productions.len());
    for (index, raw) in raw_productions.iter().enumerate() {
        let keyword =
            string_member(raw, "keyword").ok_or(MissingMember::ProductionKeyword { index })?;
        let mut names = Vec::new();
        for (clause_index, name) in array_at(raw, &["clauses"]).iter().enumerate() {
            let name = name
                .as_str()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| MissingMember::ProductionClauseName {
                    keyword: keyword.clone(),
                    index: clause_index,
                })?;
            names.push(name.to_owned());
        }
        let mut matching = entries
            .iter()
            .filter(|entry| entry.get("keyword").and_then(Value::as_str) == Some(&keyword));
        let first = matching.next().ok_or_else(|| MissingMember::NodeEntry {
            keyword: keyword.clone(),
        })?;
        let (node_kind, clauses) = read_entry(&keyword, &names, first)?;
        for duplicate in matching {
            read_entry(&keyword, &names, duplicate)?;
        }
        productions.push(Production {
            keyword,
            node_kind,
            clauses,
        });
    }
    let mut entry_keywords = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let keyword = entry
            .get("keyword")
            .and_then(Value::as_str)
            .filter(|keyword| !keyword.is_empty())
            .ok_or(MissingMember::EntryKeyword { index })?;
        if !productions
            .iter()
            .any(|production| production.keyword == keyword)
        {
            return Err(MissingMember::ProductionForEntry {
                keyword: keyword.to_owned(),
            });
        }
        entry_keywords.push(keyword.to_owned());
    }
    Ok((productions, entry_keywords))
}

/// Rule 1 over one node entry of the production `keyword` listing `names`:
/// its node kind and its clauses in the production's order.
fn read_entry(
    keyword: &str,
    names: &[String],
    entry: &Value,
) -> Result<(String, Vec<Clause>), MissingMember> {
    let node_kind = string_member(entry, "node_kind").ok_or_else(|| MissingMember::NodeKind {
        keyword: keyword.to_owned(),
    })?;
    let entry_clauses = array_at(entry, &["clauses"]);
    let mut clauses = Vec::with_capacity(names.len());
    for name in names {
        let clause_entry = entry_clauses
            .iter()
            .find(|clause| clause.get("name").and_then(Value::as_str) == Some(name))
            .ok_or_else(|| MissingMember::ClauseEntry {
                keyword: keyword.to_owned(),
                clause: name.clone(),
            })?;
        let type_ref =
            string_member(clause_entry, "type").ok_or_else(|| MissingMember::ClauseType {
                keyword: keyword.to_owned(),
                clause: name.clone(),
            })?;
        clauses.push(Clause {
            name: name.clone(),
            type_ref,
        });
    }
    for (clause_index, clause) in entry_clauses.iter().enumerate() {
        let name = string_member(clause, "name").ok_or_else(|| MissingMember::EntryClauseName {
            keyword: keyword.to_owned(),
            index: clause_index,
        })?;
        if !names.contains(&name) {
            return Err(MissingMember::UnlistedClause {
                keyword: keyword.to_owned(),
                clause: name,
            });
        }
    }
    Ok((node_kind, clauses))
}
