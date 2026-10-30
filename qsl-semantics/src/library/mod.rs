// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer-3 `library`: FR-307 reusable semantic libraries
//! (qualified imports bound to a library `package_id`, transitive closure
//! into a lock with one selection per library identity, diamond
//! unification, cycle refusal and identity-preserving migration).
//!
//! A package is identified by its [`LibraryName`] alone; there is no digest
//! of its content. [`LibraryPackage`] carries the identity preimage bytes
//! as produced by the CheckedPackage V2 writer, and resolution validates
//! that preimage structurally before any import is followed. Each export
//! node key is the structural `node_id` (`{package, path, ordinal}`) of the
//! `identity_projection` node whose `declaration.qualified_name` spells the
//! exported name; on a nominal node that name must equal its nominal
//! `qualified_declaration`, else the package is refused as
//! `declaration-nominal-mismatch`. An export no node's `declaration` spells
//! is `missing_declaration` with cause `undeclared-export`, never a guessed
//! node. A package's local declarations are exactly its exports.
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
use quire_exact::NodeKey;

pub mod bundle;
#[cfg(test)]
mod bundle_tests;
mod package_identity;

use package_identity::{project_declarations, ProjectedDeclarations};
pub use package_identity::{NodeDefect, PreimageDefect};

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
    /// The RFC 8785 JCS bytes of its `quire.checked-package-id/v2` identity
    /// preimage.
    pub identity_preimage: Box<[u8]>,
    /// Imports in declaration order.
    pub imports: Vec<ImportDeclaration>,
    /// Exported qualified declarations, spelled with `::`. Each node key is
    /// derived from the identity preimage.
    pub exports: Vec<String>,
}

/// A dependency path of library identities, importer first.
pub type ImportPath = Vec<LibraryName>;

/// The closed FR-272 cause of a library or name refusal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LibraryCause {
    /// No supplied library of the imported identity has the imported
    /// `package_id`.
    ContentMismatch,
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
            Self::ContentMismatch => "content-mismatch",
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

/// The member path of a structurally malformed identity preimage.
pub const IDENTITY_PREIMAGE_PATH: &str = "/identity_preimage";

