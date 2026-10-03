// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-275, FR-276, FR-278 (ADR-029 OP-1): the spine's front end as typed
//! library operations. [`parse`] reads source bytes into syntax (S1, S2),
//! [`select`] admits the domain packages a unit's `model` declarations name
//! (I1), [`check`] checks and links (S3, S4) and [`package`] emits the
//! checked package (E4). [`execute`] calls a checked function (S6a); its
//! signature names `qsl_eval` types, which the spine's public surface may
//! not (FR-100-AC-8), so it is crate-internal.
//!
//! Each operation takes its predecessor's own type, so a program that passes
//! another stage's value, raw bytes or an unchecked value does not compile.
//! Each takes a caller-owned [`Cancel`] handle: a handle already cancelled
//! returns before any charge, and a handle cancelled during the operation
//! stops it at its next meter charge, with no output (FR-276). The library
//! reads no clock.
//!
//! [`CheckedUnit`] and [`EmittedUnit`] hold the stage outputs
//! `qsl_package::CheckedPackage` and `qsl_package::EmittedPackage` with the
//! sources the unit was compiled from, which the package's replay needs and
//! the stage types do not carry.
//!
//! A stage refusal is the refusing stage's [`CompileRefusal`], boxed. The
//! composition `parse`, `select`, `check`, `package` over a source returns
//! the same bytes, stage and cause code the CLI's `compile` writes for it.
//!
//! # Wrong-stage input does not compile
//!
//! Raw source bytes are not a [`ParsedSource`], and the control with the
//! right type compiles:
//!
//! ```compile_fail
//! # use qsl_replay::spine::{check, AdmittedModels, DependencyInput, SpineLimits};
//! # use quire_exact::Cancel;
//! let _ = check(
//!     b"language \"ix:native\" edition \"1-draft\";",
//!     &AdmittedModels::default(),
//!     &DependencyInput::default(),
//!     SpineLimits::default(),
//!     &Cancel::new(),
//! );
//! ```
//!
//! ```
//! # use qsl_replay::spine::{check, AdmittedModels, DependencyInput, ParsedSource, SpineLimits};
//! # use quire_exact::Cancel;
//! fn control(parsed: &ParsedSource) {
//!     let _ = check(
//!         parsed,
//!         &AdmittedModels::default(),
//!         &DependencyInput::default(),
//!         SpineLimits::default(),
//!         &Cancel::new(),
//!     );
//! }
//! ```
//!
//! A [`ParsedSource`] is not a [`CheckedUnit`], so it cannot be emitted:
//!
//! ```compile_fail
//! # use qsl_replay::spine::{package, ParsedSource};
//! # use quire_exact::Cancel;
//! fn emit(parsed: &ParsedSource) {
//!     let _ = package(parsed, &Cancel::new());
//! }
//! ```
//!
//! ```
//! # use qsl_replay::spine::{package, CheckedUnit};
//! # use quire_exact::Cancel;
//! fn control(checked: &CheckedUnit) {
//!     let _ = package(checked, &Cancel::new());
//! }
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use qsl_cst::Limits as SourceLimits;
use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, Evaluation};
use qsl_forms::{build_unit, ParsedUnit};
use qsl_foundation::diagnostic::{InternalFault, StageFailure, Staged};
use qsl_foundation::selection::ImportSelection;
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::source::Source;
use qsl_foundation::SourceIdentity;
use qsl_package as pkg;
use qsl_package::{emit_checked, read_import_view, AdmittedPackages, Emission, Import};
use qsl_semantics::check::{resolve_profiles, AdmittedImport, AssemblyCause, PackageDeclarations};
use qsl_semantics::library::{ImportView, LibraryName};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, SelectedModel};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Cancel, Meter, ScalarLimits, Value};

use super::{region, CompileRefusal, DependencyInput, ImportRefusal, SpineLimits, VisitedImport};

/// What a front-end operation returns when it produces no output: the
/// stage's refusal, a reached limit, a cancellation or a fault.
pub type FrontEndFailure = StageFailure<Box<CompileRefusal>>;

