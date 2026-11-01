// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer-3 `library`: FR-307 reusable semantic libraries
//! (qualified imports bound by library name, transitive closure into a lock
//! with one selection per library identity, structural diamond conflict,
//! cycle refusal and identity-preserving migration).
//!
//! A package is identified by its [`LibraryName`] alone; there is no digest
//! of its content. [`LibraryPackage`] carries its declared exports as typed
//! `(qualified name, NodeKey)` pairs, each key the structural node id of the
//! declaration the name spells. Resolution validates those pairs before any
//! import is followed. A package's local declarations are exactly its
//! exports.
//!
//! This module owns `LibraryLock`/`resolve_libraries`, the identities
//! (`LibraryName`), declarations (`ImportDeclaration`, `LibraryPackage`),
//! the resolved lock, refusal reporting and [`ImportView`], a package's
//! exported declarations as `(name, NodeKey)` data. Name resolution over an
//! `ImportView` is the importing package's own check stage (FR-087-AC-4;
//! ADR-013 R-06); `library` has no function that takes a name and returns a
//! node key.

use std::collections::BTreeMap;

use crate::value::semantic_node::is_qualified_name;
use qsl_foundation::diagnostic::{Code, ALLOCATION_FAILED};
use qsl_foundation::source::node_table::{scoped, NodeMap};
use quire_exact::NodeKey;

pub mod bundle;
#[cfg(test)]
mod bundle_tests;

/// A library identity (ADR-015 D-3; QSpec FR-307, FR-322
/// `DependencySelection.identity`): any non-empty string, with no segment
/// structure. Two identities are equal only when their UTF-8 bytes are, and
/// the derived order is UTF-8 byte order, the order FR-322 writes
/// `dependency_selections` in. The S1 parser's identity bound is a source
/// limit, not part of the identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LibraryName(Box<str>);

/// An empty library identity, the only one [`LibraryName::new`] refuses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("a library identity is a non-empty string")]
pub struct InvalidLibraryName;

impl LibraryName {
    /// A library identity from its string. Refuses only the empty string.
    pub fn new(identity: impl Into<String>) -> Result<Self, InvalidLibraryName> {
        let identity = identity.into();
        if identity.is_empty() {
            Err(InvalidLibraryName)
        } else {
            Ok(Self(identity.into_boxed_str()))
        }
    }

    /// The identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LibraryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `import "L" as a;`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ImportDeclaration {
    /// Library identity `L`.
    pub library: LibraryName,
    /// The `as` qualifier, if written.
    pub qualifier: Option<String>,
}

/// A checked library or importing package.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LibraryPackage {
    /// Library identity.
    pub library: LibraryName,
    /// Imports in declaration order.
    pub imports: Vec<ImportDeclaration>,
    /// Exported qualified declarations: each name spelled with `::` and the
    /// structural node id of the declaration it names.
    pub exports: Vec<(String, NodeKey)>,
}

/// A dependency path of library identities, importer first.
pub type ImportPath = Vec<LibraryName>;

/// The closed FR-272 cause of a library or name refusal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LibraryCause {
    /// No library of the imported identity is supplied.
    MissingSelection,
    /// Two imports bind one qualifier.
    AmbiguousName,
    /// Two supplied packages carry one library identity.
    ConflictingDefinition,
    /// The import graph has a cycle.
    DefinitionCycle,
    /// A member value is invalid at its member path.
    InvalidValue,
    /// An export name matches no projection node's `declaration
    /// .qualified_name`.
    UndeclaredExport,
    /// A projection node's top-level `declaration.qualified_name` disagrees
    /// with its nominal `qualified_declaration`, or is absent while a
    /// nominal `qualified_declaration` is present.
    DeclarationNominalMismatch,
    /// Reading or encoding the identity preimage could not reserve memory
    /// (FR-259 B6).
    AllocationFailed,
}

impl LibraryCause {
    /// The cause tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingSelection => "missing-selection",
            Self::AmbiguousName => "ambiguous-name",
            Self::ConflictingDefinition => "conflicting-definition",
            Self::DefinitionCycle => "definition-cycle",
            Self::InvalidValue => "invalid-value",
            Self::UndeclaredExport => "undeclared-export",
            Self::DeclarationNominalMismatch => "declaration-nominal-mismatch",
            Self::AllocationFailed => ALLOCATION_FAILED.cause(),
        }
    }
}

