// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-345: profile-layering refinement gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use logos::Logos;
use qsl_analyze::analyze::Engines;
use qsl_cst::token::Kind;
use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::{Code, Setting, SettingLimits, SourceIdentity};
use qsl_replay::spine::{
    self, CheckedUnit, DependencyInput, ParseRequest, RefinementClass, RefusalCause, SpineLimits,
    SuppliedLibrary,
};
use qsl_replay::Cancel;
use qsl_route::routing::Disposition;
use qsl_route::{
    BackendDescriptor, Candidate, CandidateOutcome, ItemEvidence, RegistrationRefusal, Registry,
};
use qsl_semantics::check::Capability;
use qsl_semantics::check::CheckedStateClause;
use qsl_semantics::value::{DefinitionLock, HeaderLayer, WitnessKind, WitnessRequest};
use serde::{Deserialize, Serialize};

use super::compare::CaseResult;

/// A caller-owned, immutable view of the backend registry used by one gate
/// run. It is a snapshot of FR-075 registrations; it performs no discovery.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistrySnapshot {
    registry: Registry,
}

impl RegistrySnapshot {
    /// Build a fresh snapshot from the descriptors owned by QSL's linked
    /// analyzer engines.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryBuildError`] if the linked descriptor source fails
    /// or if the registry refuses any descriptor. No partial snapshot is
    /// returned.
    pub fn production() -> Result<Self, RegistryBuildError> {
        let engines = Engines::qsl().map_err(RegistryBuildError::DescriptorSource)?;
        Self::build_registrations(engines.registrations().cloned(), |builder, descriptor| {
            builder.register(descriptor)
        })
    }

    /// Take ownership of a caller-built QSL registry as one immutable run snapshot.
    pub fn from_registry(registry: Registry) -> Self {
        Self { registry }
    }

    /// The FR-075 candidate set for one witness kind.
    pub fn candidates(&self, kind: Capability) -> CandidateOutcome {
        self.registry.candidates(kind, None)
    }

    /// The FR-075-AC-13 candidate set for one probabilistic evidence kind.
    pub fn candidates_for_evidence(
        &self,
        kind: Capability,
        evidence: &ItemEvidence,
    ) -> CandidateOutcome {
        self.registry.candidates_for_evidence(kind, evidence, None)
    }

    fn build_registrations(
        registrations: impl IntoIterator<Item = BackendDescriptor>,
        mut register: impl FnMut(
            &mut RegistrySnapshotBuilder,
            BackendDescriptor,
        ) -> Result<(), Vec<RegistrationRefusal>>,
    ) -> Result<Self, RegistryBuildError> {
        let mut builder = RegistrySnapshotBuilder::new();
        for descriptor in registrations {
            register(&mut builder, descriptor).map_err(RegistryBuildError::Registration)?;
        }
        Ok(builder.build())
    }

    #[cfg(test)]
    fn with_injected_source(
        source: impl FnOnce() -> Result<Vec<BackendDescriptor>, RegistryBuildError>,
        register: impl FnMut(
            &mut RegistrySnapshotBuilder,
            BackendDescriptor,
        ) -> Result<(), Vec<RegistrationRefusal>>,
    ) -> Result<Self, RegistryBuildError> {
        Self::build_registrations(source()?, register)
    }
}

/// Why a production provider snapshot could not be built.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RegistryBuildError {
    /// A linked engine's descriptor source refused to construct its descriptor.
    #[error("linked engine descriptor source refused: {0}")]
    DescriptorSource(#[source] RegistrationRefusal),
    /// The registry refused one of the linked engine descriptors.
    #[error("linked engine registration refused: {0:?}")]
    Registration(Vec<RegistrationRefusal>),
}

/// Build one registry snapshot from registrations supplied by the caller.
#[derive(Clone, Debug, Default)]
pub struct RegistrySnapshotBuilder {
    registry: Registry,
}

impl RegistrySnapshotBuilder {
    /// Start an empty snapshot builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Continue from a caller-built registry without discovering providers.
    pub fn from_registry(registry: Registry) -> Self {
        Self { registry }
    }

    /// Add one already-admitted backend registration (FR-075/FR-290).
    pub fn register(
        &mut self,
        descriptor: BackendDescriptor,
    ) -> Result<(), Vec<RegistrationRefusal>> {
        self.registry.register(descriptor)
    }

    /// Freeze the registrations as an immutable snapshot for a comparison.
    pub fn build(self) -> RegistrySnapshot {
        RegistrySnapshot {
            registry: self.registry,
        }
    }
}

/// One AD-003 profile-layering edge (QSpec FR-453).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edge {
    /// Stable edge label.
    pub name: &'static str,
    /// The parent layer.
    pub parent: HeaderLayer,
    /// The child layer.
    pub child: HeaderLayer,
}

/// One source descriptor in an entry file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    authority: String,
    identity: String,
}

/// One FR-099 dependency source in an entry file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencyFile {
    library: String,
    path: String,
    authority: String,
    identity: String,
}

/// The entry's exact wire shape. Kind-specific presence is checked after
/// deserialization so unknown members are rejected at the entry boundary.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryFile {
    kind: EntryKind,
    parent: Option<SourceFile>,
    child: Option<SourceFile>,
    unit: Option<SourceFile>,
    layer: Option<String>,
    packages: Vec<String>,
    dependencies: Vec<DependencyFile>,
    #[serde(default)]
    limits: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum EntryKind {
    Case,
    Distinguishing,
    Witness,
}

impl std::fmt::Display for EntryKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Case => "case",
            Self::Distinguishing => "distinguishing",
            Self::Witness => "witness",
        })
    }
}

/// One corpus input, assembled before any compile runs.
struct LoadedUnit {
    source: UnitInput,
    reference: RawSourceRef,
    layer: HeaderLayer,
}

enum LoadedEntry {
    Case {
        edge: Edge,
        parent: LoadedUnit,
        child: LoadedUnit,
    },
    Distinguishing {
        edge: Edge,
        parent: LoadedUnit,
        child: LoadedUnit,
    },
    Witness {
        layer: HeaderLayer,
        unit: LoadedUnit,
    },
}

/// One side of a compile result in the layering report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SideResult {
    /// FR-341 class.
    pub class: &'static str,
    /// Compile stage.
    pub stage: &'static str,
    /// Compile codes and causes.
    pub codes: Vec<super::report::CodeReport>,
}

/// The case identity required by FR-345: its exact sources and edge.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CaseName {
    /// Parent source reference.
    pub parent: RawSourceRef,
    /// Child source reference.
    pub child: RawSourceRef,
    /// Parent layer identity.
    pub parent_layer: String,
    /// Child layer identity.
    pub child_layer: String,
}

/// One backend's disposition for a witness item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CandidateResult {
    /// Backend identity.
    pub backend: String,
    /// Negotiated disposition.
    pub disposition: &'static str,
}

/// One witness entry's compile and candidate-negotiation outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WitnessResult {
    /// FR-341 class of the witness unit.
    pub class: &'static str,
    /// Compile refusal codes and causes.
    pub codes: Vec<super::report::CodeReport>,
    /// Candidate dispositions, ordered by backend identity.
    pub candidates: Vec<CandidateResult>,
    /// A reason such as `not-negotiated` or an empty candidate set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// This witness entry's result.
    pub result: CaseResult,
}

/// One distinguishing entry's parent/child comparison.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EdgeComparison {
    /// Parent-side compile.
    pub parent: SideResult,
    /// Child-side compile.
    pub child: SideResult,
    /// This distinguishing entry's result.
    pub result: CaseResult,
}

