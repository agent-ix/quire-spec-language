// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checked-package handoff writer (FR-517, FR-518): [`write()`] compiles a
//! complete-V1 unit in process through [`crate::emit_checked_v2`], reads the
//! emitted package back through Contract IR's reader, and writes a new
//! directory holding the package, the exact input bytes, the selection
//! sidecar ([`Selection`]) and, on request, the adverse mutation corpus.
//!
//! The sidecar is derived from the in-process checked unit and the request,
//! never from the emitted bytes; the one join to the emitted bytes is the wire
//! id of each node the checked unit names. [`Selection::check`] compares an
//! admitted package with it through the borrowed
//! [`filament_contract_model::CheckedPackageV2`] surface, so a consumer parses
//! and compares with no copy of the format. The corpus edits package bytes
//! only to mutate them; every verdict on a package comes from
//! `read_checked_package` and [`Selection::check`].

//! # A downstream consumer
//!
//! The default `fsl-command` build exposes [`write()`] and
//! [`Selection::check`] at this public surface:
//!
//! ```no_run
//! use fsl_command::checked_handoff::{
//!     write, CheckedHandoffRequest, Corpus, Selection,
//! };
//! use fsl_command::emit_checked_v2::Request;
//! use fsl_package::NativeJoinLimits;
//!
//! fn handoff(emit: Request, directory: &std::path::Path) {
//!     let request = CheckedHandoffRequest {
//!         emit,
//!         corpus: Corpus::None,
//!         native_join: NativeJoinLimits::new(1 << 20, 1 << 20),
//!     };
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
//! use fsl_command::checked_handoff::tree::Files;
//! ```

mod corpus;
mod inventory;
mod selection;
mod tree;

use std::path::Path;
use std::collections::BTreeMap;
use std::sync::Arc;

use filament_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageV2,
};

use crate::emit_checked_v2::{self, Compiled};

pub use corpus::{Expected, ReaderRefusal, Stage};
pub use inventory::{Inventory, InventoryReadError, InventoryRow, InventorySource};
pub use selection::{Selection, SelectionAxis, SelectionReadError, SelectionRefusal};

/// The format of `selection.json`.
pub const SELECTION_FORMAT: &str = "filament.checked-handoff.selection/1";

/// The format of `inventory.json`.
pub const INVENTORY_FORMAT: &str = "filament.checked-handoff.inventory/1";

/// The format of `mutations/manifest.json`.
pub const MUTATIONS_FORMAT: &str = "filament.checked-handoff.mutations/1";

const DEPENDENCIES_FORMAT: &str = "filament.checked-handoff.dependencies/1";

#[derive(serde::Serialize)]
struct DependencyManifest<'a> {
    format: &'static str,
    dependencies: Vec<DependencyRow<'a>>,
}

#[derive(serde::Serialize)]
struct DependencyRow<'a> {
    identity: &'a str,
    directory: String,
}

fn json_file(value: &impl serde::Serialize, what: &'static str) -> Result<Vec<u8>, Error> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| Error::Encode { what, source })?;
    bytes.push(b'\n');
    Ok(bytes)
}

struct DependencyArtifact {
    emission: fsl_package::Emission,
    package: Arc<CheckedPackageV2>,
}

/// Which adverse corpus the writer adds (FR-518).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Corpus {
    /// No `mutations/` directory.
    None,
    /// Every case of FR-518 that applies to the package's clock kinds.
    Standard,
}

