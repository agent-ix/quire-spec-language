// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checked-package handoff writer (FR-517, FR-518): [`write`] compiles a
//! complete-V1 unit in process through [`crate::emit_checked_v2`], reads the
//! emitted package back through Contract IR's reader, and writes a new
//! directory holding the package, the exact input bytes, the selection
//! sidecar ([`Selection`]) and, on request, the adverse mutation corpus.
//!
//! The sidecar is derived from the in-process checked unit and the request,
//! never from the emitted bytes; the one join to the emitted bytes is the wire
//! id of each node the checked unit names. [`Selection::check`] compares an
//! admitted package with it through the borrowed
//! [`quire_contract_model::CheckedPackageV2`] surface, so a consumer parses
//! and compares with no copy of the format. The corpus edits package bytes
//! only to mutate them; every verdict on a package comes from
//! `read_checked_package` and [`Selection::check`].

//! # A downstream consumer
//!
//! A crate that enables only the `checked-handoff` feature compiles [`write`]
//! and [`Selection::check`] against this public surface alone:
//!
//! ```no_run
//! use quire_spec_language::checked_handoff::{
//!     write, CheckedHandoffRequest, Corpus, Selection,
//! };
//! use quire_spec_language::emit_checked_v2::Request;
//!
//! fn handoff(emit: Request, directory: &std::path::Path) {
//!     let request = CheckedHandoffRequest { emit, corpus: Corpus::None };
//!     write(&request, directory).expect("the handoff is written");
//!     let sidecar = std::fs::read(directory.join("selection.json")).unwrap();
//!     let _selection = Selection::read(&sidecar).expect("a sidecar");
//! }
//! # fn main() {}
//! ```
//!
//! The writer's internals are not part of that surface:
//!
//! ```compile_fail
//! use quire_spec_language::checked_handoff::tree::Files;
//! ```

mod corpus;
mod inventory;
mod selection;
mod tree;

use std::path::Path;

use quire_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageV2,
};

use crate::emit_checked_v2::{self, Compiled};

pub use corpus::{Expected, ReaderRefusal, Stage};
pub use inventory::{Inventory, InventoryReadError, InventoryRow, InventorySource};
pub use selection::{Selection, SelectionAxis, SelectionReadError, SelectionRefusal};

/// The format of `selection.json`.
pub const SELECTION_FORMAT: &str = "quire.checked-handoff.selection/1";

/// The format of `inventory.json`.
pub const INVENTORY_FORMAT: &str = "quire.checked-handoff.inventory/1";

/// The format of `mutations/manifest.json`.
pub const MUTATIONS_FORMAT: &str = "quire.checked-handoff.mutations/1";

/// Which adverse corpus the writer adds (FR-518).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Corpus {
    /// No `mutations/` directory.
    None,
    /// Every case of FR-518 that applies to the package's clock kinds.
    Standard,
}

/// What [`write`] compiles and writes.
#[derive(Debug)]
pub struct CheckedHandoffRequest {
    /// The unit, its libraries, its domain package documents, the package
    /// name, the accepted execution points and the clock configuration of
    /// each temporal clause, as [`emit_checked_v2::emit`] takes them. Its
    /// `out` member is not used.
    pub emit: emit_checked_v2::Request,
    /// The corpus to add.
    pub corpus: Corpus,
}

/// A stable code for an [`Error`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    /// The requested path exists.
    Exists,
    /// The compiler refused the request or an input could not be read.
    Compile,
    /// Contract IR's reader refused or could not finish reading the package.
    Reader,
    /// The admitted package differs from the in-process checked unit.
    ReadBackDiffers,
    /// A corpus case does not get the refusal its table row declares.
    CaseMismatch,
    /// A library could not be admitted as evidence.
    Dependency,
    /// A file label cannot name a file inside the directory.
    Label,
    /// A file could not be written.
    Io,
    /// A sidecar or manifest could not be encoded.
    Encode,
    /// The checked unit holds a value the sidecar cannot name.
    Unnameable,
}