/// Why a package's import closure is refused.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum LibraryRefusal {
    /// An export name is not a qualified declaration name.
    #[error("invalid export name")]
    InvalidExportName {
        /// The package's identity.
        library: LibraryName,
        /// The exported name.
        export: String,
    },
    /// Two exports carry one qualified name (`ambiguous-name`).
    #[error("ambiguous export name")]
    AmbiguousDeclaration {
        /// The package's identity.
        library: LibraryName,
        /// The repeated name.
        export: String,
    },
    /// An `as` qualifier is not an identifier.
    #[error("invalid import qualifier")]
    InvalidQualifier {
        /// Importer path, then the imported library.
        path: ImportPath,
    },
    /// Two different supplied packages carry one library identity, so an
    /// import of it selects neither (`invalid_package`/
    /// `conflicting-definition`).
    #[error("conflicting library definitions")]
    ConflictingDefinition {
        /// The library identity supplied twice.
        library: LibraryName,
    },
    /// The import graph has a cycle.
    #[error("import cycle")]
    ImportCycle {
        /// The cycle's dependency edges, first and last equal.
        cycle: ImportPath,
    },
    /// No library with the imported identity is supplied.
    #[error("missing import")]
    MissingImport {
        /// Importer path, then the imported library.
        path: ImportPath,
    },
}

impl LibraryRefusal {
    /// The FR-307 refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::MissingImport { .. } => Code::MissingImport,
            Self::AmbiguousDeclaration { .. } => Code::AmbiguousDeclaration,
            Self::InvalidExportName { .. }
            | Self::InvalidQualifier { .. }
            | Self::ConflictingDefinition { .. }
            | Self::ImportCycle { .. } => Code::InvalidPackage,
        }
    }

    /// The FR-272 cause.
    pub fn cause(&self) -> LibraryCause {
        match self {
            Self::AmbiguousDeclaration { .. } => LibraryCause::AmbiguousName,
            Self::InvalidExportName { .. } | Self::InvalidQualifier { .. } => {
                LibraryCause::InvalidValue
            }
            Self::ConflictingDefinition { .. } => LibraryCause::ConflictingDefinition,
            Self::ImportCycle { .. } => LibraryCause::DefinitionCycle,
            Self::MissingImport { .. } => LibraryCause::MissingSelection,
        }
    }

    /// The refused member path, for a refusal located at one.
    pub fn member_path(&self) -> Option<&'static str> {
        match self {
            Self::InvalidExportName { .. }
            | Self::AmbiguousDeclaration { .. }
            | Self::InvalidQualifier { .. }
            | Self::ConflictingDefinition { .. }
            | Self::ImportCycle { .. }
            | Self::MissingImport { .. } => None,
        }
    }

    /// FR-087-AC-12's classification: every variant classifies to exactly
    /// one of an ADR-011 I2 graph rule, a per-package admission check, or E3
    /// name resolution.
    pub fn class(&self) -> RefusalClass {
        match self {
            // The per-package admission checks `verify_package` raises.
            Self::InvalidExportName { .. } | Self::AmbiguousDeclaration { .. } => {
                RefusalClass::BindingCondition
            }
            // E3 name resolution: moves conceptually with the removed
            // `resolve_name` (owner ruling item 3(b)).
            Self::InvalidQualifier { .. } => RefusalClass::E3NameResolution,
            // ADR-011 `:203-210`'s first I2 rule: a missing or unlisted
            // identity.
            Self::MissingImport { .. } => RefusalClass::I2Rule(1),
            // I2's second rule: two supplied packages claim one library
            // identity, so the identity has no one selection.
            Self::ConflictingDefinition { .. } => RefusalClass::I2Rule(2),
            // I2's third rule: the import graph has a cycle.
            Self::ImportCycle { .. } => RefusalClass::I2Rule(3),
        }
    }
}

/// FR-087-AC-12's classification of a [`LibraryRefusal`]: exactly one of an
/// ADR-011 I2 graph rule, a per-package admission check, or E3 name
/// resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusalClass {
    /// One of ADR-011 `:203-210`'s three I2 graph rules (1, 2 or 3).
    I2Rule(u8),
    /// A per-package admission check.
    BindingCondition,
    /// E3 name resolution (moves conceptually with the removed
    /// `resolve_name`, owner ruling item 3(b)).
    E3NameResolution,
}

/// Validate `package`'s exports: every name a qualified declaration name,
/// and no name exported twice. Returns the exports keyed by structural
/// `NodeKey`.
pub(crate) fn verify_package(
    package: &LibraryPackage,
) -> Result<BTreeMap<NodeKey, String>, LibraryRefusal> {
    let mut names = std::collections::BTreeSet::new();
    let mut exports = BTreeMap::new();
    for (name, key) in &package.exports {
        let segments: Vec<String> = name.split("::").map(str::to_owned).collect();
        if !is_qualified_name(&segments) {
            return Err(LibraryRefusal::InvalidExportName {
                library: package.library.clone(),
                export: name.clone(),
            });
        }
        if !names.insert(name.as_str()) {
            return Err(LibraryRefusal::AmbiguousDeclaration {
                library: package.library.clone(),
                export: name.clone(),
            });
        }
        exports.insert(key.clone(), name.clone());
    }
    Ok(exports)
}