/// Why a package's import closure is refused.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum LibraryRefusal {
    /// A package's identity preimage is structurally malformed, refused at
    /// [`IDENTITY_PREIMAGE_PATH`] before resolution.
    #[error("malformed identity preimage")]
    InvalidPreimage {
        /// The package's identity.
        library: LibraryName,
        /// What is malformed.
        defect: PreimageDefect,
    },
    /// An export is not spelled by a nominal `qualified_declaration` in the
    /// package's identity projection, so no explicit declaration names its
    /// node.
    #[error("export names no declaration")]
    UndeclaredExport {
        /// The package's identity.
        library: LibraryName,
        /// The exported name.
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
            Self::InvalidPreimage {
                defect: PreimageDefect::AmbiguousDeclaration { .. },
                ..
            } => Code::AmbiguousDeclaration,
            Self::InvalidPreimage {
                defect: PreimageDefect::AllocationFailed { .. },
                ..
            } => Code::ResourceExhausted,
            Self::InvalidPreimage {
                defect: PreimageDefect::DeclarationNominalMismatch { .. },
                ..
            }
            | Self::InvalidPreimage { .. }
            | Self::InvalidQualifier { .. }
            | Self::ConflictingDefinition { .. }
            | Self::ImportCycle { .. } => Code::InvalidPackage,
            Self::UndeclaredExport { .. } => Code::MissingDeclaration,
        }
    }

    /// The FR-272 cause.
    pub fn cause(&self) -> LibraryCause {
        match self {
            Self::InvalidPreimage {
                defect: PreimageDefect::AmbiguousDeclaration { .. },
                ..
            } => LibraryCause::AmbiguousName,
            Self::InvalidPreimage {
                defect: PreimageDefect::DeclarationNominalMismatch { .. },
                ..
            } => LibraryCause::DeclarationNominalMismatch,
            Self::InvalidPreimage {
                defect: PreimageDefect::AllocationFailed { .. },
                ..
            } => LibraryCause::AllocationFailed,
            Self::InvalidPreimage { .. } | Self::InvalidQualifier { .. } => {
                LibraryCause::InvalidValue
            }
            Self::UndeclaredExport { .. } => LibraryCause::UndeclaredExport,
            Self::ConflictingDefinition { .. } => LibraryCause::ConflictingDefinition,
            Self::ImportCycle { .. } => LibraryCause::DefinitionCycle,
            Self::MissingImport { .. } => LibraryCause::MissingSelection,
        }
    }

    /// The refused member path, for a refusal located at one.
    pub fn member_path(&self) -> Option<&'static str> {
        match self {
            Self::InvalidPreimage { .. } => Some(IDENTITY_PREIMAGE_PATH),
            Self::InvalidQualifier { .. }
            | Self::UndeclaredExport { .. }
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
            Self::InvalidPreimage { .. } | Self::UndeclaredExport { .. } => {
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

/// Validate `package`'s identity preimage and require every name in
/// `package.exports` to be one the preimage declares. Returns the
/// declarations `package.exports` names, keyed by structural `NodeKey`;
/// both steps test name membership and neither maps a name to a node id.
pub(crate) fn verify_package(
    package: &LibraryPackage,
) -> Result<ProjectedDeclarations, LibraryRefusal> {
    let mut declarations = project_declarations(&package.identity_preimage).map_err(|defect| {
        LibraryRefusal::InvalidPreimage {
            library: package.library.clone(),
            defect,
        }
    })?;
    if let Some(export) = declarations.undeclared(&package.exports) {
        return Err(LibraryRefusal::UndeclaredExport {
            library: package.library.clone(),
            export: export.to_owned(),
        });
    }
    declarations.retain_exported(&package.exports);
    Ok(declarations)
}

/// A package's exported declarations, exposed as data (FR-087-AC-4;
/// ADR-011 §2.1 I2): each structural `NodeKey` carries its exported
/// qualified name. The importing package's own check stage builds its name
/// index from these entries. `library` performs no name resolution over this
/// data (ADR-013 R-06): `ImportView` has no method that takes a name.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportView {
    exports: BTreeMap<NodeKey, String>,
}

impl ImportView {
    /// The import view of `package`: the declarations its `exports` list
    /// names, after validating its identity preimage.
    ///
    /// # Errors
    ///
    /// [`LibraryRefusal::InvalidPreimage`] or
    /// [`LibraryRefusal::UndeclaredExport`].
    pub fn of(package: &LibraryPackage) -> Result<Self, LibraryRefusal> {
        Ok(Self {
            exports: verify_package(package)?.into_entries(),
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

/// Every name `identity_preimage` declares, in ascending order (FR-307: "a
/// package's local declarations are exactly its exports").
pub fn declared_exports(identity_preimage: &[u8]) -> Result<Vec<String>, PreimageDefect> {
    Ok(project_declarations(identity_preimage)?
        .declared_names()
        .into_iter()
        .map(str::to_owned)
        .collect())
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
/// library's identity preimage.
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
        // An import selects by identity alone, from the one supplied package
        // of that identity. Its content is bound by its `package_id` in the
        // lock.
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
    /// The successor's identity preimage is unchanged, so it is the identical
    /// package rather than a migration.
    Unchanged,
    /// A new package with a changed identity preimage.
    Migrated {
        /// The migrated package's identity.
        from: LibraryName,
        /// The successor's identity.
        to: LibraryName,
    },
}

/// Check `to` as the successor of `from`: both identity preimages are
/// validated, and equal preimages are the identical package.
pub fn check_migration(
    from: &LibraryPackage,
    to: &LibraryPackage,
) -> Result<LibraryMigration, LibraryRefusal> {
    verify_package(from)?;
    verify_package(to)?;
    if from.identity_preimage == to.identity_preimage {
        return Ok(LibraryMigration::Unchanged);
    }
    Ok(LibraryMigration::Migrated {
        from: from.library.clone(),
        to: to.library.clone(),
    })
}
