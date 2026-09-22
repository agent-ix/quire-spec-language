// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer-3 `library`: FR-307 reusable semantic libraries
//! (qualified imports bound to a library `package_id`, transitive closure
//! into a lock with one selection per library identity, diamond
//! unification, cycle refusal and identity-preserving migration).
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
//! ## Module boundary (FR-087, ADR-011 §6.1 layer 3)
//!
//! This module owns `LibraryLock`/`resolve_libraries` and the rest of the
//! FR-307 public surface: identities (`PackageId`, `LibraryName`),
//! declarations (`ImportDeclaration`, `LibraryPackage`), the resolved lock
//! (`Selection`, `LibraryLock`) and refusal reporting (`LibraryCause`,
//! `LibraryRefusal`).
//!
//! [`PackageNodeKey`]`{package: package_id, node: WireNodeId}` (ADR-013 T-3)
//! is the sole cross-package node reference this module defines. `node`'s
//! type, `crate::digest::WireNodeId`, is the `F` foundation layer's own
//! type (ADR-011 `:588`), not this module's. `ImportView` (ADR-013 T-1) is
//! a different module's type; this module does not define it.
//!
//! Name resolution against an imported dependency's exports -- binding a
//! qualified reference's `a::Name` qualifier, and reporting a missing or
//! ambiguous name -- is E3's own resolution over an `ImportView`, performed
//! by the importing package's own check stage (FR-087-AC-4), never by
//! `library` calling back into itself. This module names none of
//! `resolve_name`, `ExportIdentity`, `NameReference` or `NameRefusal`.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::diagnostic::Code;
use crate::digest::WireNodeId;
use crate::value::node::is_qualified_name;

mod package_identity;

pub(crate) use package_identity::PACKAGE_ID_VERSION;
use package_identity::{project_declarations, ProjectedDeclarations};
pub use package_identity::{NodeDefect, PreimageDefect};

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
    /// RFC 8785 JCS bytes. The only constructor (ADR-013 O-02): a wire's own
    /// declared `package_id` digest is never parsed into a `PackageId`
    /// directly (that would let untrusted hex mint an "authoritative"
    /// identity); a wire-read candidate's `package_id` is always this
    /// function applied to the preimage bytes the wire itself carries, and
    /// `verify_package` (crate-private) then requires it to equal whatever
    /// a caller separately claims.
    pub fn of_preimage(preimage: &[u8]) -> Self {
        Self(Sha256::digest(preimage).into())
    }

    /// Lowercase hex spelling of an already-constructed `PackageId`, for
    /// comparing against a wire's own hex-spelled digest (e.g. IR's
    /// already-verified `package_id.digest`). Never a second constructor:
    /// `of_preimage` remains the only way to produce a `PackageId` that
    /// flows anywhere as an actual identity; this only formats one that
    /// already exists, for reporting or cross-checking.
    pub(crate) fn hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

/// A cross-package node reference (ADR-013 T-3): the sole cross-package node
/// reference; pins the verified content (`package`) and names a node inside
/// it (`node`), without ever constructing a `NodeKey` from wire bytes.
/// Equality is declared: both components compare lexically (ADR-013 §2),
/// matching the derived `PartialEq`/`Eq`/`PartialOrd`/`Ord` below -- no
/// digest or structural comparison over the referenced node's own content
/// substitutes. An I2 reference into an imported package's `ImportView`
/// names a node this way; a `WireNodeId` becomes a `NodeKey` only by lookup
/// in an already-checked package, at E4 (the dependency's own checked
/// package, compiled from its digest-addressed source) or at E9 (`replay`'s
/// recompiled package) -- never by a conversion function, and none is
/// defined here.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageNodeKey {
    /// The verified package's own content-addressed identity.
    pub package: PackageId,
    /// The node's wire spelling inside that package, resolved to a
    /// `NodeKey` only by lookup in an already-checked package (E4/E9).
    pub node: WireNodeId,
}