/// What [`write()`] compiles and writes.
#[derive(Debug)]
pub struct CheckedHandoffRequest {
    /// The unit, its libraries, its domain package documents, the package
    /// name, the accepted execution points and the clock configuration of
    /// each temporal clause, as [`emit_checked_v2::emit`] takes them. Its
    /// `out` member is not used.
    pub emit: emit_checked_v2::Request,
    /// The corpus to add.
    pub corpus: Corpus,
    /// The finite work and logical-storage ceilings of the identity file's
    /// native joins (FR-531, FR-358). The writer owns no default: the caller
    /// states them, and one budget of them serves the write.
    pub native_join: fsl_package::NativeJoinLimits,
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
    /// The caller's native join work or storage ceiling was consumed.
    NativeJoinExhausted,
    /// A file of a handoff directory could not be read.
    Read,
    /// A handoff directory's `selection.json` is not a sidecar of this format.
    Sidecar,
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
    /// Contract IR's reader had no operation catalog to read the package against.
    #[error("the reader had no operation catalog: {0}")]
    ReaderCatalogUnavailable(String),
    /// The admitted package differs from the in-process checked unit.
    #[error("the admitted package differs from the checked unit: {detail}")]
    ReadBackDiffers {
        /// What differs.
        detail: String,
    },
    /// The admitted package departs from a sidecar row on one typed axis.
    #[error("the admitted package differs from the selection: {0}")]
    SelectionMismatch(#[from] SelectionRefusal),
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
    /// The request's native join work or storage ceiling was consumed while
    /// joining the selected identities (FR-358). The stage diagnostic keeps
    /// IR's native refusal; it is not an unnameable value.
    #[error("the native join budget was consumed: {0}")]
    NativeJoinExhausted(Box<fsl_package::NativeJoinLimitError>),
    /// A file of the handoff directory could not be read, or is over its
    /// byte ceiling.
    #[error("cannot read {path}: {source}")]
    Read {
        /// The file.
        path: String,
        /// The I/O failure.
        source: std::io::Error,
    },
    /// The handoff directory's `selection.json` does not parse.
    #[error("{0}")]
    Sidecar(#[from] SelectionReadError),
}

impl Error {
    /// The stable code of this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Exists { .. } => ErrorCode::Exists,
            Self::Compile(_) => ErrorCode::Compile,
            Self::ReaderRefused(_)
            | Self::ReaderIncomplete(_)
            | Self::ReaderCatalogUnavailable(_) => ErrorCode::Reader,
            Self::ReadBackDiffers { .. } | Self::SelectionMismatch(_) => ErrorCode::ReadBackDiffers,
            Self::CaseMismatch { .. } => ErrorCode::CaseMismatch,
            Self::Dependency { .. } => ErrorCode::Dependency,
            Self::Label { .. } => ErrorCode::Label,
            Self::Io { .. } => ErrorCode::Io,
            Self::Encode { .. } => ErrorCode::Encode,
            Self::Unnameable { .. } => ErrorCode::Unnameable,
            Self::NativeJoinExhausted(_) => ErrorCode::NativeJoinExhausted,
            Self::Read { .. } => ErrorCode::Read,
            Self::Sidecar(_) => ErrorCode::Sidecar,
        }
    }