/// Why no directory was written.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested path exists; its contents are untouched.
    #[error("{path} exists")]
    Exists {
        /// The requested path.
        path: String,
    },
    /// The compiler refused the request or an input could not be read.
    #[error("{0}")]
    Compile(#[from] emit_checked_v2::Error),
    /// Contract IR's reader refused the emitted package.
    #[error("the reader refused the emitted package: {0}")]
    ReaderRefused(Box<CheckedPackageRefusal>),
    /// Contract IR's reader reached a limit before a conclusion.
    #[error("the reader reached a limit: {0}")]
    ReaderIncomplete(String),
    /// The admitted package differs from the in-process checked unit.
    #[error("the admitted package differs from the checked unit: {detail}")]
    ReadBackDiffers {
        /// What differs.
        detail: String,
    },
    /// A corpus case does not get the refusal its table row declares.
    #[error("case {identity} does not get its declared refusal")]
    CaseMismatch {
        /// The case identity.
        identity: String,
    },
    /// A library the unit imports could not be admitted as evidence.
    #[error("library {identity} cannot be admitted as evidence: {detail}")]
    Dependency {
        /// The library identity.
        identity: String,
        /// Why.
        detail: String,
    },
    /// A file label cannot name a file inside the directory.
    #[error("{label:?} cannot name a file inside the handoff directory")]
    Label {
        /// The label.
        label: String,
    },
    /// A file could not be written.
    #[error("cannot write {path}: {source}")]
    Io {
        /// The path.
        path: String,
        /// The I/O failure.
        source: std::io::Error,
    },
    /// A sidecar or manifest could not be encoded.
    #[error("cannot encode {what}: {source}")]
    Encode {
        /// The file.
        what: &'static str,
        /// The failure.
        source: serde_json::Error,
    },
    /// The checked unit holds a value the sidecar cannot name.
    #[error("the checked unit has no {what}")]
    Unnameable {
        /// The missing value.
        what: String,
    },
}

impl Error {
    /// The stable code of this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Exists { .. } => ErrorCode::Exists,
            Self::Compile(_) => ErrorCode::Compile,
            Self::ReaderRefused(_) | Self::ReaderIncomplete(_) => ErrorCode::Reader,
            Self::ReadBackDiffers { .. } => ErrorCode::ReadBackDiffers,
            Self::CaseMismatch { .. } => ErrorCode::CaseMismatch,
            Self::Dependency { .. } => ErrorCode::Dependency,
            Self::Label { .. } => ErrorCode::Label,
            Self::Io { .. } => ErrorCode::Io,
            Self::Encode { .. } => ErrorCode::Encode,
            Self::Unnameable { .. } => ErrorCode::Unnameable,
        }
    }
}

/// The stages a package passes through, and the disk it is written to. The
/// production implementation, [`RealEnvironment`], calls Contract IR's reader and
/// [`Selection::check`] and writes real files; a test replaces one of them to
/// inject a refusal or a failure.
pub trait Environment {
    /// Read `bytes` as a checked package.
    fn read(
        &self,
        bytes: &[u8],
        limits: CheckedPackageReadLimits,
        evidence: &CheckedPackageEvidence,
    ) -> CheckedPackageDispatchResult;

    /// Compare an admitted package with the sidecar.
    fn check(
        &self,
        selection: &Selection,
        package: &CheckedPackageV2,
    ) -> Result<(), SelectionRefusal>;

    /// Write `contents` to a new file at `path`; the `index`th write of one
    /// tree.
    fn write_file(&self, index: usize, path: &Path, contents: &[u8]) -> std::io::Result<()>;
}

/// The real stages and the real disk.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealEnvironment;

impl Environment for RealEnvironment {
    fn read(
        &self,
        bytes: &[u8],
        limits: CheckedPackageReadLimits,
        evidence: &CheckedPackageEvidence,
    ) -> CheckedPackageDispatchResult {
        read_checked_package(bytes, limits, evidence)
    }

    fn check(
        &self,
        selection: &Selection,
        package: &CheckedPackageV2,
    ) -> Result<(), SelectionRefusal> {
        selection.check(package)
    }

