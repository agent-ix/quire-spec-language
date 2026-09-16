// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-307 reusable semantic libraries: qualified imports bound to a library
//! `package_id`, transitive closure into a lock with one selection per library
//! identity, diamond unification, cycle refusal, qualified name resolution and
//! identity-preserving migration.

use std::collections::BTreeMap;

use super::node::{is_qualified_name, NodeKey};
use crate::diagnostic::Code;

// SPEC-GAP(119-21): CheckedPackage V2 and its `quire.package.semantic/v2`
// `package_id` are not available to this value layer. A library package is
// the typed [`LibraryPackage`] below: its `package_id` is supplied (never
// computed or checked against content here), its library package key is that
// `package_id`, its export node keys are supplied opaquely, and its local
// declarations are exactly its exports.

/// A qualified library identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LibraryName(Box<[String]>);

/// A malformed library identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("a library identity is a non-empty sequence of identifiers")]
pub struct InvalidLibraryName;

impl LibraryName {
    /// A library identity from its qualified segments.
    pub fn new(segments: Vec<String>) -> Result<Self, InvalidLibraryName> {
        if is_qualified_name(&segments) {
            Ok(Self(segments.into_boxed_slice()))
        } else {
            Err(InvalidLibraryName)
        }
    }

    /// The qualified segments.
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

/// A `quire.package.semantic/v2` `package_id`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageId(pub [u8; 32]);

/// `import "L" version "v" digest "d" as a;`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ImportDeclaration {
    /// Library identity `L`.
    pub library: LibraryName,
    /// Version string `v`.
    pub version: String,
    /// Digest `d`: the library's `package_id`.
    pub package_id: PackageId,
    /// The `as` qualifier, if written.
    pub qualifier: Option<String>,
}

/// One exported declaration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Export {
    /// Its name.
    pub name: String,
    /// Its node key in the library's graph.
    pub node: NodeKey,
}

/// A checked library or importing package.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LibraryPackage {
    /// Library identity.
    pub library: LibraryName,
    /// Version string.
    pub version: String,
    /// Its `package_id`.
    pub package_id: PackageId,
    /// Imports in declaration order.
    pub imports: Vec<ImportDeclaration>,
    /// Exported declarations.
    pub exports: Vec<Export>,
}

/// An export identity: (library package key, export node key).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExportIdentity {
    /// The library's package key.
    pub package: PackageId,
    /// The export's node key in that library's graph.
    pub node: NodeKey,
}

/// One exact library selection.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Selection {
    /// Version string.
    pub version: String,
    /// `package_id`.
    pub package_id: PackageId,
}

/// A dependency path of library identities, importer first.
pub type ImportPath = Vec<LibraryName>;

/// Why a package's import closure is refused.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum LibraryRefusal {
    /// An import has no `as` qualifier.
    #[error("import without a qualifier")]
    MissingQualifier {
        /// Importer path, then the imported library.
        path: ImportPath,
    },
    /// An `as` qualifier is not an identifier.
    #[error("invalid import qualifier")]
    InvalidQualifier {
        /// Importer path, then the imported library.
        path: ImportPath,
    },
    /// Two dependency paths reach one library identity with a different
    /// version or `package_id`.
    #[error("conflicting library definitions")]
    ConflictingDefinition {
        /// The library identity.
        library: LibraryName,
        /// The earlier and the conflicting dependency path.
        paths: [ImportPath; 2],
    },
    /// The import graph has a cycle.
    #[error("import cycle")]
    ImportCycle {
        /// The cycle, first and last equal.
        cycle: ImportPath,
    },
    /// A supplied library's identity, version or `package_id` differs from
    /// the import.
    #[error("stale dependency")]
    StaleDependency {
        /// Importer path, then the imported library.
        path: ImportPath,
        /// The import.
        import: ImportDeclaration,
    },
    /// No library with the imported identity or `package_id` is supplied.
    #[error("missing import")]
    MissingImport {
        /// Importer path, then the imported library.
        path: ImportPath,
    },
    /// Two different supplied packages claim one `package_id`.
    #[error("duplicate package_id")]
    DuplicatePackageId(PackageId),
}

impl LibraryRefusal {
    /// The FR-307 refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::StaleDependency { .. } => Code::StaleDependency,
            Self::MissingImport { .. } => Code::MissingImport,
            Self::MissingQualifier { .. }
            | Self::InvalidQualifier { .. }
            | Self::ConflictingDefinition { .. }
            | Self::ImportCycle { .. }
            | Self::DuplicatePackageId(_) => Code::InvalidPackage,
        }
    }
}

/// A resolved import closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryLock {
    root: LibraryPackage,
    /// Selected packages by identity, with the first path that reached each.
    selected: BTreeMap<LibraryName, (LibraryPackage, ImportPath)>,
}

struct Pending<'a> {
    package: &'a LibraryPackage,
    next: usize,
    path: ImportPath,
}

