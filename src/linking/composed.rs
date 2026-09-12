// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: closed source inventory, native names and declaration dependencies.
//! This stage precedes definition, model, lexical/type and family admission.
//! Its output cannot construct a historical LinkedPackage or CheckedPackage.

mod arena;
pub mod binding;
pub mod binding_work;
pub mod definition_source;
pub mod definitions;
mod dependencies;
mod inventory;
pub mod models;
pub mod producer;
pub mod requests;
pub mod scopes;
pub mod subject;
mod work;

use std::collections::BTreeMap;

use crate::syntax::composed::{ComposedUnit, Declaration, NativeUnit};
use crate::{ByteDigest, Diagnostic, Limits, Source, SourceIdentity, Spanned};

pub use dependencies::{DependencyKind, DependencyReference, DependencyRefusal, DependencySite};
pub use work::{
    Dimension, Exhaustion, Usage, Work, WorkLimits, ACCOUNTING_VERSION, DEFAULT_LIMITS, HARD_LIMITS,
};

/// Explicit editable authority and exact selected source, independent of path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedSource {
    /// Nonempty editable authority label; repeated authorities conflict.
    pub authority: String,
    /// Exact source identity and revision required from the supplied inventory.
    pub identity: SourceIdentity,
    /// SHA-256 of the selected source's original bytes.
    pub digest: ByteDigest,
}

/// The complete selected native source set. Supplied Source objects are matched
/// by exact identity and revision; paths never add units or select an authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceInventory {
    /// Explicit language selection required in every source header.
    pub language: String,
    /// Single edition required in every source header.
    pub edition: String,
    /// Complete expected source set, independent of supplied order.
    pub units: Vec<ExpectedSource>,
}

/// No package name lookup is available while any inventory issue remains.
#[derive(Debug)]
pub enum InventoryIssue {
    /// No source unit was selected.
    EmptyInventory,
    /// This stage accepts only the composed language/edition selection.
    UnsupportedSelection,
    /// An authority, identity or revision label is blank.
    InvalidExpectedSource {
        /// Index into the report's inventory().units.
        expected: usize,
    },
    /// Multiple expected entries select the same identity and revision.
    DuplicateExpectedSource {
        /// All conflicting indices into inventory().units.
        expected: Vec<usize>,
    },
    /// Multiple supplied sources carry the same identity and revision.
    DuplicateSuppliedSource {
        /// All conflicting indices into the report's supplied().
        supplied: Vec<usize>,
    },
    /// The exact selected identity and revision are unavailable.
    MissingSource {
        /// Unmatched index into inventory().units.
        expected: usize,
    },
    /// A supplied source is outside the explicit expected inventory.
    UnexpectedSource {
        /// Unmatched index into supplied().
        supplied: usize,
    },
    /// Exact source labels match, but the selected byte digest does not.
    DigestMismatch {
        /// Selected index into inventory().units.
        expected: usize,
        /// Conflicting index into supplied().
        supplied: usize,
    },
    /// Both actual header token loci are retained, alongside the report's
    /// borrowed inventory selection. The source index addresses supplied().
    HeaderConflict {
        /// Index into supplied() for both header token locations.
        supplied: usize,
        /// Actual authored language selection.
        language: Spanned<String>,
        /// Actual authored edition selection.
        edition: Spanned<String>,
    },
    /// Source recognition failed, including per-unit parser exhaustion.
    ParseFailure {
        /// Index into supplied() for the original diagnostic source.
        supplied: usize,
        /// Unchanged source-bound parser refusal or incomplete result.
        diagnostic: Box<Diagnostic>,
    },
}

/// Parsed evidence retained when complete namespace establishment fails.
#[derive(Debug)]
pub struct ParsedSource {
    /// Index into the report's supplied() source slice.
    pub supplied: usize,
    /// Original located syntax, without namespace admission.
    pub unit: NativeUnit,
}

/// Handles are local to one namespace, never source-expression or runtime IDs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnitId(pub(super) usize);

impl UnitId {
    /// Index into this namespace's units(), not the supplied source slice.
    pub fn index(self) -> usize {
        self.0
    }
}

/// Declaration handle local to one namespace, not a cross-package identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeclarationId(pub(super) usize);

impl DeclarationId {
    /// Index into this namespace's declarations(), not a unit's AST.
    pub fn index(self) -> usize {
        self.0
    }
}