    /// The wire-safe name used by `filament-outcome/1` inspect refusals.
    pub fn code_name(&self) -> &'static str {
        match self.code() {
            ErrorCode::Exists => "handoff-exists",
            ErrorCode::Compile => "handoff-compile",
            ErrorCode::Reader => "handoff-reader",
            ErrorCode::ReadBackDiffers => "handoff-read-back-differs",
            ErrorCode::CaseMismatch => "handoff-case-mismatch",
            ErrorCode::Dependency => "handoff-dependency",
            ErrorCode::Label => "handoff-label",
            ErrorCode::Io => "handoff-io",
            ErrorCode::Encode => "handoff-encode",
            ErrorCode::Unnameable => "handoff-unnameable",
            ErrorCode::NativeJoinExhausted => "handoff-native-join-exhausted",
            ErrorCode::Read => "handoff-read",
            ErrorCode::Sidecar => "handoff-sidecar",
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

/// [`write()`] over `environment`, which a test replaces to inject a refusal or
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
    write_compiled(request, directory, environment, compiled, None)
}

/// Write a handoff from the checked inputs and package already produced by
/// `native-compile/1`. The byte equality guard runs before read-back or any
/// filesystem write.
pub(crate) fn write_compiled_from_command(
    request: &CheckedHandoffRequest,
    directory: &Path,
    compiled: Compiled,
    expected_package: &[u8],
    source_file: &str,
) -> Result<(), Error> {
    if tree::exists(directory) {
        return Err(Error::Exists {
            path: directory.display().to_string(),
        });
    }
    let package_bytes = compiled.emitted.package().bytes();
    if package_bytes != expected_package {
        return Err(Error::ReadBackDiffers {
            detail: "the command package differs from the retained checked package".to_owned(),
        });
    }
    write_compiled(
        request,
        directory,
        &RealEnvironment,
        compiled,
        Some(source_file),
    )
}

fn write_compiled(
    request: &CheckedHandoffRequest,
    directory: &Path,
    environment: &dyn Environment,
    compiled: Compiled,
    command_source_file: Option<&str>,
) -> Result<(), Error> {
    let bytes = compiled.emitted.package().bytes().to_vec();
    let (evidence, dependencies) = acquire_evidence(&compiled)?;
    let package = admit(environment.read(&bytes, READ_LIMITS, &evidence))?;
    let selection = Selection::derive(&compiled, &request.emit, &bytes, command_source_file)?;
    environment
        .check(&selection, &package)
        .map_err(|refusal| Error::ReadBackDiffers {
            detail: refusal.to_string(),
        })?;
    let mut files = tree::Files::new();
    files.insert("checked-package.json", bytes.clone())?;
    insert_inputs(&mut files, &compiled, command_source_file)?;
    files.insert("selection.json", selection.to_bytes()?)?;
    files.insert("inventory.json", inventory_file(&compiled, &bytes)?)?;
    let mut budget = request.native_join.budget();
    let manifest = DependencyManifest {
        format: DEPENDENCIES_FORMAT,
        dependencies: dependencies.keys().enumerate().map(|(index, identity)| {
            DependencyRow { identity, directory: format!("dependencies/{index}") }
        }).collect(),
    };
    files.insert("dependencies.json", json_file(&manifest, "dependencies.json")?)?;
    for (index, (_, dependency)) in dependencies.iter().enumerate() {
        let directory = format!("dependencies/{index}");
        files.insert(&format!("{directory}/checked-package.json"), dependency.emission.package().bytes().to_vec())?;
        files.insert(&format!("{directory}/identities.json"), emission_identity_file(&dependency.emission, &dependency.package, &mut budget, request.native_join)?)?;
        files.insert(&format!("{directory}/equality.json"), emission_equality_file(&dependency.emission, &dependency.package, &mut budget, request.native_join)?)?;
    }
    files.insert(
        "identities.json",
        identity_file(&compiled, &package, &mut budget, request.native_join)?,
    )?;
    files.insert(
        "equality.json",
        equality_file(
            &compiled,
            request,
            &package,
            &mut budget,
            request.native_join,
        )?,
    )?;
    if request.corpus == Corpus::Standard {
        let cases = corpus::build(&bytes, &selection, &evidence, environment)?;
        for (path, contents) in cases.files()? {
            files.insert(&path, contents)?;
        }
    }
    tree::publish(directory, &files, environment)
}

/// The admitted package of a read, or the reader's refusal.
fn admit(read: CheckedPackageDispatchResult) -> Result<Box<CheckedPackageV2>, Error> {
    match read {
        CheckedPackageDispatchResult::AdmittedV2(package) => Ok(package),
        CheckedPackageDispatchResult::Refused(refusal) => {
            Err(Error::ReaderRefused(Box::new(refusal)))
        }
        CheckedPackageDispatchResult::Incomplete(incomplete) => {
            Err(Error::ReaderIncomplete(format!("{incomplete:?}")))
        }
        CheckedPackageDispatchResult::CatalogUnavailable(unavailable) => {
            Err(Error::ReaderCatalogUnavailable(format!("{unavailable:?}")))
        }
    }
}

/// A handoff directory compiled back in process ([`compile()`]): the checked
/// package its sources compile to, the package Contract IR's reader admits
/// from `checked-package.json`, and the domain package documents. The two
/// packages are one package: [`compile()`] returns this only when the
/// recompiled bytes equal the file.
#[derive(Debug)]
pub struct CompiledHandoff {
    compiled: Compiled,
    admitted: Box<CheckedPackageV2>,
    sources: fsl_analyze::analyze::SourceProvision,
}

impl CompiledHandoff {
    /// The in-process checked package.
    pub fn checked(&self) -> &fsl_package::CheckedPackage {
        self.compiled.checked.package()
    }