/// A package's exported declarations, exposed as data (FR-087-AC-4;
/// ADR-011 §2.1 I2): each structural `NodeKey` carries its exported
/// qualified name. The importing package's own check stage builds its name
/// index from these entries. `library` performs no name resolution over this
/// data (ADR-013 R-06): `ImportView` has no method that takes a name.
///
/// quire:canonical
#[derive(Clone, Eq, PartialEq)]
pub struct ImportView {
    exports: BTreeMap<NodeKey, String>,
}

/// Debug writes each export's node through [`NodeKeyDebug`]: once in the
/// enclosing [`scoped`] table, so the output stays linear however deeply the
/// exported identities nest.
impl std::fmt::Debug for ImportView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct Body<'a>(&'a ImportView);
        impl std::fmt::Debug for Body<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("ImportView")
                    .field("exports", &NodeMap(self.0.exports.iter()))
                    .finish()
            }
        }
        scoped(f, &Body(self))
    }
}

impl ImportView {
    /// The import view of `package`: its exports, after validating them.
    ///
    /// # Errors
    ///
    /// [`LibraryRefusal::InvalidExportName`] or
    /// [`LibraryRefusal::AmbiguousDeclaration`].
    pub fn of(package: &LibraryPackage) -> Result<Self, LibraryRefusal> {
        Ok(Self {
            exports: verify_package(package)?,
        })
    }

    /// The exported declarations as `(name, NodeKey)` entries, in ascending
    /// node-key order. The importing package's own checker performs the
    /// name lookup itself, over this data (E3; FR-087-AC-4).
    pub fn exports(&self) -> impl Iterator<Item = (&str, &NodeKey)> + '_ {
        self.exports
            .iter()
            .map(|(node, name)| (name.as_str(), node))
    }
}

/// One selected package and the first dependency path that reached it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Selected {
    package: LibraryPackage,
    path: ImportPath,
}

/// A resolved import closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryLock {
    root: Selected,
    /// Selected packages by identity.
    selected: BTreeMap<LibraryName, Selected>,
}

struct Pending<'a> {
    package: &'a LibraryPackage,
    next: usize,
    path: ImportPath,
}

/// Resolve the transitive import closure of `root` over `supplied` libraries
/// in declaration order, after validating the root's and every supplied
/// library's exports.
pub fn resolve_libraries(
    root: &LibraryPackage,
    supplied: &[LibraryPackage],
) -> Result<LibraryLock, LibraryRefusal> {
    verify_package(root)?;
    // An identity is supplied once.
    let mut by_library: BTreeMap<&LibraryName, &LibraryPackage> = BTreeMap::new();
    for package in supplied {
        verify_package(package)?;
        match by_library.get(&package.library) {
            Some(existing) if *existing != package => {
                return Err(LibraryRefusal::ConflictingDefinition {
                    library: package.library.clone(),
                });
            }
            Some(_) => {}
            None => {
                by_library.insert(&package.library, package);
            }
        }
    }
    let mut selected: BTreeMap<LibraryName, Selected> = BTreeMap::new();
    let mut stack = vec![Pending {
        package: root,
        next: 0,
        path: vec![root.library.clone()],
    }];
    while let Some(top) = stack.last_mut() {
        let Some(import) = top.package.imports.get(top.next) else {
            stack.pop();
            continue;
        };
        top.next = top.next.saturating_add(1);
        let mut path = top.path.clone();
        path.push(import.library.clone());
        // An import without `as` still selects and verifies its library.
        if let Some(qualifier) = &import.qualifier {
            if !is_qualified_name(std::slice::from_ref(qualifier)) {
                return Err(LibraryRefusal::InvalidQualifier { path });
            }
        }
        if let Some(start) = top
            .path
            .iter()
            .position(|library| *library == import.library)
        {
            return Err(LibraryRefusal::ImportCycle {
                cycle: path.split_off(start),
            });
        }
        if selected.contains_key(&import.library) {
            continue;
        }
        // An import selects the one supplied package by library identity.
        let Some(&package) = by_library.get(&import.library) else {
            return Err(LibraryRefusal::MissingImport { path });
        };
        selected.insert(
            import.library.clone(),
            Selected {
                package: package.clone(),
                path: path.clone(),
            },
        );
        stack.push(Pending {
            package,
            next: 0,
            path,
        });
    }
    Ok(LibraryLock {
        root: Selected {
            package: root.clone(),
            path: vec![root.library.clone()],
        },
        selected,
    })
}

