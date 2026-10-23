// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-345: profile-layering refinement gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::{Setting, SettingLimits, SourceIdentity};
use qsl_replay::spine::{
    self, CheckedUnit, DependencyInput, ParseRequest, RefinementClass, SpineLimits, SuppliedLibrary,
};
use qsl_replay::Cancel;
use qsl_route::routing::Disposition;
use qsl_route::{BackendDescriptor, Candidate, CandidateOutcome, RegistrationRefusal, Registry};
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
    /// The FR-075 candidate set for one witness kind.
    pub fn candidates(&self, kind: Capability) -> CandidateOutcome {
        self.registry.candidates(kind, None)
    }
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

/// The concrete corpus directory that ships with QSL.
pub const REAL_CORPUS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tests/fixtures/refinement/layering/"
);

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
    kind: String,
    parent: Option<SourceFile>,
    child: Option<SourceFile>,
    unit: Option<SourceFile>,
    layer: Option<String>,
    packages: Vec<String>,
    dependencies: Vec<DependencyFile>,
    #[serde(default)]
    limits: BTreeMap<String, u64>,
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

/// One result in a layering gate report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryResult {
    /// A case's parent/child compile comparison.
    Case {
        /// Exact case identity.
        name: CaseName,
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
        /// Compile class of its witness unit.
        class: &'static str,
        /// Compile codes.
        codes: Vec<super::report::CodeReport>,
        /// Candidate dispositions, in registry order.
        candidates: Vec<CandidateResult>,
        /// A result detail such as `not-negotiated` or `empty candidate set`.
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        /// Layer result.
        result: CaseResult,
    },
    /// An edge's proper-subset check.
    Edge {
        /// Stable edge name.
        edge: String,
        /// Parent-side compile.
        parent: SideResult,
        /// Child-side compile.
        child: SideResult,
        /// Edge result.
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
    pub codes: Vec<super::report::CodeReport>,
    checked: Option<CheckedUnit>,
}

impl Compilation {
    /// A test or alternate compile seam's typed classification.
    pub fn classified(class: RefinementClass, codes: Vec<super::report::CodeReport>) -> Self {
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
            let codes = refusal
                .causes()
                .iter()
                .map(|cause| super::report::CodeReport {
                    code: cause.code.as_str(),
                    cause: cause.cause,
                })
                .collect();
            Compilation::classified(classification.class, codes)
        }
        Err(_) => Compilation::classified(RefinementClass::ToolFailure, Vec::new()),
    }
}

fn profile_identity_span(bytes: &[u8]) -> Result<(String, usize, usize), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the unit is not UTF-8".to_owned())?;
    let mut found = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("profile ") {
            if found.is_some() {
                return Err("the unit has more than one profile declaration".to_owned());
            }
            let open = line
                .find('"')
                .ok_or_else(|| "the profile declaration has no quoted identity".to_owned())?;
            let close = line[open + 1..]
                .find('"')
                .map(|index| open + 1 + index)
                .ok_or_else(|| "the profile identity is not closed".to_owned())?;
            found = Some((
                line[open + 1..close].to_owned(),
                offset + open + 1,
                offset + close,
            ));
        }
        offset += line.len();
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
            Component::ParentDir => {
                return Err(format!("{name} resolves outside the entry directory"))
            }
            Component::Normal(_) | Component::CurDir => {}
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
    match file.kind.as_str() {
        "case" | "distinguishing" => {
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
            if file.kind == "case" {
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
        "witness" => {
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
        other => Err(format!(
            "kind {other:?} is not case, witness or distinguishing"
        )),
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
    let mut witnesses = BTreeSet::new();
    let mut distinguishing = BTreeSet::new();
    let mut loaded = Vec::new();
    for directory in directories {
        let label = directory
            .file_name()
            .unwrap_or(directory.as_os_str())
            .to_string_lossy()
            .into_owned();
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
                    name: CaseName {
                        parent: parent.reference,
                        child: child.reference,
                        parent_layer: edge.parent.identity().to_owned(),
                        child_layer: edge.child.identity().to_owned(),
                    },
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
                        .any(|code| code.code == "unsupported_construct");
                let result =
                    if parent_prohibited && child_compile.class == RefinementClass::Admitted {
                        CaseResult::Holds
                    } else {
                        CaseResult::Regression
                    };
                results.push(EntryResult::Edge {
                    edge: edge.name.to_owned(),
                    parent: side_result(parent_compile),
                    child: side_result(child_compile),
                    result,
                });
            }
            LoadedEntry::Witness { layer, unit } => {
                witnesses.insert(layer);
                results.push(witness_result(
                    layer,
                    compile(&unit.source),
                    registry,
                    &mut negotiation,
                ));
            }
        }
    }

    if cases.is_empty() || distinguishing.is_empty() || witnesses.is_empty() {
        // A non-empty directory corpus with malformed entries still receives
        // the missing-item failures below; malformed entries never count.
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
    }
    for layer in HeaderLayer::ALL {
        if !witnesses.contains(&layer) {
            results.push(tool_failure(
                format!("{}/witness", layer.identity()),
                format!("layer {} has no witness entry", layer.identity()),
            ));
        }
    }
    Report::new(results)
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
        codes: compilation.codes,
    }
}