/// The namespace uniqueness rule violated by a shared conflict group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictKind {
    /// Multiple declarations share one package-wide name.
    NativeName,
    /// Multiple source units share one editable authority.
    SourceAuthority,
}

/// Shared conflict storage retains every locus without copying all members
/// into every refused declaration. Expected indices address inventory().units.
#[derive(Debug)]
pub struct Conflict {
    /// The violated uniqueness rule.
    pub kind: ConflictKind,
    /// Every affected declaration, with original loci available through syntax().
    pub declarations: Vec<DeclarationId>,
    /// Conflicting authority selections; empty for native-name conflicts.
    pub expected: Vec<usize>,
}

/// Known reasons this declaration cannot pass the native namespace stage.
#[derive(Debug)]
pub enum DeclarationRefusal {
    /// The declaration shares a package name with other candidates.
    DuplicateName {
        /// Index into this namespace's conflicts().
        group: usize,
    },
    /// Its source shares an editable authority with another selected unit.
    DuplicateAuthority {
        /// Index into this namespace's conflicts().
        group: usize,
    },
    /// A typed native declaration reference could not be resolved safely.
    Dependency(DependencyRefusal),
}

/// One original declaration's namespace record, including refused occurrences.
#[derive(Debug)]
pub struct DeclarationEntry {
    pub(super) unit: UnitId,
    pub(super) declaration: usize,
    pub(super) refusals: Vec<DeclarationRefusal>,
    pub(super) references: Vec<DependencyReference>,
}

impl DeclarationEntry {
    /// Namespace-local handle for the declaring source unit.
    pub fn unit(&self) -> UnitId {
        self.unit
    }

    /// Index into the original unit's declarations(), preserving its AST.
    pub fn declaration_index(&self) -> usize {
        self.declaration
    }

    /// Known causes; an empty slice alone does not establish availability.
    pub fn refusals(&self) -> &[DeclarationRefusal] {
        &self.refusals
    }

    /// Collected native references, including unresolved or wrong-kind targets.
    pub fn references(&self) -> &[DependencyReference] {
        &self.references
    }
}

/// Refusal is distinct from unfinished native dependency work. Available means
/// only that names and native declaration dependencies passed this stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationDisposition {
    /// Names and native dependencies passed; definition/model/type checks remain.
    Available,
    /// At least one namespace or native dependency refusal is known.
    Refused,
    /// Native dependency processing has not completed for this declaration.
    Unfinished,
}

/// Constructor-private source namespace. Unit-local profile/model aliases and
/// all family bodies remain original unresolved syntax for later admission.
#[derive(Debug)]
pub struct SyntaxNamespace {
    pub(super) units: Vec<ComposedUnit>,
    pub(super) declarations: Vec<DeclarationEntry>,
    pub(super) names: BTreeMap<String, Vec<DeclarationId>>,
    supplied: Vec<usize>,
    expected: Vec<usize>,
    conflicts: Vec<Conflict>,
    dependencies_complete: bool,
}

impl SyntaxNamespace {
    /// Original parsed units in supplied source order.
    pub fn units(&self) -> &[ComposedUnit] {
        &self.units
    }

    /// Inspect a unit using a handle from this namespace.
    pub fn unit(&self, id: UnitId) -> Option<&ComposedUnit> {
        self.units.get(id.0)
    }

    /// Every declaration in unit/source order, including refused entries.
    pub fn declarations(&self) -> &[DeclarationEntry] {
        &self.declarations
    }

    /// Inspect a declaration using a handle from this namespace.
    pub fn declaration(&self, id: DeclarationId) -> Option<&DeclarationEntry> {
        self.declarations.get(id.0)
    }

    /// Original declaration AST and token spans for a namespace-local handle.
    pub fn syntax(&self, id: DeclarationId) -> Option<&Declaration> {
        let entry = self.declaration(id)?;
        self.unit(entry.unit)?.declarations().get(entry.declaration)
    }

    /// Every candidate is retained, including ambiguous and refused names.
    pub fn lookup(&self, name: &str) -> &[DeclarationId] {
        self.names.get(name).map_or(&[], Vec::as_slice)
    }

    /// Map a namespace-local unit to the report's supplied() source index.
    pub fn supplied_index(&self, unit: UnitId) -> Option<usize> {
        self.supplied.get(unit.0).copied()
    }