impl PackageNodeKey {
    /// Build a reference from its two already-known components. Not
    /// checked typestate (R-10 governs `CheckedGraph`/`CheckedPackage`, not
    /// this plain data key): a `PackageNodeKey` names a node without
    /// claiming it resolves to one, the same way `WireNodeId::from_digest`
    /// carries no resolution claim either.
    pub fn new(package: PackageId, node: WireNodeId) -> Self {
        Self { package, node }
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
    /// A wire-read candidate's own recompute of `package_id` from its
    /// identity preimage disagrees with IR's already-verified
    /// `package_id.digest` for the same preimage bytes, refused at
    /// [`PACKAGE_ID_PATH`]. Distinct from [`Self::PackageIdMismatch`]: that
    /// variant compares two values this crate itself derives from the same
    /// bytes by the same procedure (tautological on the wire-read path);
    /// this one compares against a digest IR derived independently, so it
    /// is the check that actually catches this crate's own canonicalization
    /// ever diverging from IR's (QSL-6 L2).
    #[error("package_id digest disagrees with IR's own already-verified recompute")]
    IdentityDivergedFromIr {
        /// The package's identity.
        library: LibraryName,
        /// IR's own already-verified hex digest.
        ir_digest: Box<str>,
        /// This crate's own recompute, as lowercase hex.
        recomputed_hex: String,
    },
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
            | Self::IdentityDivergedFromIr { .. }
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
            | Self::IdentityDivergedFromIr { .. }
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
            Self::PackageIdMismatch { .. }
            | Self::DuplicatePackageId(_)
            | Self::IdentityDivergedFromIr { .. } => Some(PACKAGE_ID_PATH),
            Self::InvalidPreimage { .. } => Some(IDENTITY_PREIMAGE_PATH),
            Self::InvalidQualifier { .. }
            | Self::UndeclaredExport { .. }
            | Self::ConflictingDefinition { .. }
            | Self::ImportCycle { .. }
            | Self::StaleDependency { .. }
            | Self::MissingImport { .. } => None,
        }
    }

    /// FR-087-AC-12's classification: every variant classifies, honestly,
    /// to exactly one of an ADR-011 I2 graph rule, the §4 binding's
    /// condition 2 or 3, or E3 name resolution, or to `DuplicatePackageId`'s
    /// own named exception outside all four.
    pub fn class(&self) -> RefusalClass {
        match self {
            // The §4 binding's condition 2 itself (the digest recomputation
            // and comparison), and the two per-package admission checks
            // `verify_package` raises alongside it (FR-087-AC-3: the same
            // reused function, after condition 2's own comparison has
            // already completed).
            Self::PackageIdMismatch { .. }
            | Self::InvalidPreimage { .. }
            | Self::IdentityDivergedFromIr { .. }
            | Self::UndeclaredExport { .. } => RefusalClass::BindingCondition(2),
            // The §4 binding's condition 3: the identity is present: only
            // the lock-recorded version disagrees.
            Self::StaleDependency {
                cause: StaleCause::RevisionMismatch,
                ..
            } => RefusalClass::BindingCondition(3),
            // E3 name resolution: moves conceptually with the removed
            // `resolve_name` (owner ruling item 3(b)).
            Self::InvalidQualifier { .. } => RefusalClass::E3NameResolution,
            // ADR-011 `:203-210`'s first I2 rule: a missing or unlisted
            // identity.
            Self::MissingImport { .. }
            | Self::StaleDependency {
                cause: StaleCause::ByteDigestMismatch,
                ..
            } => RefusalClass::I2Rule(1),
            // I2's second rule: two dependency paths reach one identity with
            // different selections.
            Self::ConflictingDefinition { .. } => RefusalClass::I2Rule(2),
            // I2's third rule: the import graph has a cycle.
            Self::ImportCycle { .. } => RefusalClass::I2Rule(3),
            // The named exception: conflicting metadata over identical
            // identity content is a precondition on the supplied pool
            // itself, not an I2/§4/E3 question.
            Self::DuplicatePackageId(_) => RefusalClass::SuppliedPoolPrecondition,
        }
    }
}

/// FR-087-AC-12's classification of a [`LibraryRefusal`]: exactly one of an
/// ADR-011 I2 graph rule, the §4 binding's condition 2 or 3, E3 name
/// resolution, or the one named exception outside all four
/// (`DuplicatePackageId`, a precondition on the supplied pool itself).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusalClass {
    /// One of ADR-011 `:203-210`'s three I2 graph rules (1, 2 or 3).
    I2Rule(u8),
    /// The §4 binding's condition 2 or 3.
    BindingCondition(u8),
    /// E3 name resolution (moves conceptually with the removed
    /// `resolve_name`, owner ruling item 3(b)).
    E3NameResolution,
    /// `DuplicatePackageId`'s own reasoning: conflicting metadata over
    /// identical identity content, a precondition on the supplied pool
    /// itself, not an I2/§4/E3 question.
    SuppliedPoolPrecondition,
}