    fn write_file(&self, _index: usize, path: &Path, contents: &[u8]) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

/// The read limits the writer uses for every read it makes.
pub const READ_LIMITS: CheckedPackageReadLimits = CheckedPackageReadLimits::bounded();

/// Compile `request` and write its handoff directory at `directory`, a path
/// that does not exist yet. See the module documentation for the layout.
pub fn write(request: &CheckedHandoffRequest, directory: &Path) -> Result<(), Error> {
    write_with(request, directory, &RealEnvironment)
}

/// [`write`] over `environment`, which a test replaces to inject a refusal or
/// a failure.
pub fn write_with(
    request: &CheckedHandoffRequest,
    directory: &Path,
    environment: &dyn Environment,
) -> Result<(), Error> {
    if tree::exists(directory) {
        return Err(Error::Exists {
            path: directory.display().to_string(),
        });
    }
    let compiled = emit_checked_v2::compile(&request.emit)?;
    let bytes = compiled.emitted.package().bytes().to_vec();
    let evidence = read_evidence(&compiled)?;
    let package = match environment.read(&bytes, READ_LIMITS, &evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => package,
        CheckedPackageDispatchResult::Refused(refusal) => {
            return Err(Error::ReaderRefused(Box::new(refusal)))
        }
        CheckedPackageDispatchResult::Incomplete(incomplete) => {
            return Err(Error::ReaderIncomplete(format!("{incomplete:?}")))
        }
    };
    let selection = Selection::derive(&compiled, &request.emit, &bytes)?;
    environment
        .check(&selection, &package)
        .map_err(|refusal| Error::ReadBackDiffers {
            detail: refusal.to_string(),
        })?;
    let mut files = tree::Files::new();
    files.insert("checked-package.json", bytes.clone())?;
    insert_inputs(&mut files, &compiled)?;
    files.insert("selection.json", selection.to_bytes()?)?;
    files.insert("inventory.json", inventory_file(&compiled, &bytes)?)?;
    files.insert("identities.json", identity_file(&compiled, &package)?)?;
    files.insert(
        "equality.json",
        equality_file(&compiled, request, &package)?,
    )?;
    if request.corpus == Corpus::Standard {
        let cases = corpus::build(&bytes, &selection, &evidence, environment)?;
        for (path, contents) in cases.files()? {
            files.insert(&path, contents)?;
        }
    }
    tree::publish(directory, &files, environment)
}

/// `inventory.json` (FR-517 "The inventory file"): `qsl_inventory` over the
/// in-process unit, joined to the wire node ids of the emitted `bytes`.
fn inventory_file(compiled: &Compiled, bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let outcome = compiled
        .inventory
        .as_ref()
        .map_err(|what| Error::Unnameable {
            what: format!("inventory: {what}"),
        })?;
    Inventory::derive(outcome, bytes)?.to_bytes()
}

/// `identities.json` (FR-531 "The identity file"): the selected model identity
/// rows of the in-process unit that join the admitted `package`, in the order
/// the emitter holds them.
fn identity_file(
    compiled: &Compiled,
    package: &quire_contract_model::CheckedPackageV2,
) -> Result<Vec<u8>, Error> {
    let unnameable = |what: String| Error::Unnameable { what };
    let graph = compiled.checked.package().graph();
    let joined = graph
        .selected_exact_identities(package)
        .map_err(|refused| unnameable(format!("selected identities: {refused:?}")))?;
    let declarations: Vec<_> = graph
        .selected_exact_declaration_rows()
        .iter()
        .filter(|row| {
            joined
                .declaration_rows()
                .any(|kept| kept.structural_node == row.structural_node)
        })
        .cloned()
        .collect();
    let fields: Vec<_> = graph
        .selected_exact_field_rows()
        .iter()
        .filter(|row| joined.field_rows().contains(row))
        .cloned()
        .collect();
    qsl_package::ReadIdentityRows::of_rows(package, declarations, fields)
        .to_bytes(package)
        .map_err(|refused| unnameable(format!("identity file: {refused}")))
}

/// `equality.json` (FR-531 "The equality file"): the producer's equality
/// witness rows of the in-process unit, written empty when the unit holds no
/// equality application, in the order the emitter holds them.
fn equality_file(
    compiled: &Compiled,
    request: &CheckedHandoffRequest,
    package: &quire_contract_model::CheckedPackageV2,
) -> Result<Vec<u8>, Error> {
    let unnameable = |what: String| Error::Unnameable { what };
    let selections =
        emit_checked_v2::clock_selections(&compiled.checked, &request.emit.temporal_selections)?;
    qsl_package::emit_checked_with_temporal_selections(compiled.checked.package(), &selections)
        .map_err(|refusal| unnameable(format!("equality witness: {refusal}")))?
        .equality_witness(package)
        .to_bytes(package)
        .map_err(|refused| unnameable(format!("equality file: {refused}")))
}

/// The exact input bytes under `sources/` and `models/`.
fn insert_inputs(files: &mut tree::Files, compiled: &Compiled) -> Result<(), Error> {
    files.insert(
        &format!("sources/{}", compiled.checked.source().identity().identity),
        compiled.source.clone(),
    )?;
    for library in &compiled.libraries {
        files.insert(
            &format!("sources/{}", library.source.identity),
            library.bytes.clone(),
        )?;
    }
    for (index, model) in compiled.models.iter().enumerate() {
        files.insert(&format!("models/{index}.json"), model.clone())?;
    }
    Ok(())
}

/// The evidence Contract IR's reader needs to read the package `request`
/// compiles to.
pub fn evidence(request: &CheckedHandoffRequest) -> Result<CheckedPackageEvidence, Error> {
    read_evidence(&emit_checked_v2::compile(&request.emit)?)
}

/// The compiler's evidence, the domain package documents the writer holds,
/// and the admitted package of every library the unit imports.
fn read_evidence(compiled: &Compiled) -> Result<CheckedPackageEvidence, Error> {
    let documents =
        qsl_semantics::model::intake::package_input(compiled.models.iter().map(Vec::as_slice))
            .map_err(|refused| Error::Unnameable {
                what: format!("domain package documents: {refused:?}"),
            })?;
    let mut evidence = compiled.emitted.evidence().clone();
    supply_documents(&mut evidence, &documents, "the unit")?;
    admit_dependencies(compiled.checked.package(), &mut evidence, &documents)?;
    Ok(evidence)
}

fn supply_documents(
    evidence: &mut CheckedPackageEvidence,
    documents: &qsl_semantics::model::intake::PackageInput,
    owner: &str,
) -> Result<(), Error> {
    for (identity, document) in documents {
        evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .map_err(|conflict| Error::Dependency {
                identity: owner.to_owned(),
                detail: format!("{conflict:?}"),
            })?;
    }
    Ok(())
}

/// Admit each library of `package`'s closure and supply it to `evidence`.
fn admit_dependencies(
    package: &qsl_package::CheckedPackage,
    evidence: &mut CheckedPackageEvidence,
    documents: &qsl_semantics::model::intake::PackageInput,
) -> Result<(), Error> {
    for (identity, dependency) in package.dependencies() {
        let failed = |detail: String| Error::Dependency {
            identity: identity.as_str().to_owned(),
            detail,
        };
        let emission =
            qsl_package::emit_checked(dependency).map_err(|refusal| failed(refusal.to_string()))?;
        let mut own = emission.evidence().clone();
        supply_documents(&mut own, documents, identity.as_str())?;
        admit_dependencies(dependency, &mut own, documents)?;
        let CheckedPackageDispatchResult::AdmittedV2(admitted) =
            read_checked_package(emission.package().bytes(), READ_LIMITS, &own)
        else {
            return Err(failed("the reader did not admit it".to_owned()));
        };
        evidence
            .insert_dependency_package(identity.as_str(), admitted)
            .map_err(|conflict| failed(format!("{conflict:?}")))?;
    }
    Ok(())
}
