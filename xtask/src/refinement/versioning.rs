// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-340: `cargo xtask refinement versioning <corpus>`. For every pair of
//! revisions of one specification in a corpus, runs each case through
//! `qsl_replay::spine::run_clause` against the prior and the superseding
//! revision, classifies each run (FR-342), compares the two classes (FR-343)
//! and returns the report (FR-344).
//!
//! The gate reaches compile and evaluation only through `run_clause`
//! (FR-340-CON-1). It reads every byte from the files a pair's `pair.json`
//! names and nothing else: no environment variable, clock or search location.
//! A path is relative to the pair's directory, or `repo:` followed by a path
//! relative to the repository root, which names an existing repository
//! example in place instead of copying it.
//!
//! A case holds one selection (the FR-312 `clause`, `frame` or `function`
//! member shape), `observations` naming the snapshot and invocation files it
//! reads by key, and optionally any of `limits`, `observation_limits`,
//! `model_limits` and `accounting`. `limits` and `model_limits` map FR-255
//! setting names to bounds and `accounting` maps counter names to bounds;
//! `observation_limits` takes `document_bytes`, `objects_per_document`,
//! `values_per_document` and `post_state`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::{Setting, SettingLimits, SettingName, Source, SourceIdentity};
use qsl_replay::spine::{
    classify_clause_run, read_clause_run_request, Classification, ClauseRequestParts,
    ClauseRunRefusal, ClauseRunRequest, ClauseRunSource, ClauseRunStage, ClauseSelectionParts,
    ClauseSelectionPartsKind, DependencyInput, RefinementClass, SpineLimits, SuppliedLibrary,
    SuppliedObservation,
};
use qsl_replay::AccountingLimits;
use qsl_semantics::model::observation::{
    AnchorKind, ObservationLimits, PostStateRange, SelectedAnchor, SelectedObject,
};
use serde::Deserialize;
use serde_json::value::RawValue;

use super::compare::{compare, CaseResult, SelectionKind, Side};
use super::report::{
    limit_reports, CaseEntry, CaseName, CorpusFailure, PairFailure, Report, SelectionName,
    SideName, SideReport,
};

/// The file a pair is read from, inside the pair's directory.
const PAIR_FILE: &str = "pair.json";

/// FR-001 reduced a source's labels to authority and identity; the spine
/// still names a source by four non-blank labels. The gate compares nothing
/// about the revision labels (FR-340 "Reading a pair"), so both sides carry
/// these two fixed values.
/// The prefix of a path relative to the repository root.
const REPOSITORY_PREFIX: &str = "repo:";

const REVISION_NAMESPACE: &str = "refinement-gate";
const REVISION: &str = "pair";

/// `pair.json`: exactly these members.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairFile {
    prior: SourceFile,
    superseding: SourceFile,
    packages: Vec<String>,
    dependencies: Vec<DependencyFile>,
    cases: Vec<CaseFile>,
}

/// A unit source and its two FR-001 labels.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    authority: String,
    identity: String,
}