fn witness_result(
    layer: HeaderLayer,
    compilation: Compilation,
    registry: &RegistrySnapshot,
    negotiation: &mut Option<&mut NegotiationFn<'_>>,
) -> EntryResult {
    let layer_name = layer.identity().to_owned();
    if compilation.class != RefinementClass::Admitted {
        return EntryResult::Layer {
            layer: layer_name,
            class: super::report::class_name(compilation.class),
            codes: compilation.codes,
            candidates: Vec::new(),
            reason: Some("witness unit did not compile as admitted".to_owned()),
            result: CaseResult::Regression,
        };
    }
    let Some(checked) = compilation.checked else {
        return EntryResult::Layer {
            layer: layer_name,
            class: "admitted",
            codes: compilation.codes,
            candidates: Vec::new(),
            reason: Some("compile seam returned no checked unit".to_owned()),
            result: CaseResult::ToolFailure,
        };
    };
    let graph = checked.package().graph();
    let Some(clause) = graph.state_clauses().first() else {
        return EntryResult::Layer {
            layer: layer_name,
            class: "admitted",
            codes: compilation.codes,
            candidates: Vec::new(),
            reason: Some("witness unit has no compiled state clause".to_owned()),
            result: CaseResult::ToolFailure,
        };
    };
    let witness_request = DefinitionLock::pinned().layer_row(layer).witness_request;
    let capability = match witness_request.kind {
        WitnessKind::OperationContract => Capability::OperationContract,
        WitnessKind::ValueValidity => Capability::ValueValidity,
    };
    let CandidateOutcome::Candidates(candidate_set) = registry.candidates(capability) else {
        return EntryResult::Layer {
            layer: layer_name,
            class: "admitted",
            codes: compilation.codes,
            candidates: Vec::new(),
            reason: Some(
                "the registry returned an unknown backend for an unnamed request".to_owned(),
            ),
            result: CaseResult::ToolFailure,
        };
    };
    if candidate_set.is_empty() {
        return EntryResult::Layer {
            layer: layer_name,
            class: "admitted",
            codes: compilation.codes,
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
        return EntryResult::Layer {
            layer: layer_name,
            class: "admitted",
            codes: compilation.codes,
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
                Disposition::Supported(target) => ("supported", target.id() == candidate.id()),
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
    EntryResult::Layer {
        layer: layer_name,
        class: "admitted",
        codes: compilation.codes,
        candidates,
        reason: None,
        result: if supported {
            CaseResult::Holds
        } else {
            CaseResult::Regression
        },
    }
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
    use qsl_route::{BackendDescriptor, Candidate, ManifestDigest, Mode, ProviderOrigin};
    use qsl_semantics::check::Capability;
    use std::collections::BTreeSet;
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
    #[trace("TC-868", "FR-345-AC-6")]
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

    /// The edge table is exactly the six lock rows' direct `requires` edges.
    #[test]
    fn the_edge_table_has_the_six_fr453_pairs() {
        assert_eq!(EDGES.len(), 6);
        assert_eq!(
            edge(HeaderLayer::ModelComplete, HeaderLayer::TemporalComplete)
                .unwrap()
                .name,
            "E6"
        );
        assert!(edge(HeaderLayer::StateCore, HeaderLayer::ModelComplete).is_none());
    }

    #[test]
    fn a_case_pair_may_change_only_the_profile_identity_string() {
        let parent = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.state.core/v1\";\ninvariant I using v { true }\n";
        let child = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.state.queries/v1\";\ninvariant I using v { true }\n";
        assert!(differs_only_by_profile_identity(parent.as_bytes(), child.as_bytes()).unwrap());
        let changed = child.replace("invariant I", "invariant J");
        assert!(!differs_only_by_profile_identity(parent.as_bytes(), changed.as_bytes()).unwrap());
    }
}