    /// The package Contract IR's reader admits from `checked-package.json`.
    pub fn admitted(&self) -> &CheckedPackageV2 {
        &self.admitted
    }

    /// FR-056's package input: the directory's domain package documents by
    /// the identity each declares.
    pub fn packages(&self) -> &fsl_semantics::model::intake::PackageInput {
        &self.compiled.packages
    }

    /// The source bytes used for the in-process compilation, keyed by the
    /// checked package's declared source identities.
    pub fn sources(&self) -> &fsl_analyze::analyze::SourceProvision {
        &self.sources
    }

    /// The ceiling on each package emission the compile made
    /// (`emit.output_bytes`), which an analysis of the package reuses.
    pub fn emit_limits(&self) -> fsl_package::EmitLimits {
        self.compiled.emit
    }
}

/// Compile the handoff directory at `directory` back in process: read
/// `selection.json`, compile the unit, library and domain package files it
/// names with its package name, execution points and clock selections through
/// the entry [`write()`] compiles through, and read `checked-package.json`
/// with Contract IR's reader and the evidence [`write()`] assembles.
///
/// # Errors
///
/// [`Error::Read`] for a file that cannot be read or is over its ceiling,
/// [`Error::Sidecar`] for a sidecar that does not parse, [`Error::Label`] for
/// a sidecar path outside the directory or a label the writer does not give,
/// [`Error::Compile`] for the compiler's refusal, [`Error::ReadBackDiffers`]
/// when the recompiled bytes are not `checked-package.json` or the admitted
/// package departs from the sidecar, and the reader's refusals.
pub fn compile(directory: &Path) -> Result<CompiledHandoff, Error> {
    let package_ceiling = usize::try_from(READ_LIMITS.bytes).unwrap_or(usize::MAX);
    let selection = Selection::read(&tree::read(directory, "selection.json", package_ceiling)?)?;
    let bytes = tree::read(directory, "checked-package.json", package_ceiling)?;
    let inputs = selection
        .inputs(|file| tree::read(directory, file, fsl_foundation::source::MAX_SOURCE_BYTES))?;
    let compiled = emit_checked_v2::compile_inputs(inputs)?;
    if compiled.emitted.package().bytes() != bytes.as_slice() {
        return Err(Error::ReadBackDiffers {
            detail: "the sources compile to bytes other than checked-package.json".to_owned(),
        });
    }
    let evidence = read_evidence(&compiled)?;
    let admitted = admit(read_checked_package(&bytes, READ_LIMITS, &evidence))?;
    selection
        .check(&admitted)
        .map_err(Error::SelectionMismatch)?;
    let mut sources = fsl_analyze::analyze::SourceProvision::default();
    let unit_source = compiled.checked.source().identity();
    let unit_ref = fsl_foundation::source::provenance::RawSourceRef::new(
        unit_source.authority.clone(),
        unit_source.identity.clone(),
    )
    .expect("a checked source identity is valid");
    sources.insert(&unit_ref, compiled.source.clone());
    for library in &compiled.libraries {
        let source = fsl_foundation::source::provenance::RawSourceRef::new(
            library.source.authority.clone(),
            library.source.identity.clone(),
        )
        .expect("a checked library source identity is valid");
        sources.insert(&source, library.bytes.clone());
    }
    Ok(CompiledHandoff {
        compiled,
        admitted,
        sources,
    })
}

/// Verify a handoff directory by compiling its named inputs and checking the
/// emitted package against the strict reader and selection sidecar.
///
/// The command's inspect/read-back surface uses this entry point; the return
/// value retains the admitted package and the checked in-process package for
/// callers that need them.
pub fn verify(directory: &Path) -> Result<Box<CheckedPackageV2>, Error> {
    let _ = std::fs::read_dir(directory).map_err(|source| Error::Read {
        path: directory.display().to_string(),
        source,
    })?;
    let package_ceiling = usize::try_from(READ_LIMITS.bytes).unwrap_or(usize::MAX);
    let selection = Selection::read(&tree::read(directory, "selection.json", package_ceiling)?)?;
    let bytes = tree::read(directory, "checked-package.json", package_ceiling)?;
    let inputs = selection
        .inputs(|file| tree::read(directory, file, fsl_foundation::source::MAX_SOURCE_BYTES))?;
    let compiled = emit_checked_v2::compile_inputs(inputs)?;
    let evidence = read_evidence(&compiled)?;
    let admitted = admit(read_checked_package(&bytes, READ_LIMITS, &evidence))?;
    selection
        .check(&admitted)
        .map_err(Error::SelectionMismatch)?;
    Ok(admitted)
}

/// `inventory.json` (FR-517 "The inventory file"): `fsl_inventory` over the
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
    package: &filament_contract_model::CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
    limits: fsl_package::NativeJoinLimits,
) -> Result<Vec<u8>, Error> {
    emission_identity_file(compiled.emitted.emission(), package, budget, limits)
}