/// Recompute `package`'s `package_id` from its identity preimage, then
/// validate the preimage and derive the package's export node keys from it.
///
/// ADR-011 §4's I2 verified binding, checks 1-2: `package.package_id` must
/// equal `PackageId::of_preimage(&package.identity_preimage)` for every
/// `LibraryPackage` this crate ever admits, since `of_preimage` is
/// [`PackageId`]'s only constructor (ADR-013 O-02). What that equality
/// actually proves depends on the caller. On the wire path (the layer-4
/// `package` module's `checked_v2` reader), the candidate's `package_id` is
/// minted from these same `identity_preimage` bytes one call earlier, so
/// this function's own recompute is a structural invariant, not a fresh
/// test of the wire's claim -- `checked_v2` has already cross-checked that
/// mint against IR's own independently-verified `package_id.digest` before
/// ever constructing the candidate (QSL-6 L2), and that comparison, not
/// this one, is what actually catches the wire path's condition 2. What
/// this function performs freshly on every path is validating that the
/// preimage itself is well-formed. Check 3 -- the identity is listed in the
/// consumer's library lock or pinned request -- is the caller's:
/// [`resolve_libraries`] applies it when the package is offered as a
/// candidate import.
pub(crate) fn verify_package(
    package: &LibraryPackage,
) -> Result<ProjectedDeclarations, LibraryRefusal> {
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

/// Every name `identity_preimage` declares, in ascending order (FR-307: "a
/// package's local declarations are exactly its exports", this module's own
/// doc). The layer-4 `package` I2 reader calls this to populate a freshly
/// wire-read [`LibraryPackage::exports`] before handing the candidate to
/// [`verify_package`]: a package read straight from its own wire bytes
/// carries no separate export selection, so its exports are exactly what its
/// preimage declares.
#[allow(
    dead_code,
    reason = "no production caller yet: the I2 reader (`package::checked_v2`) is `pub(crate)` with no caller until ADR-011 §4's round trip (QSL-6 slice S3) lands; until then only its own tests reach this"
)]
pub(crate) fn declared_exports(identity_preimage: &[u8]) -> Result<Vec<String>, PreimageDefect> {
    Ok(project_declarations(identity_preimage)?
        .declared_names()
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
/// in declaration order, after recomputing the root's and every supplied
/// library's `package_id`.
pub fn resolve_libraries(
    root: &LibraryPackage,
    supplied: &[LibraryPackage],
) -> Result<LibraryLock, LibraryRefusal> {
    verify_package(root)?;
    let mut by_id: BTreeMap<PackageId, &LibraryPackage> = BTreeMap::new();
    for package in supplied {
        verify_package(package)?;
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
        let found = by_id.get(&import.package_id).filter(|package| {
            package.library == import.library && package.version == import.version
        });
        let Some(package) = found else {
            let same_id = by_id
                .get(&import.package_id)
                .is_some_and(|package| package.library == import.library);
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

#[cfg(test)]
mod package_node_key_tests {
    use super::*;
    use ix_trace_rs::trace;

    fn package_id(byte: u8) -> PackageId {
        PackageId::of_preimage(&[byte; 32])
    }

    /// FR-087-AC-5: two `PackageNodeKey` values compare equal iff both
    /// components compare lexically equal -- the declared-equality claim
    /// itself, not only that the derive exists. A mutation that swapped the
    /// equality implementation for a structural comparison over the
    /// referenced node's content (rather than the two components
    /// themselves) would still pass a same-value-same-value check; this
    /// test additionally pins that changing either component alone breaks
    /// equality, which such a mutation could not do consistently for an
    /// opaque `WireNodeId`.
    #[trace("TC-245", "FR-087-AC-5")]
    #[test]
    fn equality_holds_iff_both_components_are_lexically_equal() {
        let a = PackageNodeKey::new(package_id(1), WireNodeId::from_digest([9; 32]));
        let same = PackageNodeKey::new(package_id(1), WireNodeId::from_digest([9; 32]));
        let different_package =
            PackageNodeKey::new(package_id(2), WireNodeId::from_digest([9; 32]));
        let different_node = PackageNodeKey::new(package_id(1), WireNodeId::from_digest([8; 32]));

        assert_eq!(a, same);
        assert_ne!(a, different_package);
        assert_ne!(a, different_node);
    }
}