/// The one place an operation's cancellation is decided. A handle already
/// cancelled returns before `run` makes any charge. Otherwise a charge that
/// saw the cancellation makes `run` stop early, and its result, whatever
/// the denial became on its way up, is replaced by the cancellation: an
/// operation that was cancelled returns no output.
fn stage<T>(
    cancel: &Cancel,
    run: impl FnOnce() -> Result<T, Box<CompileRefusal>>,
) -> Result<Staged<T>, FrontEndFailure> {
    if let Some(cause) = cancel.cause() {
        return Err(StageFailure::Cancelled(cause));
    }
    let result = run();
    if let Some(cause) = cancel.tripped() {
        return Err(StageFailure::Cancelled(cause));
    }
    result.map(Staged::new).map_err(StageFailure::Refused)
}

/// The stage refusal of `failure`, for a caller that made its own [`Cancel`]
/// handle and shares it with nobody, so it is never cancelled. The front end
/// reports a reached limit inside its stage refusals, so a failure that is
/// neither a refusal nor a cancellation is a broken invariant, returned as
/// the fault.
pub fn refusal_or_fault(failure: FrontEndFailure) -> Result<Box<CompileRefusal>, InternalFault> {
    match failure {
        StageFailure::Refused(refusal) => Ok(refusal),
        StageFailure::Cancelled(_) => Err(InternalFault::new(
            "spine",
            "cancelled-without-a-shared-handle",
        )),
        StageFailure::Limit(_) => Err(InternalFault::new(
            "spine",
            "front-end-limit-outside-a-stage-refusal",
        )),
    }
}

/// The request [`parse`] takes: the unit's source bytes, the identity they
/// are labelled with and the path they are displayed as. The path is only
/// displayed, never opened.
#[derive(Clone, Copy, Debug)]
pub struct ParseRequest<'a> {
    /// The source's identity.
    pub source: &'a SourceIdentity,
    /// The path the source is displayed as.
    pub path: &'a str,
    /// The source bytes.
    pub bytes: &'a [u8],
}

/// A unit read into syntax (S1) and forms (S2).
#[derive(Clone, Debug)]
pub struct ParsedSource {
    syntax: qsl_cst::ParsedSource,
    unit: ParsedUnit,
}

impl ParsedSource {
    /// The unit's source, exactly as S1 read it.
    pub fn source(&self) -> &Source {
        self.syntax.source()
    }

    /// S1's parse, whose [`LosslessCst`](qsl_cst::LosslessCst) tooling reads.
    pub fn syntax(&self) -> &qsl_cst::ParsedSource {
        &self.syntax
    }

    /// S2's forms.
    pub fn unit(&self) -> &ParsedUnit {
        &self.unit
    }
}

/// The domain packages a unit's `model` declarations selected, admitted at
/// I1, with the package documents they were admitted from.
#[derive(Clone, Debug, Default)]
pub struct AdmittedModels {
    models: Vec<SelectedModel>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
}

impl AdmittedModels {
    /// The admitted `model` declarations, in source order.
    pub fn models(&self) -> &[SelectedModel] {
        &self.models
    }
}

/// A unit checked and linked: the in-process checked package and the
/// sources it was compiled from.
#[derive(Debug)]
pub struct CheckedUnit {
    package: pkg::CheckedPackage,
    source: Source,
    libraries: Vec<Source>,
}

impl CheckedUnit {
    /// The S4 in-process package.
    pub fn package(&self) -> &pkg::CheckedPackage {
        &self.package
    }

    /// The unit's own source, exactly as S1 read it (FND-016: a caller
    /// resolving a locus over it reuses this rather than reading the same
    /// bytes again).
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Every library the check actually resolved, transitively, each
    /// exactly as its own S1 read it. Never the full dependency input: a
    /// supplied library the unit's own imports never reached is not compiled
    /// and carries no source here.
    pub fn libraries(&self) -> &[Source] {
        &self.libraries
    }
}