/// Resolve the transitive import closure of `root` over `supplied` libraries
/// in declaration order.
pub fn resolve_libraries(
    root: &LibraryPackage,
    supplied: &[LibraryPackage],
) -> Result<LibraryLock, LibraryRefusal> {
    let mut by_id: BTreeMap<PackageId, &LibraryPackage> = BTreeMap::new();
    for package in supplied {
        match by_id.get(&package.package_id) {
            Some(existing) if *existing != package => {
                return Err(LibraryRefusal::DuplicatePackageId(package.package_id));
            }
            Some(_) => {}
            None => {
                by_id.insert(package.package_id, package);
            }
        }
    }
    let mut selected: BTreeMap<LibraryName, (LibraryPackage, ImportPath)> = BTreeMap::new();
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
        match &import.qualifier {
            None => return Err(LibraryRefusal::MissingQualifier { path }),
            Some(qualifier) if !is_qualified_name(std::slice::from_ref(qualifier)) => {
                return Err(LibraryRefusal::InvalidQualifier { path });
            }
            Some(_) => {}
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
        if let Some((existing, first)) = selected.get(&import.library) {
            if existing.version == import.version && existing.package_id == import.package_id {
                continue;
            }
            return Err(LibraryRefusal::ConflictingDefinition {
                library: import.library.clone(),
                paths: [first.clone(), path],
            });
        }
        let found = by_id.get(&import.package_id).copied().filter(|package| {
            package.library == import.library && package.version == import.version
        });
        let Some(package) = found else {
            let stale = by_id.contains_key(&import.package_id)
                || supplied
                    .iter()
                    .any(|package| package.library == import.library);
            return Err(if stale {
                LibraryRefusal::StaleDependency {
                    path,
                    import: import.clone(),
                }
            } else {
                LibraryRefusal::MissingImport { path }
            });
        };
        selected.insert(import.library.clone(), (package.clone(), path.clone()));
        stack.push(Pending {
            package,
            next: 0,
            path,
        });
    }
    Ok(LibraryLock {
        root: root.clone(),
        selected,
    })
}

/// A name use inside a package.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NameReference {
    /// `Name`.
    Unqualified(String),
    /// `a::Name`.
    Qualified {
        /// The import qualifier `a`.
        qualifier: String,
        /// `Name`.
        name: String,
    },
}

/// Why a name use does not resolve.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum NameRefusal {
    /// No local declaration, import qualifier or export matches.
    #[error("missing declaration")]
    MissingDeclaration(NameReference),
    /// Two imports bind the qualifier.
    #[error("ambiguous declaration")]
    AmbiguousDeclaration {
        /// The qualifier.
        qualifier: String,
        /// Every import path binding it, in declaration order.
        paths: Vec<ImportPath>,
    },
}

impl NameRefusal {
    /// The FR-307 refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::MissingDeclaration(_) => Code::MissingDeclaration,
            Self::AmbiguousDeclaration { .. } => Code::AmbiguousDeclaration,
        }
    }
}

impl LibraryLock {
    /// The resolved root package.
    pub fn root(&self) -> &LibraryPackage {
        &self.root
    }

    /// One selection per library identity, in ascending identity order.
    pub fn selections(&self) -> Vec<(LibraryName, Selection)> {
        self.selected
            .iter()
            .map(|(library, (package, _))| {
                (
                    library.clone(),
                    Selection {
                        version: package.version.clone(),
                        package_id: package.package_id,
                    },
                )
            })
            .collect()
    }

    /// Resolve `reference` as used in the package with identity `user` (the
    /// root or a selected library). Unqualified names resolve only to the
    /// package's own declarations.
    pub fn resolve_name(
        &self,
        user: &LibraryName,
        reference: &NameReference,
    ) -> Result<ExportIdentity, NameRefusal> {
        let missing = || NameRefusal::MissingDeclaration(reference.clone());
        let (package, path) = if *user == self.root.library {
            (&self.root, vec![self.root.library.clone()])
        } else {
            let (package, path) = self.selected.get(user).ok_or_else(missing)?;
            (package, path.clone())
        };
        let (owner, name) = match reference {
            NameReference::Unqualified(name) => (package, name),
            NameReference::Qualified { qualifier, name } => {
                let bound: Vec<&ImportDeclaration> = package
                    .imports
                    .iter()
                    .filter(|import| import.qualifier.as_ref() == Some(qualifier))
                    .collect();
                match bound.as_slice() {
                    [] => return Err(missing()),
                    [import] => {
                        let (owner, _) = self.selected.get(&import.library).ok_or_else(missing)?;
                        (owner, name)
                    }
                    [..] => {
                        return Err(NameRefusal::AmbiguousDeclaration {
                            qualifier: qualifier.clone(),
                            paths: bound
                                .iter()
                                .map(|import| {
                                    let mut import_path = path.clone();
                                    import_path.push(import.library.clone());
                                    import_path
                                })
                                .collect(),
                        });
                    }
                }
            }
        };
        owner
            .exports
            .iter()
            .find(|export| export.name == *name)
            .map(|export| ExportIdentity {
                package: owner.package_id,
                node: export.node,
            })
            .ok_or_else(missing)
    }
}

/// Why a migration is refused.
// SPEC-GAP(119-22): FR-307 requires a migrated library or importer to be a new
// package with a new `package_id` but names no refusal for one that reuses
// it. Reuse refuses as `invalid_package`, and no correspondence record is
// produced beyond the two package identities.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum MigrationRefusal {
    /// The successor reuses the migrated `package_id`.
    #[error("migration reuses the package_id")]
    PackageIdReused,
}

impl MigrationRefusal {
    /// The refusal code.
    pub fn code(self) -> Code {
        match self {
            Self::PackageIdReused => Code::InvalidPackage,
        }
    }
}

/// A checked migration from one package to its successor. Evidence keyed by
/// either identity keeps its key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LibraryMigration {
    /// The migrated package's identity, unchanged.
    pub from: (LibraryName, PackageId),
    /// The successor's identity.
    pub to: (LibraryName, PackageId),
}

/// Check that `to` is a new package succeeding `from`.
pub fn check_migration(
    from: &LibraryPackage,
    to: &LibraryPackage,
) -> Result<LibraryMigration, MigrationRefusal> {
    if from.package_id == to.package_id {
        return Err(MigrationRefusal::PackageIdReused);
    }
    Ok(LibraryMigration {
        from: (from.library.clone(), from.package_id),
        to: (to.library.clone(), to.package_id),
    })
}
