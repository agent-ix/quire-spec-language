// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer-3 `library`: FR-307 reusable semantic libraries
//! (qualified imports bound to a library `package_id`, transitive closure
//! into a lock with one selection per library identity, diamond
//! unification, cycle refusal and identity-preserving migration), and
//! [`WireNodeId`] (ADR-013 O-04, T-3).
//!
//! A package's `quire.package.semantic/v2` `package_id` is the SHA-256 of the
//! RFC 8785 JCS bytes of its `quire.checked-package-id/v2` identity preimage.
//! [`LibraryPackage`] carries those bytes as produced by the CheckedPackage V2
//! writer. Resolution recomputes every `package_id` from them and validates the
//! preimage structurally before any import is followed. Each export node key is
//! the `node_id` of the `identity_projection` node whose `declaration
//! .qualified_name` spells the exported name; on a nominal node that name must
//! equal its nominal `qualified_declaration`, else the package is refused as
//! `declaration-nominal-mismatch`. QSpec 82f84d3 gives no other node an
//! explicit declared name, so an export no node's `declaration` spells is
//! `missing_declaration` with cause `undeclared-export`, never a guessed node.
//! A package's local declarations are exactly its exports.
//!
//! ## FR-087 relocation (#213 S-3a, owner ruling on QSL-158, 2026-09-21)
//!
//! Relocated from `value::library` and `value::package_identity` (ADR-011's
//! own module-move table, `:744`), which no longer exist. `LibraryLock` and
//! `resolve_libraries` carry forward unchanged in shape (owner ruling item
//! 3(a)); `PackageId`, `ImportDeclaration`, `LibraryPackage` and the rest of
//! `value::library`'s public surface likewise. Two names do not carry
//! forward in their pre-move shape:
//!
//! - `ExportIdentity{package, node: NodeKey}` is gone, not renamed: T-3's
//!   `PackageNodeKey{package: package_id, node: WireNodeId}` is the module's
//!   sole cross-package node reference after relocation (owner ruling item
//!   3(c)). `PackageNodeKey` and `ImportView` are the typestate lane's own
//!   PR against this same ticket (ADR-013 T-1's shells); this module defines
//!   neither, only the [`WireNodeId`] field type both depend on.
//! - `resolve_name` does not relocate (owner ruling item 3(b)): name
//!   resolution against an imported dependency's exports is E3's own
//!   resolution over an `ImportView`, performed by the importing package's
//!   own check stage, never by `library` calling back into itself. Removed,
//!   not deprecated or aliased (no migration/fallback layer for prerelease
//!   software):
//!
//! ```compile_fail,E0432
//! use quire_spec_language::library::resolve_name;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::library::ExportIdentity;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::library::NameReference;
//! ```
//! ```compile_fail,E0432
//! use quire_spec_language::library::NameRefusal;
//! ```

use std::collections::BTreeMap;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::diagnostic::Code;
use crate::value::node::is_qualified_name;

mod package_identity;

pub use package_identity::{NodeDefect, PreimageDefect};
use package_identity::{project_declarations, ProjectedDeclarations};

/// A node id exactly as it travels on the wire (a v2 node key's 64
/// lowercase-hex digest), before a checked-package lookup resolves it to a
/// `quire_exact::NodeKey` (ADR-013 O-04). Its canonical home (FR-087, #213
/// S-3a): `T-3`'s `PackageNodeKey{package: package_id, node: WireNodeId}`
/// pins it here, and `replay`'s #231 envelopes re-export this same type
/// rather than defining a second one.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WireNodeId([u8; 32]);

impl WireNodeId {
    /// Wrap an already-known wire node-id digest. Unlike
    /// `quire_exact::NodeKey::from_digest`, this constructor carries no
    /// "only `check` calls this" restriction: a `WireNodeId` is exactly the
    /// unchecked wire spelling, never a claim that the id resolves to a
    /// real node.
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// Parse 64 lowercase hexadecimal digits, exactly as a wire node id
    /// travels (`{domain, digest}`'s `digest` member).
    pub fn from_hex(digest: &str) -> Option<Self> {
        let (pairs, []) = digest.as_bytes().as_chunks::<2>() else {
            return None;
        };
        if pairs.len() != 32 {
            return None;
        }
        let mut key = [0_u8; 32];
        for (slot, [high, low]) in key.iter_mut().zip(pairs) {
            *slot = (lower_hex(*high)? << 4) | lower_hex(*low)?;
        }
        Some(Self(key))
    }

    /// The raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

fn lower_hex(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

impl fmt::Display for WireNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for WireNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WireNodeId({self})")
    }
}

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
///
/// ADR-013 O-02: it is computed from a `CheckedPackage` and never accepted
/// from a caller. The field is private for exactly that invariant --
/// [`PackageId::of_preimage`] is production code's only constructor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageId([u8; 32]);