    /// Map a namespace-local unit to inventory().units.
    pub fn expected_index(&self, unit: UnitId) -> Option<usize> {
        self.expected.get(unit.0).copied()
    }

    /// Shared uniqueness conflicts and all their original declaration loci.
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    /// Whether the native dependency pass finished, including any refusals.
    pub fn dependencies_complete(&self) -> bool {
        self.dependencies_complete
    }

    /// Status for this stage only; never a model, type or executable judgment.
    pub fn disposition(&self, id: DeclarationId) -> Option<DeclarationDisposition> {
        let entry = self.declaration(id)?;
        Some(if !entry.refusals.is_empty() {
            DeclarationDisposition::Refused
        } else if self.dependencies_complete {
            DeclarationDisposition::Available
        } else {
            DeclarationDisposition::Unfinished
        })
    }
}

/// Inputs remain borrowed and unchanged; parsed units share original Source
/// storage. A fresh invocation starts fresh counters even after exhaustion.
#[derive(Debug)]
pub struct NamespaceReport<'a> {
    inventory: &'a SourceInventory,
    supplied: &'a [Source],
    parsed: Vec<ParsedSource>,
    namespace: Option<SyntaxNamespace>,
    issues: Vec<InventoryIssue>,
    exhaustion: Option<Exhaustion>,
    usage: Usage,
    limits: WorkLimits,
    parser_limits: Limits,
}

impl<'a> NamespaceReport<'a> {
    /// Borrow the original explicit selections without copying caller labels.
    pub fn inventory(&self) -> &'a SourceInventory {
        self.inventory
    }

    /// Borrow the unchanged supplied sources, including refused inventory entries.
    pub fn supplied(&self) -> &'a [Source] {
        self.supplied
    }

    /// Present only after every expected unit and native declaration name has
    /// been collected. Dependency refusals do not erase the closed namespace.
    pub fn namespace(&self) -> Option<&SyntaxNamespace> {
        self.namespace.as_ref()
    }

    /// Successful parses before namespace failure; empty after establishment,
    /// when the same ASTs instead reside in namespace().units().
    pub fn parsed_sources(&self) -> &[ParsedSource] {
        &self.parsed
    }

    /// Known issues preventing establishment of the complete source namespace.
    pub fn issues(&self) -> &[InventoryIssue] {
        &self.issues
    }

    /// First unaffordable package charge; parser exhaustion stays in issues().
    pub fn exhaustion(&self) -> Option<&Exhaustion> {
        self.exhaustion.as_ref()
    }

    /// Whether package or per-unit limits prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.exhaustion.is_some()
            || self.issues.iter().any(|issue| {
                matches!(issue, InventoryIssue::ParseFailure { diagnostic, .. } if diagnostic.is_incomplete())
            })
    }

    /// Successful package charges from this invocation only.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    /// Effective package capacities after clamping caller limits.
    pub fn limits(&self) -> WorkLimits {
        self.limits
    }

    /// Effective per-unit parser limits, separate from whole-package accounting.
    pub fn parser_limits(&self) -> Limits {
        self.parser_limits
    }
}

/// Establish exact inventoried source names and resolve native declaration
/// dependencies. No definition artifacts, producer models or runtime input are
/// accepted here, and no result is a fully linked or checked package.
pub fn admit_namespace<'a>(
    inventory: &'a SourceInventory,
    supplied: &'a [Source],
    limits: WorkLimits,
    parser_limits: Limits,
) -> NamespaceReport<'a> {
    let mut work = Work::new(limits);
    let mut report = NamespaceReport {
        inventory,
        supplied,
        parsed: Vec::new(),
        namespace: None,
        issues: Vec::new(),
        exhaustion: None,
        usage: Usage::default(),
        limits: work.limits(),
        parser_limits: parser_limits.bounded(),
    };
    if let Err(exhaustion) = establish(&mut report, &mut work) {
        report.exhaustion = Some(exhaustion);
    }
    report.usage = work.usage();
    report
}

fn establish(report: &mut NamespaceReport<'_>, work: &mut Work) -> Result<(), Exhaustion> {
    let Some(expected) = inventory::prepare(report, work)? else {
        return Ok(());
    };
    let namespace = inventory::collect(report, expected, work)?;
    report.namespace = Some(namespace);
    let namespace = report
        .namespace
        .as_mut()
        .expect("namespace just established");
    dependencies::resolve(namespace, work)?;
    namespace.dependencies_complete = true;
    Ok(())
}