/// One result in a layering gate report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryResult {
    /// A case's parent/child compile comparison.
    Case {
        /// Exact case identity.
        name: Box<CaseName>,
        /// Parent compile.
        parent: SideResult,
        /// Child compile.
        child: SideResult,
        /// Comparison result.
        result: CaseResult,
    },
    /// A layer's witness check.
    Layer {
        /// Layer identity.
        layer: String,
        /// Every witness entry for the layer.
        witnesses: Vec<WitnessResult>,
        /// The aggregate layer result.
        result: CaseResult,
    },
    /// An edge's proper-subset check.
    Edge {
        /// Stable edge name.
        edge: String,
        /// Parent layer identity.
        parent_layer: String,
        /// Child layer identity.
        child_layer: String,
        /// Every distinguishing entry for the edge.
        comparisons: Vec<EdgeComparison>,
        /// The edge result holds only when every comparison holds.
        result: CaseResult,
    },
    /// A malformed or missing corpus item.
    ToolFailure {
        /// Entry, corpus or missing-item label.
        entry: String,
        /// The defect.
        defect: String,
        /// Always tool failure.
        result: CaseResult,
    },
}

impl EntryResult {
    fn verdict(&self) -> super::report::Verdict {
        match self {
            Self::Case { result, .. } | Self::Layer { result, .. } | Self::Edge { result, .. } => {
                super::report::Verdict::of(*result)
            }
            Self::ToolFailure { .. } => super::report::Verdict::ToolFailure,
        }
    }

    fn order_key(&self) -> (u8, String) {
        match self {
            Self::Case { name, .. } => (
                0,
                format!("{}:{}", name.parent.identity(), name.child.identity()),
            ),
            Self::Layer { layer, .. } => (1, layer.clone()),
            Self::Edge { edge, .. } => (2, edge.clone()),
            Self::ToolFailure { entry, .. } => (3, entry.clone()),
        }
    }
}

/// FR-344's report for FR-345.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    /// Every case, layer, edge and tool-failure result.
    pub results: Vec<EntryResult>,
    /// Highest-severity verdict.
    pub verdict: super::report::Verdict,
    /// FR-344 exit code.
    pub exit: u8,
}

impl Report {
    fn new(mut results: Vec<EntryResult>) -> Self {
        results.sort_by_key(EntryResult::order_key);
        let verdict = results
            .iter()
            .map(EntryResult::verdict)
            .max()
            .unwrap_or(super::report::Verdict::Success);
        Self {
            results,
            verdict,
            exit: verdict.exit_code(),
        }
    }

    /// The report as deterministic JSON text.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// The six edges in QSpec FR-453's lock order.
pub const EDGES: [Edge; 6] = [
    Edge {
        name: "E1",
        parent: HeaderLayer::StateCore,
        child: HeaderLayer::StateQueries,
    },
    Edge {
        name: "E2",
        parent: HeaderLayer::StateQueries,
        child: HeaderLayer::StateGraph,
    },
    Edge {
        name: "E3",
        parent: HeaderLayer::StateCore,
        child: HeaderLayer::ValueComplete,
    },
    Edge {
        name: "E4",
        parent: HeaderLayer::StateGraph,
        child: HeaderLayer::ModelComplete,
    },
    Edge {
        name: "E5",
        parent: HeaderLayer::ValueComplete,
        child: HeaderLayer::ModelComplete,
    },
    Edge {
        name: "E6",
        parent: HeaderLayer::ModelComplete,
        child: HeaderLayer::TemporalComplete,
    },
];

/// Find an edge by its exact parent and child layer pair.
pub fn edge(parent: HeaderLayer, child: HeaderLayer) -> Option<Edge> {
    EDGES
        .into_iter()
        .find(|edge| edge.parent == parent && edge.child == child)
}

/// A source unit and the compile inputs it names in one corpus entry.
pub struct UnitInput {
    /// FR-001's source labels, with the run's fixed revision labels.
    pub identity: SourceIdentity,
    /// The source path relative to its entry directory.
    pub path: String,
    /// Exact source bytes.
    pub bytes: Vec<u8>,
    /// FR-056 package documents.
    pub packages: Vec<Vec<u8>>,
    /// FR-099 dependency inputs.
    pub dependencies: DependencyInput,
    /// Compile limits, with published defaults for unstated settings.
    pub limits: SpineLimits,
}

/// One unit's compile result, passed through FR-341's classification.
pub struct Compilation {
    /// The FR-341 compile class.
    pub class: RefinementClass,
    /// Catalog codes and causes of the compile result, in order.
    pub codes: Vec<RefusalCause>,
    checked: Option<CheckedUnit>,
}

impl Compilation {
    /// A test or alternate compile seam's typed classification.
    pub fn classified(class: RefinementClass, codes: Vec<RefusalCause>) -> Self {
        Self {
            class,
            codes,
            checked: None,
        }
    }

    /// A successful FR-278 compile retaining its checked unit.
    fn admitted(checked: CheckedUnit) -> Self {
        Self {
            class: RefinementClass::Admitted,
            codes: Vec::new(),
            checked: Some(checked),
        }
    }
}

/// Run one source through FR-278's parse, select, check and package stages.
pub fn compile_unit(input: &UnitInput) -> Compilation {
    let cancellation = Cancel::new();
    let packages =
        qsl_semantics::model::intake::package_input(input.packages.iter().map(Vec::as_slice));
    let parsed = match spine::parse(
        &ParseRequest {
            source: &input.identity,
            path: &input.path,
            bytes: &input.bytes,
        },
        Default::default(),
        &cancellation,
    ) {
        Ok(parsed) => parsed.into_value(),
        Err(failure) => return classify_failure(failure),
    };
    let models = match spine::select(&parsed, &packages, Default::default(), &cancellation) {
        Ok(models) => models.into_value(),
        Err(failure) => return classify_failure(failure),
    };
    let checked = match spine::check(
        &parsed,
        &models,
        &input.dependencies,
        &qsl_semantics::check::LockEvidence::default(),
        input.limits,
        &cancellation,
    ) {
        Ok(checked) => checked.into_value(),
        Err(failure) => return classify_failure(failure),
    };
    if let Err(failure) = spine::package(&checked, Default::default(), &cancellation) {
        return classify_failure(failure);
    }
    Compilation::admitted(checked)
}

fn classify_failure(failure: spine::FrontEndFailure) -> Compilation {
    match spine::refusal_or_fault(failure) {
        Ok(refusal) => {
            let classification = spine::classify_refusal(&refusal);
            Compilation::classified(classification.class, refusal.causes())
        }
        Err(_) => Compilation::classified(RefinementClass::ToolFailure, Vec::new()),
    }
}

fn profile_identity_span(bytes: &[u8]) -> Result<(String, usize, usize), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the unit is not UTF-8".to_owned())?;
    let mut found = None;
    // Read header metadata with the language's token vocabulary. Parsing the
    // whole unit here would turn body syntax/limit results into loader faults.
    let mut tokens = Kind::lexer(text)
        .spanned()
        .filter(|(kind, _)| kind != &Ok(Kind::Comment));
    while let Some((kind, _)) = tokens.next() {
        if kind == Ok(Kind::Profile) {
            if found.is_some() {
                return Err("the unit has more than one profile declaration".to_owned());
            }
            let (
                Some((Ok(Kind::Identifier(_)), _)),
                Some((Ok(Kind::Equal), _)),
                Some((Ok(Kind::Text(identity)), span)),
                Some((Ok(Kind::Semicolon), _)),
            ) = (tokens.next(), tokens.next(), tokens.next(), tokens.next())
            else {
                return Err("the profile declaration has no quoted identity".to_owned());
            };
            found = Some((identity, span.start + 1, span.end - 1));
        }
    }
    found.ok_or_else(|| "the unit has no profile declaration".to_owned())
}