impl PackageId {
    /// The `package_id` of identity preimage `preimage`: SHA-256 of its exact
    /// RFC 8785 JCS bytes.
    pub fn of_preimage(preimage: &[u8]) -> Self {
        Self(Sha256::digest(preimage).into())
    }
}

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

/// A checked library or importing package.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LibraryPackage {
    /// Library identity.
    pub library: LibraryName,
    /// Version string.
    pub version: String,
    /// Its claimed `package_id`.
    pub package_id: PackageId,
    /// The RFC 8785 JCS bytes of its `quire.checked-package-id/v2` identity
    /// preimage, from which `package_id` is recomputed.
    pub identity_preimage: Box<[u8]>,
    /// Imports in declaration order.
    pub imports: Vec<ImportDeclaration>,
    /// Exported qualified declarations, spelled with `::`. Each node key is
    /// derived from the identity preimage.
    pub exports: Vec<String>,
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

/// The closed FR-272 cause of a library or name refusal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LibraryCause {
    /// A supplied library has the imported `package_id` but another version.
    RevisionMismatch,
    /// No supplied library of the imported identity has the imported
    /// `package_id`.
    ByteDigestMismatch,
    /// No library of the imported identity is supplied.
    MissingSelection,
    /// A name has no declaration in scope.
    MissingName,
    /// Two imports bind one qualifier.
    AmbiguousName,
    /// Two dependency paths reach one library with different selections.
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
}

impl LibraryCause {
    /// The cause tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RevisionMismatch => "revision-mismatch",
            Self::ByteDigestMismatch => "byte-digest-mismatch",
            Self::MissingSelection => "missing-selection",
            Self::MissingName => "missing-name",
            Self::AmbiguousName => "ambiguous-name",
            Self::ConflictingDefinition => "conflicting-definition",
            Self::DefinitionCycle => "definition-cycle",
            Self::InvalidValue => "invalid-value",
            Self::UndeclaredExport => "undeclared-export",
            Self::DeclarationNominalMismatch => "declaration-nominal-mismatch",
        }
    }
}

/// How a supplied library differs from its import.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StaleCause {
    /// The `package_id` matches and the version differs.
    RevisionMismatch,
    /// The `package_id` differs.
    ByteDigestMismatch,
}

/// The member path of a refused `package_id`.
pub const PACKAGE_ID_PATH: &str = "/package_id";
/// The member path of a structurally malformed identity preimage.
pub const IDENTITY_PREIMAGE_PATH: &str = "/identity_preimage";

/// Why a package's import closure is refused.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum LibraryRefusal {
    /// A package's `package_id` is not the digest of its identity preimage,
    /// refused at [`PACKAGE_ID_PATH`] before resolution.
    #[error("package_id differs from its identity preimage digest")]
    PackageIdMismatch {
        /// The package's identity.
        library: LibraryName,
        /// Its claimed `package_id`.
        claimed: PackageId,
        /// The recomputed `package_id`.
        recomputed: PackageId,
    },
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
        /// The cycle's dependency edges, first and last equal.
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
        /// Which selection differs.
        cause: StaleCause,
    },
    /// No library with the imported identity or `package_id` is supplied.
    #[error("missing import")]
    MissingImport {
        /// Importer path, then the imported library.
        path: ImportPath,
    },
    /// Two different supplied packages share one identity preimage, refused
    /// at [`PACKAGE_ID_PATH`].
    #[error("duplicate package_id")]
    DuplicatePackageId(PackageId),
}

impl LibraryRefusal {
    /// The FR-307 refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::StaleDependency { .. } => Code::StaleDependency,
            Self::MissingImport { .. } => Code::MissingImport,
            Self::InvalidPreimage {
                defect: PreimageDefect::AmbiguousDeclaration { .. },
                ..
            } => Code::AmbiguousDeclaration,
            Self::InvalidPreimage {
                defect: PreimageDefect::DeclarationNominalMismatch { .. },
                ..
            }
            | Self::PackageIdMismatch { .. }
            | Self::InvalidPreimage { .. }
            | Self::DuplicatePackageId(_)
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
            Self::PackageIdMismatch { .. }
            | Self::InvalidPreimage { .. }
            | Self::DuplicatePackageId(_)
            | Self::InvalidQualifier { .. } => LibraryCause::InvalidValue,
            Self::UndeclaredExport { .. } => LibraryCause::UndeclaredExport,
            Self::ConflictingDefinition { .. } => LibraryCause::ConflictingDefinition,
            Self::ImportCycle { .. } => LibraryCause::DefinitionCycle,
            Self::StaleDependency {
                cause: StaleCause::RevisionMismatch,
                ..
            } => LibraryCause::RevisionMismatch,
            Self::StaleDependency {
                cause: StaleCause::ByteDigestMismatch,
                ..
            } => LibraryCause::ByteDigestMismatch,
            Self::MissingImport { .. } => LibraryCause::MissingSelection,
        }
    }

    /// The refused member path, for a refusal located at one.
    pub fn member_path(&self) -> Option<&'static str> {
        match self {
            Self::PackageIdMismatch { .. } | Self::DuplicatePackageId(_) => Some(PACKAGE_ID_PATH),
            Self::InvalidPreimage { .. } => Some(IDENTITY_PREIMAGE_PATH),
            Self::InvalidQualifier { .. }
            | Self::UndeclaredExport { .. }
            | Self::ConflictingDefinition { .. }
            | Self::ImportCycle { .. }
            | Self::StaleDependency { .. }
            | Self::MissingImport { .. } => None,
        }
    }
}