/// One FR-099 dependency input: a library identity and its source.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencyFile {
    library: String,
    path: String,
    authority: String,
    identity: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFile {
    selection: SelectionFile,
    observations: BTreeMap<String, String>,
    #[serde(default)]
    limits: BTreeMap<String, u64>,
    #[serde(default)]
    observation_limits: Option<ObservationLimitsFile>,
    #[serde(default)]
    model_limits: BTreeMap<String, u64>,
    #[serde(default)]
    accounting: BTreeMap<String, u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum SelectionFile {
    Clause(ClauseFile),
    Frame(FrameFile),
    Function(FunctionFile),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClauseFile {
    name: String,
    observation: String,
    #[serde(default, rename = "self")]
    self_object: Option<ObjectFile>,
    #[serde(default)]
    anchor: Option<AnchorFile>,
    #[serde(default)]
    parameters: Option<BTreeMap<String, Box<RawValue>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectFile {
    population: String,
    key: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AnchorFile {
    Initialization { name: String },
    Handler { name: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameFile {
    operation: String,
    invocation: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionFile {
    name: String,
    arguments: Vec<ArgumentFile>,
    snapshot: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArgumentFile {
    parameter: String,
    value: Box<RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationLimitsFile {
    document_bytes: Option<u64>,
    objects_per_document: Option<u64>,
    values_per_document: Option<u64>,
    post_state: Option<PostStateFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PostStateFile {
    Refuse,
    Witness,
}

/// What is wrong with one pair. Rendered into the pair's tool-failure result.
#[derive(Debug, thiserror::Error)]
enum PairDefect {
    #[error("cannot read {path}: {source}")]
    Read { path: String, source: io::Error },
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{path} is an absolute path")]
    AbsolutePath { path: String },
    #[error("{path} resolves outside the pair directory")]
    OutsidePair { path: String },
    #[error(
        "the prior source is {prior_authority}/{prior_identity} and the superseding source is \
         {superseding_authority}/{superseding_identity}; the two units are revisions of one \
         specification"
    )]
    IdentityMismatch {
        prior_authority: String,
        prior_identity: String,
        superseding_authority: String,
        superseding_identity: String,
    },
    #[error("cases is empty")]
    NoCases,
    #[error("{path}: {message}")]
    Source { path: String, message: String },
    #[error("dependencies: {0}")]
    Dependencies(String),
}

/// One revision of the specification: its unit.
struct Revision {
    identity: SourceIdentity,
    path: String,
    bytes: Vec<u8>,
    reference: RawSourceRef,
}

impl Revision {
    fn source(&self) -> ClauseRunSource {
        ClauseRunSource::Program {
            identity: self.identity.clone(),
            path: self.path.clone(),
            bytes: self.bytes.clone(),
        }
    }
}

/// One case: its requests against each revision, or why they cannot be
/// formed.
struct Case {
    selection: SelectionName,
    requests: Result<(ClauseRunRequest, ClauseRunRequest), String>,
}

/// The two directories a path is resolved against.
struct Roots {
    pair: PathBuf,
    repository: PathBuf,
}

/// A well-formed pair: its two references and every case ready to run.
struct Pair {
    prior: RawSourceRef,
    superseding: RawSourceRef,
    cases: Vec<Case>,
}

/// One side's run: the stage it settled at and its classification.
struct Observed {
    stage: ClauseRunStage,
    classification: Classification,
}

/// FR-340: run the gate over `corpus`.
pub fn run(corpus: &Path, repository: &Path) -> Report {
    let directories = match pair_directories(corpus) {
        Ok(directories) if directories.is_empty() => {
            return corpus_failure(corpus, "the corpus holds no pair subdirectory".to_owned());
        }
        Ok(directories) => directories,
        Err(error) => {
            return corpus_failure(
                corpus,
                format!("the corpus cannot be read as a directory: {error}"),
            );
        }
    };
    let mut failures = Vec::new();
    let mut entries = Vec::new();
    for directory in directories {
        let label = pair_label(&directory);
        match load_pair(&directory, repository) {
            Ok(pair) => entries.extend(run_pair(pair)),
            Err(defect) => failures.push(PairFailure::new(label, defect.to_string())),
        }
    }
    Report::new(Vec::new(), failures, entries)
}

fn corpus_failure(corpus: &Path, defect: String) -> Report {
    Report::new(
        vec![CorpusFailure::new(corpus.display().to_string(), defect)],
        Vec::new(),
        Vec::new(),
    )
}

/// Every immediate subdirectory of `corpus`, in name order.
fn pair_directories(corpus: &Path) -> io::Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(corpus)? {
        let path = entry?.path();
        if path.is_dir() {
            directories.push(path);
        }
    }
    directories.sort();
    Ok(directories)
}

/// A pair's `pair.json` path relative to the corpus.
fn pair_label(directory: &Path) -> String {
    let name = directory.file_name().unwrap_or(directory.as_os_str());
    Path::new(name).join(PAIR_FILE).display().to_string()
}

fn run_pair(pair: Pair) -> Vec<CaseEntry> {
    let Pair {
        prior,
        superseding,
        cases,
    } = pair;
    cases
        .into_iter()
        .map(|case| {
            let name = CaseName {
                prior: prior.clone(),
                superseding: superseding.clone(),
                selection: case.selection,
            };
            let (before_request, after_request) = match case.requests {
                Ok(requests) => requests,
                Err(defect) => {
                    return CaseEntry {
                        name,
                        prior: None,
                        superseding: None,
                        limits: Vec::new(),
                        defect: Some(defect),
                        result: CaseResult::ToolFailure,
                    };
                }
            };
            let kind = SelectionKind::of(&before_request.selection);
            let before = observe(before_request);
            let after = observe(after_request);
            let result = compare(
                Side {
                    class: before.classification.class,
                    stage: before.stage,
                },
                Side {
                    class: after.classification.class,
                    stage: after.stage,
                },
                kind,
            );
            let mut limits = limit_reports(SideName::Prior, &before.classification);
            limits.extend(limit_reports(SideName::Superseding, &after.classification));
            CaseEntry {
                name,
                prior: Some(SideReport::new(before.stage, &before.classification)),
                superseding: Some(SideReport::new(after.stage, &after.classification)),
                limits,
                defect: None,
                result,
            }
        })
        .collect()
}

/// Run one request and classify it. A request that cannot be formed is a
/// tool failure of the case.
fn observe(request: ClauseRunRequest) -> Observed {
    match qsl_replay::spine::run_clause(request) {
        Ok(report) => {
            let run = classify_clause_run(&report);
            Observed {
                stage: run.stage,
                classification: run.classification,
            }
        }
        Err(ClauseRunRefusal::EmptySource) => Observed {
            stage: ClauseRunStage::Compile,
            classification: Classification {
                class: RefinementClass::ToolFailure,
                causes: Vec::new(),
            },
        },
    }
}

/// Read and check one pair. Nothing runs until `pair.json`, both units, the
/// packages and the dependencies are well formed; a case that cannot be
/// formed is a tool failure of that case alone.
fn load_pair(directory: &Path, repository: &Path) -> Result<Pair, PairDefect> {
    let roots = Roots {
        pair: directory
            .canonicalize()
            .map_err(|source| read_error(directory, source))?,
        repository: repository
            .canonicalize()
            .map_err(|source| read_error(repository, source))?,
    };
    let file: PairFile = serde_json::from_slice(&read_in(&roots, PAIR_FILE)?)?;
    if file.prior.authority != file.superseding.authority
        || file.prior.identity != file.superseding.identity
    {
        return Err(PairDefect::IdentityMismatch {
            prior_authority: file.prior.authority,
            prior_identity: file.prior.identity,
            superseding_authority: file.superseding.authority,
            superseding_identity: file.superseding.identity,
        });
    }
    if file.cases.is_empty() {
        return Err(PairDefect::NoCases);
    }
    let prior = revision(&roots, &file.prior)?;
    let superseding = revision(&roots, &file.superseding)?;
    let package_files = file
        .packages
        .iter()
        .map(|path| read_in(&roots, path))
        .collect::<Result<Vec<_>, _>>()?;
    let packages =
        qsl_semantics::model::intake::package_input(package_files.iter().map(Vec::as_slice));
    let dependencies = dependencies(&roots, &file.dependencies)?;
    let mut documents = Documents::new(&roots);
    let mut cases = Vec::new();
    for (index, case) in file.cases.iter().enumerate() {
        let requests = case_requests(
            index,
            case,
            &packages,
            &dependencies,
            &mut documents,
            [&prior, &superseding],
        )?;
        cases.push(Case {
            selection: selection_name(&case.selection),
            requests,
        });
    }
    Ok(Pair {
        prior: prior.reference,
        superseding: superseding.reference,
        cases,
    })
}

/// The requests of one case against both revisions. A path that is absolute
/// or outside its directory is the pair's defect; any other defect is the
/// case's.
fn case_requests(
    index: usize,
    case: &CaseFile,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    documents: &mut Documents<'_>,
    revisions: [&Revision; 2],
) -> Result<Result<(ClauseRunRequest, ClauseRunRequest), String>, PairDefect> {
    let at = |member: &str, reason: String| format!("cases[{index}].{member}: {reason}");
    let limits = match Limits::of(case) {
        Ok(limits) => limits,
        Err((member, reason)) => return Ok(Err(at(member, reason))),
    };
    let mut observations = BTreeMap::new();
    for (key, path) in &case.observations {
        match documents.read(key, path) {
            Ok(document) => {
                observations.insert(key.clone(), document);
            }
            Err(defect @ (PairDefect::AbsolutePath { .. } | PairDefect::OutsidePair { .. })) => {
                return Err(defect);
            }
            Err(defect) => return Ok(Err(at(&format!("observations.{key}"), defect.to_string()))),
        }
    }
    let build = |revision: &Revision| {
        read_clause_run_request(ClauseRequestParts {
            source: revision.source(),
            packages: packages.clone(),
            dependencies: dependencies.clone(),
            observations: observations.clone(),
            selection: selection_parts(&case.selection),
            expected_package_id_claim: None,
            limits: limits.spine,
            observation_limits: limits.observation,
            accounting: limits.accounting(),
        })
        .map_err(|error| at(&error.member, error.reason))
    };
    let [prior, superseding] = revisions;
    Ok(build(prior).and_then(|before| Ok((before, build(superseding)?))))
}

fn read_error(path: &Path, source: io::Error) -> PairDefect {
    PairDefect::Read {
        path: path.display().to_string(),
        source,
    }
}

/// The bytes of `path`, inside the pair directory or, after the `repo:`
/// prefix, inside the repository. A path that is absolute, or that reaches
/// outside its directory lexically or through a link, is refused.
#[qsl_attrs::string_edge]
fn read_in(roots: &Roots, path: &str) -> Result<Vec<u8>, PairDefect> {
    let (root, relative) = match path.strip_prefix(REPOSITORY_PREFIX) {
        Some(relative) => (&roots.repository, relative),
        None => (&roots.pair, path),
    };
    let relative_path = Path::new(relative);
    let mut depth = 0_usize;
    for component in relative_path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(PairDefect::AbsolutePath {
                    path: path.to_owned(),
                });
            }
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth = depth.checked_sub(1).ok_or_else(|| PairDefect::OutsidePair {
                    path: path.to_owned(),
                })?;
            }
        }
    }
    let resolved = root
        .join(relative_path)
        .canonicalize()
        .map_err(|source| read_error(Path::new(path), source))?;
    if !resolved.starts_with(root) {
        return Err(PairDefect::OutsidePair {
            path: path.to_owned(),
        });
    }
    fs::read(&resolved).map_err(|source| read_error(Path::new(path), source))
}

fn revision(roots: &Roots, file: &SourceFile) -> Result<Revision, PairDefect> {
    let bytes = read_in(roots, &file.path)?;
    let identity = SourceIdentity::new(
        file.authority.clone(),
        file.identity.clone(),
        REVISION_NAMESPACE,
        REVISION,
    );
    let source = Source::read_typed(identity.clone(), file.path.clone(), &bytes, usize::MAX)
        .map_err(|refusal| PairDefect::Source {
            path: file.path.clone(),
            message: refusal.error.message,
        })?;
    Ok(Revision {
        identity,
        path: file.path.clone(),
        bytes,
        reference: source.reference().clone(),
    })
}

fn dependencies(roots: &Roots, files: &[DependencyFile]) -> Result<DependencyInput, PairDefect> {
    let libraries = files
        .iter()
        .map(|file| {
            Ok(SuppliedLibrary {
                identity: file.library.clone(),
                source: SourceIdentity::new(
                    file.authority.clone(),
                    file.identity.clone(),
                    REVISION_NAMESPACE,
                    REVISION,
                ),
                path: file.path.clone(),
                bytes: read_in(roots, &file.path)?,
            })
        })
        .collect::<Result<Vec<_>, PairDefect>>()?;
    DependencyInput::new(libraries).map_err(|error| PairDefect::Dependencies(error.to_string()))
}

/// The limit sets one case states, each resolved against the setting table.
struct Limits {
    spine: SpineLimits,
    observation: ObservationLimits,
    accounting: Vec<(AccountingSetting, u64)>,
}

impl Limits {
    /// Resolve the four limit sets of `case`; a set the case does not state
    /// keeps its published default. Refuses with the member at fault.
    fn of(case: &CaseFile) -> Result<Self, (&'static str, String)> {
        let mut spine = SpineLimits::default();
        for (name, bound) in &case.limits {
            let setting = Setting::from_name(name)
                .ok_or_else(|| ("limits", format!("{name} is not a setting")))?;
            if !spine.set_bound(setting, *bound) {
                return Err(("limits", format!("{name} is not a spine limit")));
            }
        }
        for (name, bound) in &case.model_limits {
            let setting = Setting::from_name(name)
                .ok_or_else(|| ("model_limits", format!("{name} is not a setting")))?;
            if !spine.model.set_bound(setting, *bound) {
                return Err(("model_limits", format!("{name} is not a model limit")));
            }
        }
        let mut observation = ObservationLimits::default();
        if let Some(stated) = &case.observation_limits {
            if let Some(bound) = stated.document_bytes {
                observation.document_bytes = bound;
            }
            if let Some(bound) = stated.objects_per_document {
                observation.objects_per_document = bound;
            }
            if let Some(bound) = stated.values_per_document {
                observation.values_per_document = bound;
            }
            if let Some(policy) = &stated.post_state {
                observation.post_state = match policy {
                    PostStateFile::Refuse => PostStateRange::Refuse,
                    PostStateFile::Witness => PostStateRange::Witness,
                };
            }
        }
        let accounting = case
            .accounting
            .iter()
            .map(|(name, bound)| match SettingName::from_name(name) {
                Some(SettingName::Accounting(setting)) => Ok((setting, *bound)),
                Some(SettingName::Stage(_)) | None => {
                    Err(("accounting", format!("{name} is not a counter")))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            spine,
            observation,
            accounting,
        })
    }

    fn accounting(&self) -> qsl_replay::ScalarLimits {
        let mut limits = AccountingLimits::default();
        for (setting, bound) in &self.accounting {
            limits.set_bound(*setting, *bound);
        }
        limits.0
    }
}

/// The observation documents of one pair, each read once.
struct Documents<'a> {
    roots: &'a Roots,
    read: BTreeMap<String, SuppliedObservation>,
}

impl<'a> Documents<'a> {
    fn new(roots: &'a Roots) -> Self {
        Self {
            roots,
            read: BTreeMap::new(),
        }
    }

    /// The document at `path`, read for the observation `key`.
    fn read(&mut self, key: &str, path: &str) -> Result<SuppliedObservation, PairDefect> {
        if let Some(held) = self.read.get(path) {
            return Ok(held.clone());
        }
        let bytes = read_in(self.roots, path)?;
        let document = SuppliedObservation::from_document(key, bytes).map_err(|error| {
            PairDefect::Source {
                path: path.to_owned(),
                message: error.to_string(),
            }
        })?;
        self.read.insert(path.to_owned(), document.clone());
        Ok(document)
    }
}

fn selection_parts(selection: &SelectionFile) -> ClauseSelectionPartsKind {
    match selection {
        SelectionFile::Clause(clause) => ClauseSelectionPartsKind::Clause(ClauseSelectionParts {
            name: clause.name.clone(),
            observation: clause.observation.clone(),
            self_object: clause.self_object.as_ref().map(|object| SelectedObject {
                population: object.population.clone(),
                key: object.key.clone(),
            }),
            anchor: clause.anchor.as_ref().map(|anchor| match anchor {
                AnchorFile::Initialization { name } => SelectedAnchor {
                    kind: AnchorKind::Initialization,
                    name: name.clone(),
                },
                AnchorFile::Handler { name } => SelectedAnchor {
                    kind: AnchorKind::Handler,
                    name: name.clone(),
                },
            }),
            parameters: clause.parameters.clone(),
        }),
        SelectionFile::Frame(frame) => ClauseSelectionPartsKind::Frame {
            operation: frame.operation.clone(),
            invocation: frame.invocation.clone(),
        },
        SelectionFile::Function(function) => ClauseSelectionPartsKind::Function {
            name: function.name.clone(),
            arguments: function
                .arguments
                .iter()
                .map(|argument| (argument.parameter.clone(), argument.value.clone()))
                .collect(),
            snapshot: function.snapshot.clone(),
        },
    }
}

fn selection_name(selection: &SelectionFile) -> SelectionName {
    match selection {
        SelectionFile::Clause(clause) => SelectionName::Clause {
            name: clause.name.clone(),
            observation: clause.observation.clone(),
        },
        SelectionFile::Frame(frame) => SelectionName::Frame {
            operation: frame.operation.clone(),
            invocation: frame.invocation.clone(),
        },
        SelectionFile::Function(function) => SelectionName::Function {
            name: function.name.clone(),
            snapshot: function.snapshot.clone(),
        },
    }
}