fn differs_only_by_profile_identity(parent: &[u8], child: &[u8]) -> Result<bool, String> {
    let (parent_identity, parent_start, parent_end) = profile_identity_span(parent)?;
    let (child_identity, child_start, child_end) = profile_identity_span(child)?;
    if parent_identity == child_identity {
        return Ok(false);
    }
    Ok(
        parent[..parent_start] == child[..child_start]
            && parent[parent_end..] == child[child_end..],
    )
}

fn corpus_directories(corpus: &Path) -> io::Result<Vec<PathBuf>> {
    let mut directories = fs::read_dir(corpus)?
        .filter_map(|entry| match entry {
            Ok(entry) if entry.path().is_dir() => Some(Ok(entry.path())),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<io::Result<Vec<_>>>()?;
    directories.sort();
    Ok(directories)
}

fn read_entry_file(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    let relative = Path::new(name);
    for component in relative.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!("{name} is an absolute path"));
            }
            Component::ParentDir | Component::Normal(_) | Component::CurDir => {}
        }
    }
    let canonical = root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("cannot read {name}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("{name} resolves outside the entry directory"));
    }
    fs::read(canonical).map_err(|error| format!("cannot read {name}: {error}"))
}

fn source_reference(source: &SourceFile, bytes: &[u8]) -> Result<RawSourceRef, String> {
    let digest = DigestRecord::mint(
        DigestDomain::SourceBytesV1,
        ByteDigest::of(bytes).as_bytes(),
    );
    RawSourceRef::new(source.authority.clone(), source.identity.clone(), digest)
        .map_err(|error| format!("invalid source labels: {error}"))
}