/// Recompute `package`'s `package_id` from its identity preimage, then
/// validate the preimage and derive the package's export node keys from it.
fn verify_package(package: &LibraryPackage) -> Result<ProjectedDeclarations, LibraryRefusal> {
    let recomputed = PackageId::of_preimage(&package.identity_preimage);
    if recomputed != package.package_id {
        return Err(LibraryRefusal::PackageIdMismatch {
            library: package.library.clone(),
            claimed: package.package_id,
            recomputed,
        });
    }
    project_declarations(&package.identity_preimage)
        .map_err(|defect| LibraryRefusal::InvalidPreimage {
            library: package.library.clone(),
            defect,
        })?
        .select(&package.exports)
        .map_err(|export| LibraryRefusal::UndeclaredExport {
            library: package.library.clone(),
            export: export.to_owned(),
        })
}

/// One selected package with its derived export node keys and the first
/// dependency path that reached it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Selected {
    package: LibraryPackage,
    exports: ProjectedDeclarations,
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
/// in declaration order, after recomputing the root's and every supplied
/// library's `package_id`.
pub fn resolve_libraries(
    root: &LibraryPackage,
    supplied: &[LibraryPackage],
) -> Result<LibraryLock, LibraryRefusal> {
    let root_exports = verify_package(root)?;
    let mut by_id: BTreeMap<PackageId, (&LibraryPackage, ProjectedDeclarations)> = BTreeMap::new();
    for package in supplied {
        let exports = verify_package(package)?;
        match by_id.get(&package.package_id) {
            Some((existing, _)) if *existing != package => {
                return Err(LibraryRefusal::DuplicatePackageId(package.package_id));
            }
            Some(_) => {}
            None => {
                by_id.insert(package.package_id, (package, exports));
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
        if let Some(existing) = selected.get(&import.library) {
            if existing.package.version == import.version
                && existing.package.package_id == import.package_id
            {
                continue;
            }
            return Err(LibraryRefusal::ConflictingDefinition {
                library: import.library.clone(),
                paths: [existing.path.clone(), path],
            });
        }
        let found = by_id.get(&import.package_id).filter(|(package, _)| {
            package.library == import.library && package.version == import.version
        });
        let Some((package, exports)) = found else {
            let same_id = by_id
                .get(&import.package_id)
                .is_some_and(|(package, _)| package.library == import.library);
            let same_identity = supplied
                .iter()
                .any(|package| package.library == import.library);
            let cause = if same_id {
                StaleCause::RevisionMismatch
            } else if same_identity || by_id.contains_key(&import.package_id) {
                StaleCause::ByteDigestMismatch
            } else {
                return Err(LibraryRefusal::MissingImport { path });
            };
            return Err(LibraryRefusal::StaleDependency {
                path,
                import: import.clone(),
                cause,
            });
        };
        selected.insert(
            import.library.clone(),
            Selected {
                package: (*package).clone(),
                exports: exports.clone(),
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
            exports: root_exports,
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

    /// One selection per library identity, in ascending identity order.
    pub fn selections(&self) -> Vec<(LibraryName, Selection)> {
        self.selected
            .iter()
            .map(|(library, selected)| {
                (
                    library.clone(),
                    Selection {
                        version: selected.package.version.clone(),
                        package_id: selected.package.package_id,
                    },
                )
            })
            .collect()
    }
}

/// A checked migration from one package to its successor. Evidence keyed by
/// either identity keeps its key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LibraryMigration {
    /// The successor's identity preimage is unchanged, so it is the identical
    /// package and identity rather than a migration.
    Unchanged,
    /// A new package with a new `package_id`.
    Migrated {
        /// The migrated package's identity, unchanged.
        from: (LibraryName, PackageId),
        /// The successor's identity.
        to: (LibraryName, PackageId),
    },
}

/// Check `to` as the successor of `from`, recomputing both `package_id`s. A
/// successor that reuses its source's `package_id` with a different preimage
/// is refused at `/package_id`.
pub fn check_migration(
    from: &LibraryPackage,
    to: &LibraryPackage,
) -> Result<LibraryMigration, LibraryRefusal> {
    verify_package(from)?;
    verify_package(to)?;
    if from.package_id == to.package_id {
        return Ok(LibraryMigration::Unchanged);
    }
    Ok(LibraryMigration::Migrated {
        from: (from.library.clone(), from.package_id),
        to: (to.library.clone(), to.package_id),
    })
}