/// A checked package emitted as `quire.checked-package/v2` bytes, with the
/// digest-addressed sources another party needs to import or replay it.
#[derive(Clone, Debug)]
pub struct EmittedUnit {
    package: pkg::EmittedPackage,
    sources: Vec<Source>,
}

impl EmittedUnit {
    /// The v2 wire bytes with their `package_id`.
    pub fn package(&self) -> &pkg::EmittedPackage {
        &self.package
    }

    /// The source provision: the unit's source and every resolved library's,
    /// each by its own digest ([`Source::reference`]).
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
}

/// S1 and S2: read `request`'s bytes into syntax (FR-278). Refuses
/// `invalid_syntax` for a source the parser recovered from, and with S2's
/// own refusal for a unit no form represents.
pub fn parse(
    request: &ParseRequest<'_>,
    limits: SourceLimits,
    cancel: &Cancel,
) -> Result<Staged<ParsedSource>, FrontEndFailure> {
    stage(cancel, || {
        parse_unit(
            request.source.clone(),
            request.path,
            request.bytes,
            limits,
            cancel,
        )
    })
}

/// I1: admit each domain package `parsed`'s `model` declarations select,
/// from `packages`, the supplied documents by their `sha256-jcs` digest
/// (FR-056, FR-278). Refuses at the first `model` declaration that does not
/// admit.
pub fn select(
    parsed: &ParsedSource,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<Staged<AdmittedModels>, FrontEndFailure> {
    stage(cancel, || {
        let models = select_models(parsed, packages, limits)?;
        Ok(AdmittedModels {
            models,
            packages: packages.clone(),
        })
    })
}

/// S3 and S4: check and link `parsed` against `models` and the dependency
/// input (ADR-015 D-1, FR-099, FR-278). The S4 source resolution compiles
/// each library the unit's `import`s reach, against the same package input,
/// dependency input and `limits`.
pub fn check(
    parsed: &ParsedSource,
    models: &AdmittedModels,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
    stage(cancel, || {
        dependencies
            .check_unit_owner(parsed.source().identity())
            .map_err(|refusal| Box::new(CompileRefusal::DependencyInput(refusal)))?;
        let mut resolution = Resolution {
            dependencies,
            packages: &models.packages,
            limits,
            cancel,
            active: Vec::new(),
            visited: BTreeMap::new(),
            compiled: BTreeMap::new(),
            admitted: AdmittedPackages::default(),
        };
        let package = resolution.check_unit(parsed, models.models.clone())?;
        Ok(CheckedUnit {
            package,
            source: parsed.source().clone(),
            libraries: resolution
                .compiled
                .into_values()
                .map(|library| library.source.clone())
                .collect(),
        })
    })
}

/// E4: emit `checked` as `quire.checked-package/v2` bytes with its source
/// provision (FR-278). It runs no stage up to S4.
pub fn package(
    checked: &CheckedUnit,
    cancel: &Cancel,
) -> Result<Staged<EmittedUnit>, FrontEndFailure> {
    stage(cancel, || {
        let emission = emit_unit(&checked.package)?;
        let sources = std::iter::once(&checked.source)
            .chain(&checked.libraries)
            .cloned()
            .collect();
        Ok(EmittedUnit {
            package: emission.package().clone(),
            sources,
        })
    })
}

/// The request [`execute`] takes: the function, its admitted argument
/// values and the object environment it runs in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExecuteRequest<'a> {
    /// The function to call.
    pub function: &'a qsl_eval::value::QualifiedName,
    /// Its argument values, in parameter order.
    pub arguments: &'a [Value],
    /// The object environment.
    pub objects: &'a ObjectEnvironment,
}

/// S6a: call a checked function under `accounting` (FR-275). It runs no
/// stage up to S4. A cancelled handle returns
/// [`CallFailure::Cancelled`] and no value.
pub(crate) fn execute(
    package: &CheckedUnit,
    request: &ExecuteRequest<'_>,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<Evaluation, CallFailure> {
    if let Some(cause) = cancel.cause() {
        return Err(CallFailure::Cancelled(cause));
    }
    let mut meter = Meter::new(accounting).with_cancel(cancel.clone());
    let evaluation = package.package.call(
        request.function,
        request.arguments.to_vec(),
        request.objects,
        &mut meter,
    );
    if let Some(cause) = cancel.tripped() {
        return Err(CallFailure::Cancelled(cause));
    }
    evaluation
}