impl LibraryLock {
    /// The resolved root package.
    pub fn root(&self) -> &LibraryPackage {
        &self.root.package
    }

    /// The selected library identities, in ascending identity order.
    pub fn selections(&self) -> Vec<LibraryName> {
        self.selected.keys().cloned().collect()
    }
}

/// A checked migration from one package to its successor.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LibraryMigration {
    /// The successor's exports are unchanged, so it is the identical
    /// package rather than a migration.
    Unchanged,
    /// A new package with changed exports.
    Migrated {
        /// The migrated package's identity.
        from: LibraryName,
        /// The successor's identity.
        to: LibraryName,
    },
}

/// Check `to` as the successor of `from`: both exports are
/// validated, and equal exports are the identical package.
pub fn check_migration(
    from: &LibraryPackage,
    to: &LibraryPackage,
) -> Result<LibraryMigration, LibraryRefusal> {
    let (from_exports, to_exports) = (verify_package(from)?, verify_package(to)?);
    if from_exports == to_exports {
        return Ok(LibraryMigration::Unchanged);
    }
    Ok(LibraryMigration::Migrated {
        from: from.library.clone(),
        to: to.library.clone(),
    })
}

#[cfg(test)]
mod import_view_debug_tests {
    use super::ImportView;
    use quire_exact::{NodeKey, PathSegment};
    use std::collections::BTreeMap;

    /// An import view's debug output writes each nested export identity once
    /// and names an export by position, so it stays linear in the nesting.
    #[test]
    fn import_view_debug_writes_each_node_once() {
        let inner = NodeKey::new("lib", vec![PathSegment::Text("f".into())], 0).unwrap();
        let outer = NodeKey::new(
            "lib",
            vec![
                PathSegment::Text("g".into()),
                PathSegment::Node(inner.clone()),
            ],
            0,
        )
        .unwrap();
        let view = ImportView {
            exports: BTreeMap::from([(inner, "f".to_owned()), (outer, "g".to_owned())]),
        };
        let text = format!("{view:?}");
        assert_eq!(
            text.matches("lib:[").count(),
            2,
            "the inner node is written once, not again inside the outer: {text}"
        );
        assert!(text.contains("\"f\"") && text.contains("\"g\""), "{text}");
    }

    /// Keys retained by timed and protocol records join the package's node
    /// table even when several different field shapes name the same node.
    /// Provenance: QSL-1358, QSL-381; component completeness regression.
    #[test]
    fn retained_timed_and_protocol_keys_share_one_complete_debug_table() {
        use crate::check::{CheckedJoinPolicy, CheckedProtocolNodeKeys, ConstraintTargetNodes};
        use qsl_foundation::source::node_table::scoped;
        use std::fmt;
        struct Records(
            ConstraintTargetNodes,
            CheckedJoinPolicy,
            CheckedProtocolNodeKeys,
        );
        impl fmt::Debug for Records {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                scoped(f, &(&self.0, &self.1, &self.2))
            }
        }
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let mut key =
                    NodeKey::new("retained", vec![PathSegment::Text("leaf".into())], 0).unwrap();
                for ordinal in 0..1_000 {
                    key = NodeKey::new(
                        "retained",
                        vec![PathSegment::Text("operand".into()), PathSegment::Node(key)],
                        ordinal,
                    )
                    .unwrap();
                }
                let records = Records(
                    ConstraintTargetNodes::Parameter {
                        operation: key.clone(),
                        parameter: "clock".into(),
                    },
                    CheckedJoinPolicy::Predicate {
                        function: key.clone(),
                    },
                    CheckedProtocolNodeKeys {
                        clause: Some(key.clone()),
                        compensations: vec![key.clone()],
                        terminal: Some(key),
                        ..Default::default()
                    },
                );
                let text = format!("{records:?}");
                assert_eq!(
                    text.matches("retained:[").count(),
                    1_001,
                    "every distinct node is written once"
                );
                assert_eq!(text.matches("\"leaf\"").count(), 1);
                assert!(text.contains("operation: #0"), "{text}");
                assert!(text.contains("function: #0"), "{text}");
                assert!(text.contains("clause: Some(#0)"), "{text}");
                assert!(text.contains("compensations: [#0]"), "{text}");
                assert!(text.contains("terminal: Some(#0)"), "{text}");
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