fn load_unit(
    root: &Path,
    source: SourceFile,
    packages: &[Vec<u8>],
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<LoadedUnit, String> {
    let bytes = read_entry_file(root, &source.path)?;
    let (identity, _, _) = profile_identity_span(&bytes)?;
    let layer = HeaderLayer::from_identity(&identity)
        .ok_or_else(|| format!("profile identity {identity:?} is not a header-selectable layer"))?;
    let reference = source_reference(&source, &bytes)?;
    let input = UnitInput {
        identity: SourceIdentity::new(
            source.authority,
            source.identity,
            "refinement-layering",
            "run",
        ),
        path: source.path,
        bytes,
        packages: packages.to_vec(),
        dependencies: dependencies.clone(),
        limits,
    };
    Ok(LoadedUnit {
        source: input,
        reference,
        layer,
    })
}

fn load_entry(directory: &Path) -> Result<LoadedEntry, String> {
    let root = directory
        .canonicalize()
        .map_err(|error| format!("cannot read entry directory: {error}"))?;
    let bytes = read_entry_file(&root, "entry.json")?;
    let file: EntryFile =
        serde_json::from_slice(&bytes).map_err(|error| format!("entry.json: {error}"))?;
    let packages = file
        .packages
        .iter()
        .map(|path| read_entry_file(&root, path))
        .collect::<Result<Vec<_>, _>>()?;
    let libraries = file
        .dependencies
        .into_iter()
        .map(|dependency| {
            Ok(SuppliedLibrary {
                identity: dependency.library,
                source: SourceIdentity::new(
                    dependency.authority,
                    dependency.identity,
                    "refinement-layering",
                    "run",
                ),
                path: dependency.path.clone(),
                bytes: read_entry_file(&root, &dependency.path)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let dependencies = DependencyInput::new(libraries).map_err(|error| error.to_string())?;
    let mut limits = SpineLimits::default();
    for (name, bound) in file.limits {
        let setting =
            Setting::from_name(&name).ok_or_else(|| format!("limits.{name} is not a setting"))?;
        if !limits.set_bound(setting, bound) {
            return Err(format!("limits.{name} is not a compile setting"));
        }
    }
    match file.kind {
        EntryKind::Case | EntryKind::Distinguishing => {
            let parent_file = file
                .parent
                .ok_or_else(|| format!("{} requires parent", file.kind))?;
            let child_file = file
                .child
                .ok_or_else(|| format!("{} requires child", file.kind))?;
            if file.unit.is_some() || file.layer.is_some() {
                return Err(format!("{} cannot contain unit or layer", file.kind));
            }
            let parent = load_unit(&root, parent_file, &packages, &dependencies, limits)?;
            let child = load_unit(&root, child_file, &packages, &dependencies, limits)?;
            if !differs_only_by_profile_identity(&parent.source.bytes, &child.source.bytes)? {
                return Err(
                    "parent and child differ outside the profile identity string".to_owned(),
                );
            }
            let edge = edge(parent.layer, child.layer).ok_or_else(|| {
                format!(
                    "{} and {} are not a QSpec FR-453 edge",
                    parent.layer.identity(),
                    child.layer.identity()
                )
            })?;
            if file.kind == EntryKind::Case {
                Ok(LoadedEntry::Case {
                    edge,
                    parent,
                    child,
                })
            } else {
                Ok(LoadedEntry::Distinguishing {
                    edge,
                    parent,
                    child,
                })
            }
        }
        EntryKind::Witness => {
            let unit_file = file
                .unit
                .ok_or_else(|| "witness requires unit".to_owned())?;
            let layer_name = file
                .layer
                .ok_or_else(|| "witness requires layer".to_owned())?;
            if file.parent.is_some() || file.child.is_some() {
                return Err("witness cannot contain parent or child".to_owned());
            }
            let layer = HeaderLayer::from_identity(&layer_name)
                .ok_or_else(|| format!("unknown witness layer {layer_name:?}"))?;
            let unit = load_unit(&root, unit_file, &packages, &dependencies, limits)?;
            if unit.layer != layer {
                return Err(format!(
                    "witness header names {}, expected {}",
                    unit.layer.identity(),
                    layer.identity()
                ));
            }
            Ok(LoadedEntry::Witness { layer, unit })
        }
    }
}

/// The optional FR-331 request seam. The layer's typed witness request
/// carries both capability kind and extent; each call is for one candidate.
pub type NegotiationFn<'a> =
    dyn FnMut(WitnessRequest, &Candidate, &CheckedStateClause) -> Disposition + 'a;

/// FR-345's public comparison core. A caller supplies the compile seam,
/// caller-owned registry snapshot and, optionally, one witness negotiator.
/// It writes no process streams.
pub fn compare(
    corpus: &Path,
    mut compile: impl FnMut(&UnitInput) -> Compilation,
    registry: &RegistrySnapshot,
    mut negotiation: Option<&mut NegotiationFn<'_>>,
) -> Report {
    let directories = match corpus_directories(corpus) {
        Ok(directories) if directories.is_empty() => {
            return Report::new(vec![tool_failure(
                corpus.display().to_string(),
                "the corpus holds no entry subdirectory".to_owned(),
            )]);
        }
        Ok(directories) => directories,
        Err(error) => {
            return Report::new(vec![tool_failure(
                corpus.display().to_string(),
                format!("the corpus cannot be read as a directory: {error}"),
            )]);
        }
    };

    let mut results = Vec::new();
    let mut cases = BTreeSet::new();
    let mut distinguishing = BTreeSet::new();
    let mut layer_witnesses: Vec<(HeaderLayer, Vec<WitnessResult>)> = HeaderLayer::ALL
        .into_iter()
        .map(|layer| (layer, Vec::new()))
        .collect();
    let mut edge_comparisons: Vec<(Edge, Vec<EdgeComparison>)> =
        EDGES.into_iter().map(|edge| (edge, Vec::new())).collect();
    let mut loaded = Vec::new();
    for directory in directories {
        let label = directory.join("entry.json").display().to_string();
        match load_entry(&directory) {
            Ok(entry) => loaded.push(entry),
            Err(defect) => results.push(tool_failure(label, defect)),
        }
    }

    for entry in loaded {
        match entry {
            LoadedEntry::Case {
                edge,
                parent,
                child,
            } => {
                cases.insert(edge.name);
                let parent_compile = compile(&parent.source);
                let child_compile = compile(&child.source);
                let result = compare_case(parent_compile.class, child_compile.class);
                results.push(EntryResult::Case {
                    name: Box::new(CaseName {
                        parent: parent.reference,
                        child: child.reference,
                        parent_layer: edge.parent.identity().to_owned(),
                        child_layer: edge.child.identity().to_owned(),
                    }),
                    parent: side_result(parent_compile),
                    child: side_result(child_compile),
                    result,
                });
            }
            LoadedEntry::Distinguishing {
                edge,
                parent,
                child,
            } => {
                distinguishing.insert(edge.name);
                let parent_compile = compile(&parent.source);
                let child_compile = compile(&child.source);
                let parent_prohibited = parent_compile.class == RefinementClass::Prohibited
                    && parent_compile
                        .codes
                        .iter()
                        .any(|code| code.code == Code::UnsupportedConstruct);
                let result =
                    if parent_prohibited && child_compile.class == RefinementClass::Admitted {
                        CaseResult::Holds
                    } else {
                        CaseResult::Regression
                    };
                if let Some((_, comparisons)) = edge_comparisons
                    .iter_mut()
                    .find(|(known, _)| known.name == edge.name)
                {
                    comparisons.push(EdgeComparison {
                        parent: side_result(parent_compile),
                        child: side_result(child_compile),
                        result,
                    });
                }
            }
            LoadedEntry::Witness { layer, unit } => {
                if let Some((_, witnesses)) = layer_witnesses
                    .iter_mut()
                    .find(|(known, _)| *known == layer)
                {
                    witnesses.push(witness_result(
                        layer,
                        compile(&unit.source),
                        registry,
                        &mut negotiation,
                    ));
                }
            }
        }
    }

    for edge in EDGES {
        if !cases.contains(edge.name) {
            results.push(tool_failure(
                format!("{}/case", edge.name),
                format!("edge {} has no case entry", edge.name),
            ));
        }
        if !distinguishing.contains(edge.name) {
            results.push(tool_failure(
                format!("{}/distinguishing", edge.name),
                format!("edge {} has no distinguishing entry", edge.name),
            ));
        }
        if let Some((_, comparisons)) = edge_comparisons
            .iter()
            .find(|(known, _)| known.name == edge.name)
        {
            if !comparisons.is_empty() {
                let result = if comparisons
                    .iter()
                    .all(|comparison| comparison.result == CaseResult::Holds)
                {
                    CaseResult::Holds
                } else {
                    CaseResult::Regression
                };
                results.push(EntryResult::Edge {
                    edge: edge.name.to_owned(),
                    parent_layer: edge.parent.identity().to_owned(),
                    child_layer: edge.child.identity().to_owned(),
                    comparisons: comparisons.clone(),
                    result,
                });
            }
        }
    }
    for layer in HeaderLayer::ALL {
        let witnesses = layer_witnesses
            .iter()
            .find(|(known, _)| *known == layer)
            .map(|(_, witnesses)| witnesses)
            .expect("every header layer has a witness bucket");
        if witnesses.is_empty() {
            results.push(tool_failure(
                format!("{}/witness", layer.identity()),
                format!("layer {} has no witness entry", layer.identity()),
            ));
        } else {
            results.push(EntryResult::Layer {
                layer: layer.identity().to_owned(),
                result: aggregate_layer_result(witnesses),
                witnesses: witnesses.clone(),
            });
        }
    }
    Report::new(results)
}

/// Run the FR-345 gate with QSL's compile composition and a caller-owned
/// provider snapshot, without witness negotiation.
pub fn run(corpus: &Path, registry: &RegistrySnapshot) -> Report {
    compare(corpus, compile_unit, registry, None)
}

fn tool_failure(entry: String, defect: String) -> EntryResult {
    EntryResult::ToolFailure {
        entry,
        defect,
        result: CaseResult::ToolFailure,
    }
}

fn side_result(compilation: Compilation) -> SideResult {
    SideResult {
        class: super::report::class_name(compilation.class),
        stage: "compile",
        codes: compilation
            .codes
            .iter()
            .map(super::report::code_report)
            .collect(),
    }
}

fn witness_result(
    layer: HeaderLayer,
    compilation: Compilation,
    registry: &RegistrySnapshot,
    negotiation: &mut Option<&mut NegotiationFn<'_>>,
) -> WitnessResult {
    if compilation.class != RefinementClass::Admitted {
        return WitnessResult {
            class: super::report::class_name(compilation.class),
            codes: compilation
                .codes
                .iter()
                .map(super::report::code_report)
                .collect(),
            candidates: Vec::new(),
            reason: Some("witness unit did not compile as admitted".to_owned()),
            result: CaseResult::Regression,
        };
    }
    let Some(checked) = compilation.checked else {
        return WitnessResult {
            class: "admitted",
            codes: compilation
                .codes
                .iter()
                .map(super::report::code_report)
                .collect(),
            candidates: Vec::new(),
            reason: Some("compile seam returned no checked unit".to_owned()),
            result: CaseResult::ToolFailure,
        };
    };
    let graph = checked.package().graph();
    // The witness item is the unit's one `invariant` clause (QSpec
    // FR-453-AC-8): the temporal witness also holds its guard and reset as
    // `pre` and `post` state clauses.
    let invariants: Vec<&CheckedStateClause> = graph
        .state_clauses()
        .iter()
        .filter(|clause| clause.is_invariant())
        .collect();
    let [clause] = invariants[..] else {
        return WitnessResult {
            class: "admitted",
            codes: compilation
                .codes
                .iter()
                .map(super::report::code_report)
                .collect(),
            candidates: Vec::new(),
            reason: Some(format!(
                "witness unit has {} compiled invariant clauses; expected exactly one",
                invariants.len()
            )),
            result: CaseResult::ToolFailure,
        };
    };
    let witness_request = DefinitionLock::pinned().layer_row(layer).witness_request;
    let capability = match witness_request.kind {
        WitnessKind::OperationContract => Capability::OperationContract,
        WitnessKind::ValueValidity => Capability::ValueValidity,
    };
    let candidate_set = match registry.candidates(capability) {
        CandidateOutcome::Candidates(candidate_set) => candidate_set,
        CandidateOutcome::UnknownBackend(_) => {
            return WitnessResult {
                class: "admitted",
                codes: compilation
                    .codes
                    .iter()
                    .map(super::report::code_report)
                    .collect(),
                candidates: Vec::new(),
                reason: Some("unnamed candidate lookup returned an unknown backend".to_owned()),
                result: CaseResult::ToolFailure,
            };
        }
    };
    if candidate_set.is_empty() {
        return WitnessResult {
            class: "admitted",
            codes: compilation
                .codes
                .iter()
                .map(super::report::code_report)
                .collect(),
            candidates: Vec::new(),
            reason: Some("empty candidate set".to_owned()),
            result: CaseResult::Regression,
        };
    }
    let Some(negotiation) = negotiation.as_deref_mut() else {
        let candidates = candidate_set
            .candidates()
            .iter()
            .map(|candidate| CandidateResult {
                backend: candidate.id().as_str().to_owned(),
                disposition: "not-negotiated",
            })
            .collect();
        return WitnessResult {
            class: "admitted",
            codes: compilation
                .codes
                .iter()
                .map(super::report::code_report)
                .collect(),
            candidates,
            reason: Some("not-negotiated".to_owned()),
            result: CaseResult::UnresolvedUnsupported,
        };
    };
    let mut supported = false;
    let candidates = candidate_set
        .candidates()
        .iter()
        .map(|candidate| {
            let disposition = negotiation(witness_request, candidate, clause);
            let (name, supports) = match &disposition {
                Disposition::Supported(target) if target.id() == candidate.id() => {
                    ("supported", true)
                }
                Disposition::Supported(_) => ("supported-other-backend", false),
                Disposition::RequiresBound => ("requires-bound", false),
                Disposition::Unsupported => ("unsupported", false),
                Disposition::InvalidRequest => ("invalid-request", false),
            };
            supported |= supports;
            CandidateResult {
                backend: candidate.id().as_str().to_owned(),
                disposition: name,
            }
        })
        .collect();
    WitnessResult {
        class: "admitted",
        codes: compilation
            .codes
            .iter()
            .map(super::report::code_report)
            .collect(),
        candidates,
        reason: None,
        result: if supported {
            CaseResult::Holds
        } else {
            CaseResult::Regression
        },
    }
}

fn aggregate_layer_result(witnesses: &[WitnessResult]) -> CaseResult {
    witnesses
        .iter()
        .map(|witness| super::report::Verdict::of(witness.result))
        .max()
        .map(|verdict| match verdict {
            super::report::Verdict::Success => CaseResult::Holds,
            super::report::Verdict::Violation => CaseResult::Regression,
            super::report::Verdict::Incomplete => CaseResult::UnresolvedIncomplete,
            super::report::Verdict::Unsupported => CaseResult::UnresolvedUnsupported,
            super::report::Verdict::ToolFailure => CaseResult::ToolFailure,
        })
        .unwrap_or(CaseResult::ToolFailure)
}

/// FR-345 / QSpec FR-453: compare the compile classes of an edge case.
pub fn compare_case(parent: RefinementClass, child: RefinementClass) -> CaseResult {
    use CaseResult::{
        Holds, NotApplicable, Regression, ToolFailure, UnresolvedIncomplete, UnresolvedUnsupported,
    };
    use RefinementClass::{
        Admitted, Incomplete, Prohibited, Refused, ToolFailure as Failed, Unsupported,
    };

    match (parent, child) {
        (Failed, _) | (_, Failed) => ToolFailure,
        (Refused, Refused) => Holds,
        (Refused, Admitted | Prohibited) => Regression,
        (Refused, Unsupported) => UnresolvedUnsupported,
        (Refused, Incomplete) | (Incomplete, _) => UnresolvedIncomplete,
        _ => NotApplicable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    use qsl_foundation::bound::DomainKind;
    use qsl_foundation::digest::ByteDigest;
    use qsl_foundation::BackendId;
    use qsl_route::EvidenceKind;
    use qsl_route::{BackendDescriptor, Candidate, ManifestDigest, Mode, ProviderOrigin};
    use qsl_semantics::check::Capability;
    use serde_json::json;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;
    use CaseResult::{
        Holds as H, NotApplicable as N, Regression as R, ToolFailure as T,
        UnresolvedIncomplete as I, UnresolvedUnsupported as U,
    };
    use RefinementClass::{
        Absent as Z, Admitted as A, Incomplete as IClass, Prohibited as P, Refused as F,
        ToolFailure as TClass, Unsupported as UClass,
    };

    /// FR-345-AC-1 (TC-867): each literal is the expected row result, not a
    /// result derived by calling the comparison implementation.
    #[trace("TC-867", "FR-345-AC-1")]
    #[test]
    fn every_parent_and_child_class_pair_matches_the_normative_table() {
        let table: [(RefinementClass, [CaseResult; 7]); 7] = [
            (F, [H, R, R, U, I, T, N]),
            (A, [N, N, N, N, N, T, N]),
            (P, [N, N, N, N, N, T, N]),
            (UClass, [N, N, N, N, N, T, N]),
            (IClass, [I, I, I, I, I, T, I]),
            (TClass, [T, T, T, T, T, T, T]),
            (Z, [N, N, N, N, N, T, N]),
        ];
        let children = [F, A, P, UClass, IClass, TClass, Z];
        for (parent, expected) in table {
            for (child, expected) in children.into_iter().zip(expected) {
                assert_eq!(
                    compare_case(parent, child),
                    expected,
                    "{parent:?} x {child:?}"
                );
            }
        }
    }

    /// FR-345-AC-3 (TC-867): a parent prohibition does not constrain the child.
    #[trace("TC-867", "FR-345-AC-3")]
    #[test]
    fn a_parent_prohibition_followed_by_admission_is_not_applicable() {
        assert_eq!(compare_case(P, A), N);
    }

    /// FR-345's registry seam is a value assembled by its caller; building a
    /// snapshot neither discovers providers nor changes FR-075 routing.
    #[trace("TC-868")]
    #[test]
    fn the_snapshot_builder_preserves_registered_witness_candidates() {
        let descriptor = BackendDescriptor::new(
            Candidate::new(BackendId::new("backend-a")),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest(ByteDigest::of(b"manifest").as_bytes()),
            [(Capability::OperationContract, Mode::Bounded)],
            Some(BTreeSet::from([DomainKind::Collection])),
            None,
        )
        .expect("well-formed backend");
        let mut builder = RegistrySnapshotBuilder::new();
        builder.register(descriptor).expect("registration admitted");
        let snapshot = builder.build();
        let CandidateOutcome::Candidates(candidates) =
            snapshot.candidates(Capability::OperationContract)
        else {
            panic!("registered backend gives a candidate set");
        };
        assert_eq!(candidates.candidates().len(), 1);
        assert_eq!(candidates.candidates()[0].id().as_str(), "backend-a");
    }

    fn refusal() -> RegistrationRefusal {
        qsl_route::descriptor_from_manifest(
            Candidate::new(BackendId::new("injected-invalid")),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest([0; 32]),
            [(Some("unknown-kind"), Some("bounded"))],
            qsl_route::ManifestMembers::default(),
        )
        .expect_err("unknown capability labels are refused")
    }

    /// FR-489-AC-1/2/4/5: the production snapshot contains exactly the real
    /// EN-1, EN-4 and EN-6 registrations and the two witness arms.
    #[trace("TC-1272", "FR-489-AC-1", "FR-489-AC-2", "FR-489-AC-4", "FR-489-AC-5")]
    #[test]
    fn production_snapshot_contains_only_linked_engine_registrations() {
        let first = RegistrySnapshot::production().expect("linked registry builds");
        let second = RegistrySnapshot::production().expect("linked registry builds again");

        assert_eq!(first, second);
        let CandidateOutcome::Candidates(temporal) =
            first.candidates(Capability::TemporalSatisfaction)
        else {
            panic!("EN-1 and EN-6 give a temporal candidate set");
        };
        assert_eq!(
            temporal
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
            ["en-1", "en-6"]
        );

        let engines = Engines::qsl().expect("linked engine descriptors are admitted");
        let registrations: Vec<_> = engines.registrations().collect();
        assert_eq!(registrations[1].id().as_str(), "en-4");
        assert_eq!(
            registrations[1].evidence_kinds(),
            &BTreeSet::from([EvidenceKind::Statistical])
        );
        let CandidateOutcome::Candidates(probabilistic) = first.candidates_for_evidence(
            Capability::ProbabilisticSatisfaction,
            &ItemEvidence::Kind(EvidenceKind::Statistical),
        ) else {
            panic!("EN-4 gives an evidence-qualified candidate set");
        };
        assert_eq!(
            probabilistic
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
            ["en-4"]
        );
        let CandidateOutcome::Candidates(value) = first.candidates(Capability::ValueValidity)
        else {
            panic!("the value-validity arm gives a candidate set");
        };
        assert_eq!(
            value
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
            ["value-checker"]
        );
        let CandidateOutcome::Candidates(contract) =
            first.candidates(Capability::OperationContract)
        else {
            panic!("the operation-contract arm gives a candidate set");
        };
        assert_eq!(
            contract
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
            ["contract-checker"]
        );
        assert_eq!(
            registrations
                .iter()
                .map(|descriptor| descriptor.id().as_str())
                .collect::<Vec<_>>(),
            ["en-1", "en-4", "en-6", "value-checker", "contract-checker"]
        );
    }

    /// FR-489-AC-3/6: a source failure or refused registration returns a
    /// typed error rather than a partially populated snapshot.
    #[trace("TC-1272", "FR-489-AC-3", "FR-489-AC-6")]
    #[test]
    fn injected_registry_failures_never_return_a_partial_snapshot() {
        let source_refusal = refusal();
        let source_result = RegistrySnapshot::with_injected_source(
            || Err(RegistryBuildError::DescriptorSource(source_refusal.clone())),
            |builder, descriptor| builder.register(descriptor),
        );
        assert!(matches!(
            source_result,
            Err(RegistryBuildError::DescriptorSource(found)) if found == source_refusal
        ));

        let descriptors: Vec<_> = Engines::qsl()
            .expect("linked engines build")
            .registrations()
            .cloned()
            .collect();
        let registration_refusal = refusal();
        let registration_result = RegistrySnapshot::with_injected_source(
            || Ok(descriptors),
            |builder, descriptor| {
                if descriptor.id().as_str() == "en-4" {
                    Err(vec![registration_refusal.clone()])
                } else {
                    builder.register(descriptor)
                }
            },
        );
        assert!(matches!(
            registration_result,
            Err(RegistryBuildError::Registration(found)) if found == vec![registration_refusal]
        ));
    }

    /// With a caller-supplied candidate, an admitted witness stays unresolved
    /// when the caller has not supplied a negotiation function.
    #[trace("TC-868")]
    #[test]
    fn an_admitted_witness_with_candidates_is_unresolved_without_negotiation() {
        let temp = crate::test_support::tempdir();
        let root = temp.path().join("state-core-witness");
        fs::create_dir_all(&root).unwrap();
        let source = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.state.core/v1\";\nmodel Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:@DIGEST@\";\ninvariant One using v on Config::ConfigVersion at current { true }\n";
        let package = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask manifest has a workspace root")
                .join("examples/config-version/model.semantic-ir.json"),
        )
        .unwrap();
        // The model's `sha256-jcs` digest, computed from the document: the
        // SHA-256 of its RFC 8785 encoding.
        let digest = {
            let parsed: serde_json::Value = serde_json::from_slice(&package).unwrap();
            let canonical = serde_json_canonicalizer::to_vec(&parsed).unwrap();
            format!("{:x}", ByteDigest::of(&canonical))
        };
        fs::write(
            root.join("witness.native"),
            source.replace("@DIGEST@", &digest),
        )
        .unwrap();
        fs::write(root.join("model.semantic-ir.json"), &package).unwrap();
        let entry = json!({
            "kind": "witness",
            "unit": {
                "path": "witness.native",
                "authority": "agent-ix",
                "identity": "ix://test/layering/state-core-witness"
            },
            "layer": "quire.state.core/v1",
            "packages": ["model.semantic-ir.json"],
            "dependencies": []
        });
        fs::write(root.join("entry.json"), serde_json::to_vec(&entry).unwrap()).unwrap();

        let descriptor = BackendDescriptor::new(
            Candidate::new(BackendId::new("backend-a")),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest(ByteDigest::of(b"manifest").as_bytes()),
            [
                (Capability::OperationContract, Mode::Bounded),
                (Capability::ValueValidity, Mode::Bounded),
            ],
            Some(BTreeSet::from([DomainKind::Collection])),
            None,
        )
        .expect("well-formed backend");
        let mut builder = RegistrySnapshotBuilder::new();
        builder.register(descriptor).expect("registration admitted");
        let report = compare(temp.path(), compile_unit, &builder.build(), None);
        let witness = report
            .results
            .iter()
            .find_map(|result| match result {
                EntryResult::Layer {
                    layer,
                    witnesses,
                    result,
                } if layer == "quire.state.core/v1" => Some((witnesses, result)),
                _ => None,
            })
            .expect("state core layer has one witness result");
        assert_eq!(witness.0.len(), 1);
        assert_eq!(witness.0[0].class, "admitted");
        assert_eq!(witness.0[0].reason.as_deref(), Some("not-negotiated"));
        assert_eq!(witness.0[0].candidates[0].backend, "backend-a");
        assert_eq!(witness.0[0].result, U);
        assert_eq!(*witness.1, U);
    }

    /// The edge table is exactly the six lock rows' direct `requires` edges.
    #[trace("TC-868", "FR-345-AC-4", "FR-345-AC-7")]
    #[test]
    fn the_edge_table_has_the_six_fr453_pairs() {
        let lock = DefinitionLock::pinned();
        let expected: Vec<(HeaderLayer, HeaderLayer)> = HeaderLayer::ALL
            .into_iter()
            .flat_map(|child| {
                lock.layer_row(child)
                    .requires
                    .iter()
                    .copied()
                    .map(move |parent| (parent, child))
            })
            .collect();
        assert_eq!(EDGES.len(), expected.len());
        assert_eq!(
            EDGES
                .iter()
                .map(|item| (item.parent, item.child))
                .collect::<Vec<_>>(),
            expected
        );
        assert!(edge(HeaderLayer::StateCore, HeaderLayer::ModelComplete).is_none());
    }

    /// FR-345-AC-4/7 (TC-868): every edge gets a case and a distinguishing
    /// entry, and the core compares both sides supplied by its compile seam.
    #[trace("TC-868", "FR-345-AC-4", "FR-345-AC-7")]
    #[test]
    fn the_core_compares_cases_and_distinguishing_entries_on_all_edges() {
        let temp = crate::test_support::tempdir();
        let corpus = temp.path();
        for item in EDGES {
            write_pair(
                corpus,
                &format!("{}-case", item.name),
                "case",
                item,
                "parent-case.native",
                "child-case.native",
                "invariant Broken using v on Config::ConfigVersion at current { self.missing > 0 }\n",
            );
            write_pair(
                corpus,
                &format!("{}-distinguishing", item.name),
                "distinguishing",
                item,
                "parent-distinguishing.native",
                "child-distinguishing.native",
                "invariant Same using v on Config::ConfigVersion at current { true }\n",
            );
        }
        for layer in HeaderLayer::ALL {
            write_witness(
                corpus,
                layer,
                &format!("{}-witness", layer.identity().replace('/', "-")),
            );
        }

        let mut calls = 0;
        let report = compare(
            corpus,
            |input| {
                calls += 1;
                let class = match input.path.as_str() {
                    "parent-case.native" | "child-case.native" => F,
                    "parent-distinguishing.native" => P,
                    "child-distinguishing.native" => A,
                    "witness.native" => F,
                    other => panic!("unexpected source path {other}"),
                };
                let codes = if input.path == "parent-distinguishing.native" {
                    vec![RefusalCause {
                        code: Code::UnsupportedConstruct,
                        cause: Some("declaration-form"),
                        limit: None,
                    }]
                } else {
                    Vec::new()
                };
                Compilation::classified(class, codes)
            },
            &RegistrySnapshot::default(),
            None,
        );

        assert_eq!(calls, 30);
        assert_eq!(report.verdict, super::super::report::Verdict::Violation);
        assert_eq!(report.exit, 10);
        let cases: Vec<_> = report
            .results
            .iter()
            .filter_map(|result| match result {
                EntryResult::Case { result, .. } => Some(*result),
                _ => None,
            })
            .collect();
        assert_eq!(cases, vec![H; 6]);
        let edges: Vec<_> = report
            .results
            .iter()
            .filter_map(|result| match result {
                EntryResult::Edge {
                    edge,
                    parent_layer,
                    child_layer,
                    comparisons,
                    result,
                } => {
                    let expected = EDGES.iter().find(|item| item.name == edge).unwrap();
                    assert_eq!(parent_layer, expected.parent.identity());
                    assert_eq!(child_layer, expected.child.identity());
                    assert_eq!(comparisons.len(), 1);
                    Some(*result)
                }
                _ => None,
            })
            .collect();
        assert_eq!(edges, vec![H; 6]);
        assert_eq!(report.results.len(), 18);
    }

    /// FR-345-AC-2 (TC-867): a typed parent refusal followed by child
    /// admission is reported with both raw source references and parent codes.
    #[trace("TC-867", "FR-345-AC-2")]
    #[test]
    fn a_case_regression_names_both_sources_edge_layers_and_parent_code() {
        let temp = crate::test_support::tempdir();
        write_pair(
            temp.path(),
            "typed-refusal",
            "case",
            EDGES[0],
            "parent.native",
            "child.native",
            "invariant Broken using v on Config::ConfigVersion at current { true }\n",
        );
        let report = compare(
            temp.path(),
            |input| match input.path.as_str() {
                "parent.native" => Compilation::classified(
                    F,
                    vec![RefusalCause {
                        code: Code::UnsupportedConstruct,
                        cause: Some("declaration-form"),
                        limit: None,
                    }],
                ),
                "child.native" => Compilation::classified(A, Vec::new()),
                other => panic!("unexpected source path {other}"),
            },
            &RegistrySnapshot::default(),
            None,
        );
        let case = report
            .results
            .iter()
            .find_map(|result| match result {
                EntryResult::Case {
                    name,
                    parent,
                    child,
                    result,
                } => Some((name, parent, child, result)),
                _ => None,
            })
            .expect("the valid case is reported");
        assert_eq!(*case.3, R);
        assert_eq!(case.0.parent.identity(), "ix://test/layering/typed-refusal");
        assert_eq!(case.0.child.identity(), "ix://test/layering/typed-refusal");
        assert_eq!(case.0.parent_layer, EDGES[0].parent.identity());
        assert_eq!(case.0.child_layer, EDGES[0].child.identity());
        assert_eq!(case.1.class, "refused");
        assert_eq!(case.1.codes[0].code, "unsupported_construct");
        assert_eq!(case.2.class, "admitted");
        assert_eq!(
            report.exit, 30,
            "missing corpus entries also cause tool failure"
        );
    }

    /// FR-345-AC-8 (TC-868): missing required entries each produce a named
    /// tool failure, and valid entries elsewhere still reach the compile seam.
    #[trace("TC-868", "FR-345-AC-8")]
    #[test]
    fn missing_state_graph_witness_and_e5_distinguishing_are_reported_once() {
        let temp = crate::test_support::tempdir();
        for item in EDGES {
            write_pair(
                temp.path(),
                &format!("{}-case", item.name),
                "case",
                item,
                "parent.native",
                "child.native",
                "invariant I using v on Config::ConfigVersion at current { true }\n",
            );
            if item.name != "E5" {
                write_pair(
                    temp.path(),
                    &format!("{}-distinguishing", item.name),
                    "distinguishing",
                    item,
                    "parent.native",
                    "child.native",
                    "invariant I using v on Config::ConfigVersion at current { true }\n",
                );
            }
        }
        for layer in HeaderLayer::ALL {
            if layer != HeaderLayer::StateGraph {
                write_witness(
                    temp.path(),
                    layer,
                    &format!("{}-witness", layer.identity().replace('/', "-")),
                );
            }
        }
        let report = compare(
            temp.path(),
            |input| match input.path.as_str() {
                "parent.native" => Compilation::classified(P, Vec::new()),
                "child.native" => Compilation::classified(A, Vec::new()),
                "witness.native" => Compilation::classified(F, Vec::new()),
                other => panic!("unexpected source path {other}"),
            },
            &RegistrySnapshot::default(),
            None,
        );
        let failures: Vec<_> = report
            .results
            .iter()
            .filter_map(|result| match result {
                EntryResult::ToolFailure { entry, .. } => Some(entry.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(failures.len(), 2);
        assert!(failures.contains(&"E5/distinguishing"));
        assert!(failures.contains(&"quire.state.graph/v1/witness"));
        assert_eq!(report.exit, 30);
    }

    /// FR-345-AC-5 (TC-868): parent and child bytes may differ only at the
    /// profile identity string.
    #[trace("TC-868", "FR-345-AC-5")]
    #[test]
    fn a_case_pair_with_another_byte_change_is_a_tool_failure() {
        let temp = crate::test_support::tempdir();
        let root = write_pair(
            temp.path(),
            "bad-pair",
            "case",
            EDGES[0],
            "parent.native",
            "child.native",
            "invariant Broken using v on Config::ConfigVersion at current { true }\n",
        );
        let child_path = root.join("child.native");
        let child = fs::read_to_string(&child_path).unwrap();
        fs::write(child_path, child.replace("Broken", "Changed")).unwrap();

        let mut called = false;
        let report = compare(
            temp.path(),
            |_| {
                called = true;
                Compilation::classified(A, Vec::new())
            },
            &RegistrySnapshot::default(),
            None,
        );
        assert!(!called);
        let (entry, defect) = report
            .results
            .iter()
            .find_map(|result| match result {
                EntryResult::ToolFailure { entry, defect, .. }
                    if entry.ends_with("bad-pair/entry.json") =>
                {
                    Some((entry, defect))
                }
                _ => None,
            })
            .expect("malformed pair is reported by its entry.json path");
        assert!(entry.ends_with("bad-pair/entry.json"));
        assert!(defect.contains("differ outside"));
    }

    /// FR-345-AC-8 (TC-868): an empty corpus produces one tool failure.
    #[trace("TC-868", "FR-345-AC-8")]
    #[test]
    fn an_empty_corpus_produces_one_tool_failure() {
        let temp = crate::test_support::tempdir();
        let report = compare(
            temp.path(),
            |_| panic!("the empty corpus cannot compile a unit"),
            &RegistrySnapshot::default(),
            None,
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.exit, 30);
        assert!(matches!(report.results[0], EntryResult::ToolFailure { .. }));
    }

    /// FR-345-AC-8 (TC-868): a malformed entry is named by its entry file and
    /// does not prevent the other entry from compiling.
    #[trace("TC-868", "FR-345-AC-8")]
    #[test]
    fn malformed_entry_fields_are_refused_without_stopping_other_entries() {
        let temp = crate::test_support::tempdir();
        let malformed = temp.path().join("malformed");
        fs::create_dir(&malformed).unwrap();
        fs::write(
            malformed.join("entry.json"),
            r#"{"kind":"witness","expected":true}"#,
        )
        .unwrap();
        write_witness(temp.path(), HeaderLayer::StateCore, "valid-witness");

        let mut calls = 0;
        let report = compare(
            temp.path(),
            |_| {
                calls += 1;
                Compilation::classified(F, Vec::new())
            },
            &RegistrySnapshot::default(),
            None,
        );
        assert_eq!(calls, 1);
        let (entry, defect) = report
            .results
            .iter()
            .find_map(|result| match result {
                EntryResult::ToolFailure { entry, defect, .. }
                    if entry.ends_with("malformed/entry.json") =>
                {
                    Some((entry, defect))
                }
                _ => None,
            })
            .expect("malformed entry is reported");
        assert!(entry.ends_with("malformed/entry.json"));
        assert!(defect.contains("expected"));
    }

    fn write_pair(
        corpus: &Path,
        directory: &str,
        kind: &str,
        edge: Edge,
        parent_path: &str,
        child_path: &str,
        declarations: &str,
    ) -> std::path::PathBuf {
        let root = corpus.join(directory);
        fs::create_dir_all(&root).unwrap();
        let parent_source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"{}\";\n{declarations}",
            edge.parent.identity()
        );
        let child_source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"{}\";\n{declarations}",
            edge.child.identity()
        );
        fs::write(root.join(parent_path), parent_source).unwrap();
        fs::write(root.join(child_path), child_source).unwrap();
        let entry = json!({
            "kind": kind,
            "parent": {
                "path": parent_path,
                "authority": "agent-ix",
                "identity": format!("ix://test/layering/{directory}")
            },
            "child": {
                "path": child_path,
                "authority": "agent-ix",
                "identity": format!("ix://test/layering/{directory}")
            },
            "packages": [],
            "dependencies": []
        });
        fs::write(root.join("entry.json"), serde_json::to_vec(&entry).unwrap()).unwrap();
        root
    }

    fn write_witness(corpus: &Path, layer: HeaderLayer, directory: &str) {
        let root = corpus.join(directory);
        fs::create_dir_all(&root).unwrap();
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"{}\";\ninvariant I using v {{ true }}\n",
            layer.identity()
        );
        fs::write(root.join("witness.native"), source).unwrap();
        let entry = json!({
            "kind": "witness",
            "unit": {
                "path": "witness.native",
                "authority": "agent-ix",
                "identity": format!("ix://test/layering/{directory}")
            },
            "layer": layer.identity(),
            "packages": [],
            "dependencies": []
        });
        fs::write(root.join("entry.json"), serde_json::to_vec(&entry).unwrap()).unwrap();
    }

    #[trace("TC-868", "FR-345-AC-5")]
    #[test]
    fn a_case_pair_may_change_only_the_profile_identity_string() {
        let parent = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.state.core/v1\";\ninvariant I using v { true }\n";
        let child = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.state.queries/v1\";\ninvariant I using v { true }\n";
        assert!(differs_only_by_profile_identity(parent.as_bytes(), child.as_bytes()).unwrap());
        let changed = child.replace("invariant I", "invariant J");
        assert!(!differs_only_by_profile_identity(parent.as_bytes(), changed.as_bytes()).unwrap());
    }

    #[trace("TC-868", "FR-345-AC-5")]
    #[test]
    fn profile_metadata_uses_tokens_without_admitting_the_body() {
        let text = "// profile ignored = \"not-a-profile\";\nprofile\n v // identity\n =\n \"quire.state.core/v1\";\ninvariant Broken { @ }";
        let (identity, start, end) = profile_identity_span(text.as_bytes()).unwrap();
        assert_eq!(identity, "quire.state.core/v1");
        assert_eq!(&text[start..end], identity);
        assert!(profile_identity_span(b"// profile v = \"ignored\";\n").is_err());
        assert!(profile_identity_span(b"profile v = \"one\"; profile w = \"two\";").is_err());

        let temp = crate::test_support::tempdir();
        let root = write_pair(
            temp.path(),
            "token-profile",
            "case",
            EDGES[0],
            "parent.native",
            "child.native",
            "invariant I using v { @ }\n",
        );
        for name in ["parent.native", "child.native"] {
            let path = root.join(name);
            let source = fs::read_to_string(&path)
                .unwrap()
                .replace("profile v =", "profile\n v // identity\n =");
            fs::write(path, source).unwrap();
        }
        let child_path = root.join("child.native");
        let child = fs::read_to_string(&child_path)
            .unwrap()
            .replace("invariant I", "invariant J");
        fs::write(child_path, child).unwrap();
        let report = compare(
            temp.path(),
            |_| panic!("mismatched pair must not compile"),
            &RegistrySnapshot::default(),
            None,
        );
        assert!(report.results.iter().any(|result| matches!(result,
            EntryResult::ToolFailure { entry, defect, .. }
                if entry.ends_with("token-profile/entry.json")
                    && defect == "parent and child differ outside the profile identity string"
        )));
        assert_eq!(report.verdict, super::super::report::Verdict::ToolFailure);
        assert_eq!(report.exit, 30);
    }

    #[trace("TC-868", "FR-345-AC-7")]
    #[test]
    fn distinguishing_checks_the_typed_code_not_the_cause_text() {
        for (code, expected) in [
            (Code::UnsupportedConstruct, CaseResult::Holds),
            (Code::IllTyped, CaseResult::Regression),
        ] {
            let temp = crate::test_support::tempdir();
            write_pair(
                temp.path(),
                "typed-code",
                "distinguishing",
                EDGES[0],
                "parent.native",
                "child.native",
                "invariant I using v { true }\n",
            );
            let report = compare(
                temp.path(),
                |input| {
                    if input.path == "parent.native" {
                        Compilation::classified(
                            P,
                            vec![RefusalCause {
                                code,
                                cause: Some("unsupported_construct"),
                                limit: None,
                            }],
                        )
                    } else {
                        Compilation::classified(A, Vec::new())
                    }
                },
                &RegistrySnapshot::default(),
                None,
            );
            let comparison = report
                .results
                .iter()
                .find_map(|result| match result {
                    EntryResult::Edge { comparisons, .. } => comparisons.first(),
                    _ => None,
                })
                .expect("the populated edge");
            assert_eq!(comparison.result, expected);
            assert_eq!(comparison.parent.codes[0].code, code.as_str());
            assert_eq!(
                comparison.parent.codes[0].cause,
                Some("unsupported_construct")
            );
        }
    }

    #[trace("TC-868", "FR-345-AC-8")]
    #[test]
    fn entry_kind_is_closed_at_json_intake() {
        for (wire, expected) in [
            ("case", EntryKind::Case),
            ("distinguishing", EntryKind::Distinguishing),
            ("witness", EntryKind::Witness),
        ] {
            let entry: EntryFile = serde_json::from_value(json!({
                "kind": wire, "packages": [], "dependencies": [],
            }))
            .unwrap();
            assert!(entry.kind == expected);
        }
        assert!(serde_json::from_value::<EntryFile>(json!({
            "kind": "unknown", "packages": [], "dependencies": [],
        }))
        .is_err());

        let temp = crate::test_support::tempdir();
        write_pair(
            temp.path(),
            "valid",
            "case",
            EDGES[0],
            "parent.native",
            "child.native",
            "invariant I using v { true }\n",
        );
        let invalid = temp.path().join("invalid");
        fs::create_dir(&invalid).unwrap();
        fs::write(
            invalid.join("entry.json"),
            serde_json::to_vec(&json!({
                "kind": "case", "packages": [], "dependencies": [], "expected": "holds",
            }))
            .unwrap(),
        )
        .unwrap();
        let mut calls = 0;
        let report = compare(
            temp.path(),
            |_| {
                calls += 1;
                Compilation::classified(F, Vec::new())
            },
            &RegistrySnapshot::default(),
            None,
        );
        assert_eq!(calls, 2, "the valid entry still compiles both units");
        let defects: Vec<_> = report
            .results
            .iter()
            .filter_map(|result| match result {
                EntryResult::ToolFailure { entry, defect, .. }
                    if entry.ends_with("invalid/entry.json") =>
                {
                    Some(defect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(defects.len(), 1);
        assert!(defects[0].contains("unknown field `expected`"));
        assert_eq!(report.verdict, super::super::report::Verdict::ToolFailure);
        assert_eq!(report.exit, 30);
    }
}