/// S1 then S2 over one unit's bytes: the part of every unit's compile that
/// does not depend on its imports.
fn parse_unit(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    limits: SourceLimits,
    cancel: &Cancel,
) -> Result<ParsedSource, Box<CompileRefusal>> {
    let parsed = qsl_cst::parse_with_cancel(source, path, bytes, limits, cancel)
        .map_err(|diagnostic| Box::new(CompileRefusal::Source(diagnostic)))?;
    if let Some(first) = parsed.diagnostics().first() {
        return Err(Box::new(CompileRefusal::Source(Box::new(first.clone()))));
    }
    let raw = parsed.source().reference().clone();
    let unit = build_unit(&parsed).map_err(|failure| {
        Box::new(CompileRefusal::Forms {
            region: failure.span.and_then(|span| region(&raw, span)),
            failure,
        })
    })?;
    Ok(ParsedSource {
        syntax: parsed,
        unit,
    })
}

/// I1 over one unit's `model` declarations.
fn select_models(
    parsed: &ParsedSource,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
) -> Result<Vec<SelectedModel>, Box<CompileRefusal>> {
    let raw = parsed.source().reference();
    admit_unit(&parsed.unit.selections().models, packages, limits).map_err(|refusal| {
        Box::new(CompileRefusal::Intake {
            region: region(raw, refusal.span),
            refusal,
        })
    })
}

/// E4 over one checked package, refusing a wire that would omit nodes: a
/// package missing part of the checked graph is partial output, which E4
/// never writes (ADR-011 §2.3).
fn emit_unit(package: &pkg::CheckedPackage) -> Result<Emission, Box<CompileRefusal>> {
    let emission =
        emit_checked(package).map_err(|refusal| Box::new(CompileRefusal::Emit(refusal)))?;
    if !emission.omitted().is_empty() {
        return Err(Box::new(CompileRefusal::Omitted(
            emission.omitted().to_vec(),
        )));
    }
    Ok(emission)
}

/// One unit through all four operations, over a handle nobody cancels: the
/// compile the spine's unit tests use to build their fixtures. Production
/// callers compose the operations themselves.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ComposedUnit {
    /// The S4 in-process package.
    pub(crate) package: pkg::CheckedPackage,
    /// The emitted wire bytes with their `package_id`.
    pub(crate) emitted: pkg::EmittedPackage,
    /// The unit's own source.
    pub(crate) source: Source,
}