fn emission_identity_file(
    emission: &fsl_package::Emission,
    package: &CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
    limits: fsl_package::NativeJoinLimits,
) -> Result<Vec<u8>, Error> {
    let unnameable = |what: String| Error::Unnameable { what };
    emission
        .identity_rows(package, budget)
        .map_err(|refused| match refused {
            fsl_package::IdentityFileRefusal::Rows(rows) => match limits.refusal(rows, budget) {
                Some(denial) => Error::NativeJoinExhausted(Box::new(denial)),
                None => unnameable("adapter budget denial has no unpaid charge".to_owned()),
            },
            fsl_package::IdentityFileRefusal::Native(native)
                if fsl_semantics::model::intake::is_native_budget_denial(&native) =>
            {
                match limits.refusal(native, budget) {
                    Some(denial) => Error::NativeJoinExhausted(Box::new(denial)),
                    None => unnameable("native budget denial has no unpaid charge".to_owned()),
                }
            }
            other => unnameable(format!("identity file: {other}")),
        })?
        .to_bytes(package)
        .map_err(|refused| unnameable(format!("identity file: {refused}")))
}

/// `equality.json` (FR-531 "The equality file"): the producer's equality
/// witness rows of the in-process unit, written empty when the unit holds no
/// equality application, in the order the emitter holds them.
fn equality_file(
    compiled: &Compiled,
    _request: &CheckedHandoffRequest,
    package: &filament_contract_model::CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
    limits: fsl_package::NativeJoinLimits,
) -> Result<Vec<u8>, Error> {
    emission_equality_file(compiled.emitted.emission(), package, budget, limits)
}

fn emission_equality_file(
    emission: &fsl_package::Emission,
    package: &CheckedPackageV2,
    budget: &mut filament_contract_model::NativeJoinBudget,
    limits: fsl_package::NativeJoinLimits,
) -> Result<Vec<u8>, Error> {
    let unnameable = |what: String| Error::Unnameable { what };
    emission
    .equality_witness(package, budget)
    .map_err(|error| match limits.refusal(error, budget) {
        Some(denial) => Error::NativeJoinExhausted(Box::new(denial)),
        None => unnameable("equality adapter refused without a resource denial".to_owned()),
    })?
    .to_bytes(package)
    .map_err(|refused| unnameable(format!("equality file: {refused}")))
}

/// The exact input bytes under `sources/` and `models/`.
fn insert_inputs(
    files: &mut tree::Files,
    compiled: &Compiled,
    command_source_file: Option<&str>,
) -> Result<(), Error> {
    let unit_file = command_source_file.map_or_else(
        || format!("sources/{}", compiled.checked.source().identity().identity),
        |file| format!("sources/source:{file}"),
    );
    files.insert(&unit_file, compiled.source.clone())?;
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
    acquire_evidence(compiled).map(|(evidence, _)| evidence)
}

fn acquire_evidence(compiled: &Compiled) -> Result<(CheckedPackageEvidence, BTreeMap<String, DependencyArtifact>), Error> {
    let documents =
        fsl_semantics::model::intake::package_input(compiled.models.iter().map(Vec::as_slice))
            .map_err(|refused| Error::Unnameable {
                what: format!("domain package documents: {refused:?}"),
            })?;
    let mut evidence = compiled.emitted.evidence().clone();
    supply_documents(&mut evidence, &documents, "the unit")?;
    let root = compiled.checked.package();
    let mut held = BTreeMap::new();
    let mut pending = vec![root];
    while let Some(package) = pending.pop() {
        for (identity, dependency) in package.dependencies() {
            if held.insert(identity.as_str().to_owned(), dependency.as_ref()).is_none() {
                pending.push(dependency);
            }
        }
    }
    let mut artifacts: BTreeMap<String, DependencyArtifact> = BTreeMap::new();
    let mut pending: Vec<_> = root.direct_dependency_selections().rev().map(|(identity, _)| (identity.as_str().to_owned(), false)).collect();
    while let Some((identity, expanded)) = pending.pop() {
        if artifacts.contains_key(&identity) { continue; }
        let dependency = held.get(&identity).ok_or_else(|| Error::Dependency { identity: identity.clone(), detail: "selected dependency was not retained".to_owned() })?;
        if !expanded {
            pending.push((identity.clone(), true));
            pending.extend(dependency.direct_dependency_selections().rev().map(|(child, _)| (child.as_str().to_owned(), false)));
            continue;
        }
        let emission = fsl_package::emit_checked(dependency, compiled.emit).map_err(|refused| Error::Dependency { identity: identity.clone(), detail: refused.to_string() })?;
        let mut own = emission.evidence().clone();
        supply_documents(&mut own, &documents, &identity)?;
        insert_direct_dependencies(dependency, &artifacts, &mut own)?;
        let package = Arc::from(admit(read_checked_package(emission.package().bytes(), READ_LIMITS, &own))?);
        artifacts.insert(identity, DependencyArtifact { emission, package });
    }
    insert_direct_dependencies(root, &artifacts, &mut evidence)?;
    Ok((evidence, artifacts))
}

fn insert_direct_dependencies(package: &fsl_package::CheckedPackage, artifacts: &BTreeMap<String, DependencyArtifact>, evidence: &mut CheckedPackageEvidence) -> Result<(), Error> {
    for (identity, _) in package.direct_dependency_selections() {
        let artifact = artifacts.get(identity.as_str()).ok_or_else(|| Error::Dependency { identity: identity.as_str().to_owned(), detail: "selected dependency was not admitted".to_owned() })?;
        evidence.insert_dependency_package(identity.as_str(), Arc::clone(&artifact.package)).map_err(|conflict| Error::Dependency { identity: identity.as_str().to_owned(), detail: format!("{conflict:?}") })?;
    }
    Ok(())
}

fn supply_documents(
    evidence: &mut CheckedPackageEvidence,
    documents: &fsl_semantics::model::intake::PackageInput,
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