/// [`ComposedUnit`] of `bytes` under `limits`.
#[cfg(test)]
pub(crate) fn compose(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<ComposedUnit, Box<CompileRefusal>> {
    let cancel = Cancel::new();
    let refusal = |failure| match refusal_or_fault(failure) {
        Ok(refusal) => refusal,
        Err(fault) => panic!("the front end faulted: {fault:?}"),
    };
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path,
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = select(&parsed, packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    let checked = check(&parsed, &models, dependencies, limits, &cancel)
        .map_err(refusal)?
        .into_value();
    let emitted = package(&checked, &cancel).map_err(refusal)?.into_value();
    Ok(ComposedUnit {
        emitted: emitted.package().clone(),
        source: checked.source,
        package: checked.package,
    })
}

#[cfg(test)]
mod tests;

/// A library the S4 source resolution compiled: its checked package and
/// its import view.
#[derive(Debug)]
struct ResolvedLibrary {
    package: Arc<pkg::CheckedPackage>,
    view: ImportView,
    /// This library's own source, exactly as its own `qsl_cst::parse` read
    /// it (FND-016).
    source: Source,
}

/// One compile's S4 source resolution state (ADR-015 D-1).
struct Resolution<'a> {
    dependencies: &'a DependencyInput,
    packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
    limits: SpineLimits,
    cancel: &'a Cancel,
    /// The libraries whose compile is in progress, outermost first.
    active: Vec<LibraryName>,
    /// Each identity's first import in the closure.
    visited: BTreeMap<LibraryName, VisitedImport>,
    /// Each library whose compile completed. A library is compiled at most
    /// once per compile, so the number of library compiles is at most the
    /// number of supplied libraries.
    compiled: BTreeMap<LibraryName, Arc<ResolvedLibrary>>,
    /// IR's admitted package of each library read so far, which a later
    /// library's view read takes for its closure instead of reading again.
    admitted: AdmittedPackages,
}

impl Resolution<'_> {
    /// One library through S1 to E4, resolving its own imports depth first.
    fn compile_library(
        &mut self,
        source: SourceIdentity,
        path: &str,
        bytes: &[u8],
    ) -> Result<(pkg::CheckedPackage, Emission, Source), Box<CompileRefusal>> {
        let parsed = parse_unit(source, path, bytes, self.limits.source, self.cancel)?;
        let models = select_models(&parsed, self.packages, self.limits.model)?;
        let package = self.check_unit(&parsed, models)?;
        let emission = emit_unit(&package)?;
        Ok((package, emission, parsed.source().clone()))
    }

    /// One parsed unit through S3 and S4, resolving its imports depth first.
    fn check_unit(
        &mut self,
        parsed: &ParsedSource,
        models: Vec<SelectedModel>,
    ) -> Result<pkg::CheckedPackage, Box<CompileRefusal>> {
        let limits = self.limits;
        let raw = parsed.source().reference().clone();
        let unit = parsed.unit.clone();
        let mut admitted = Vec::with_capacity(unit.selections().imports.len());
        let mut links = Vec::with_capacity(unit.selections().imports.len());
        for import in &unit.selections().imports {
            let (identity, library) = self.resolve(&raw, import)?;
            admitted.push(AdmittedImport {
                identity: identity.clone(),
                view: library.view.clone(),
                graph: library.package.shared_graph(),
            });
            links.push(Import {
                identity,
                version: import.version.clone(),
                digest: import.digest,
                package: Arc::clone(&library.package),
            });
        }
        resolve_profiles(&unit.selections().profiles).map_err(|refusals| {
            Box::new(CompileRefusal::Profile {
                region: refusals
                    .first()
                    .and_then(|refusal| region(&raw, refusal.identity_span)),
                refusals,
            })
        })?;
        let declarations = PackageDeclarations::assemble(raw.clone(), unit, models, admitted)
            .map_err(|refusal| {
                Box::new(CompileRefusal::Assembly {
                    // A type-environment stage limit names no declaration,
                    // so it has no region (FR-082, FR-096).
                    region: refusal
                        .errors
                        .first()
                        .filter(|error| !matches!(error.cause, AssemblyCause::TypeLimit(_)))
                        .and_then(|error| region(&raw, error.span)),
                    refusal,
                })
            })?;
        let regions = declarations.regions();
        let graph = declarations
            .check_with_cancel(limits.checking, self.cancel)
            .map_err(|refusals| {
                Box::new(CompileRefusal::Check {
                    region: refusals
                        .first()
                        .and_then(|refusal| regions.refusal_region(refusal)),
                    refusals,
                })
            })?;
        pkg::CheckedPackage::link_with(graph, links)
            .map_err(|refusal| Box::new(CompileRefusal::Link(refusal)))
    }

    /// ADR-015 D-1's six steps for one `import` of the unit `raw` names:
    /// cycle, diamond, selection, compile, identity, view.
    fn resolve(
        &mut self,
        raw: &RawSourceRef,
        import: &ImportSelection,
    ) -> Result<(LibraryName, Arc<ResolvedLibrary>), Box<CompileRefusal>> {
        let at_identity = || region(raw, import.identity_span);
        let at_import = || region(raw, import.span);
        let refuse = |refusal, region| Box::new(CompileRefusal::Import { refusal, region });
        // The parser admits no empty identity.
        let Ok(identity) = LibraryName::new(import.identity.as_str()) else {
            return Err(refuse(ImportRefusal::UnnamedImport, at_identity()));
        };
        // 1. Cycle, before any digest is compared.
        if let Some(start) = self.active.iter().position(|active| *active == identity) {
            let mut path = self.active[start..].to_vec();
            path.push(identity);
            return Err(refuse(ImportRefusal::Cycle { path }, at_identity()));
        }
        let mut path = self.active.clone();
        path.push(identity.clone());
        let visit = VisitedImport {
            path,
            version: import.version.clone(),
            digest: import.digest,
        };
        // 2. Diamond; an equal earlier import reuses its completed library.
        if let Some(first) = self.visited.get(&identity) {
            if first.version != visit.version || first.digest != visit.digest {
                return Err(refuse(
                    ImportRefusal::Diamond {
                        identity,
                        first: Box::new(first.clone()),
                        second: Box::new(visit),
                    },
                    at_identity(),
                ));
            }
            if let Some(library) = self.compiled.get(&identity) {
                return Ok((identity, Arc::clone(library)));
            }
        }
        self.visited.insert(identity.clone(), visit);
        // 3. Selection.
        let dependencies = self.dependencies;
        let Some(supplied) = dependencies.libraries.get(&identity) else {
            return Err(refuse(
                ImportRefusal::MissingSelection { identity },
                at_identity(),
            ));
        };
        if supplied.version != import.version {
            return Err(refuse(
                ImportRefusal::RevisionMismatch {
                    identity,
                    imported: import.version.clone(),
                    supplied: supplied.version.clone(),
                },
                at_import(),
            ));
        }
        // 4. Compile, by this same resolution, within the depth ceiling.
        if self.active.len() >= self.limits.dependencies.depth {
            let mut path = self.active.clone();
            path.push(identity);
            return Err(refuse(
                ImportRefusal::DepthLimit {
                    limit: self.limits.dependencies.depth,
                    path,
                },
                at_identity(),
            ));
        }
        self.active.push(identity.clone());
        let compiled =
            self.compile_library(supplied.source.clone(), &supplied.path, &supplied.bytes);
        self.active.pop();
        let (package, emission, library_source) =
            compiled.map_err(|refusal| wrap(&identity, refusal))?;
        // 5. Identity.
        let recompiled = emission.package().package_id();
        if !recompiled.matches(&import.digest) {
            return Err(refuse(
                ImportRefusal::DependencyIdentityMismatch {
                    identity,
                    recorded: import.digest,
                    recompiled,
                },
                at_import(),
            ));
        }
        // 6. View.
        let view = read_import_view(
            &package,
            &emission,
            identity.clone(),
            &import.version,
            self.packages,
            &mut self.admitted,
        )
        .map_err(|refusal| {
            refuse(
                ImportRefusal::View {
                    identity: identity.clone(),
                    refusal: Box::new(refusal),
                },
                at_import(),
            )
        })?;
        let library = Arc::new(ResolvedLibrary {
            package: Arc::new(package),
            view,
            source: library_source,
        });
        self.compiled.insert(identity.clone(), Arc::clone(&library));
        Ok((identity, library))
    }
}

/// `refusal`, raised compiling the library `identity`, as the importing
/// unit reports it (ADR-015 D-1): a closure-level refusal unwrapped, and
/// any other as the library's own under its dependency path.
fn wrap(identity: &LibraryName, refusal: Box<CompileRefusal>) -> Box<CompileRefusal> {
    if refusal.is_closure_level() {
        return refusal;
    }
    match *refusal {
        CompileRefusal::Dependency { mut path, refusal } => {
            path.insert(0, identity.clone());
            Box::new(CompileRefusal::Dependency { path, refusal })
        }
        refusal => Box::new(CompileRefusal::Dependency {
            path: vec![identity.clone()],
            refusal: Box::new(refusal),
        }),
    }
}
