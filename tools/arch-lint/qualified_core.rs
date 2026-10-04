// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-284 (ADR-029 CB-2, CB-3): the qualified core stays separable by crate.
//!
//! Two checks over the QSL core crates, [`CORE_CRATES`]:
//!
//! **The direction check** (CB-3 item 1) walks each core crate's resolved
//! dependency closure from `cargo metadata` and fails naming each offending
//! `(core crate, dependency)` pair. A dependency offends when it is
//!
//! - a QSL workspace member outside the core (the root crate, `qsl-analyze`,
//!   `qsl-walk-grow`, `qsl-bench`, the tools), or a named QSL crate above
//!   the core ([`QSL_ABOVE_CORE`]: `qsl-inspect`, `qsl-jit`);
//! - a driver crate ([`DRIVER`]);
//! - a CG or RT crate ([`crate::graph::classify`]). An IR crate is not: QSL
//!   depends on the IR model (ADR-011 §7.1) and the prove path's IR crates
//!   are inside the core (CB-2);
//! - an argument parser, a terminal or colour crate, a non-JSON renderer, a
//!   plugin-host or cache crate, or an execution backend ([`FRONTEND`]).
//!
//! Every other QSL workspace member gets the CG, RT and driver part of that
//! rule (FR-280-AC-3): no QSL crate depends on a CG, RT or driver crate.
//!
//! The walk follows normal dependency edges only and does not enter a
//! proc-macro crate: build scripts, build dependencies and proc macros run
//! in the compiler and link nothing into the core crate. It stops at an
//! offending dependency, so each pair names the crate the core reaches, not
//! that crate's own dependencies. One edge is not followed, temporarily:
//! [`FCD_LIFT_CLAP`], FCD's `agent-ix-extraction-frontend` depending on
//! `clap`. Under a core crate, only `qsl-semantics` may depend on that
//! frontend; an edge into it from any other crate fails
//! ([`Offence::FcdFrontend`]), so `clap` reached any way other than
//! `qsl-semantics -> agent-ix-extraction-frontend -> clap` fails. Metadata
//! is resolved with `--all-features`, so an optional dependency any feature turns on counts,
//! over the targets QSL builds for (the host and [`NO_STD_TARGET`]), so a dependency gated on either target counts and
//! one behind a `cfg` neither sets (`cfg(loom)`) does not.
//!
//! **The ambient-input scan** (CB-3 item 2) token-scans the shipped source
//! of each core crate -- `#[cfg(test)]` items and `#[cfg(test)] mod` files
//! excluded, comments and string literals never matching, the same scan
//! FR-060 uses ([`crate::api_surface`]) -- for an environment read, a clock
//! read, a filesystem or temp-directory access, a mutable global, a write to
//! stdout or stderr, or a process exit ([`AMBIENT`]).
//!
//! One site is not reported, temporarily: [`FCD_LIFT_SCRATCH`], the scratch
//! directory `lift_document` hands FCD's `lift`, which writes its document
//! only to an output path. Any other ambient access in that file or function
//! still fails.
//!
//! **Stated limitations.** The scan matches paths, so a function imported
//! by name (`use std::env::var; var(..)`) is caught at its `use` line, not
//! at the call; a `use std::{env, ..}` group naming `env` alone is not
//! caught. Compile-time `env!` and `option_env!` are not reads of the
//! process environment and are not matched. A dependency's own ambient
//! reads are not scanned; the direction check is what governs dependencies.
//! A path method (`.exists()`, `.metadata()`) is matched by name, so a
//! same-named method on another type matches too; it is renamed.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;
use syn::visit::Visit;

use crate::api_surface::{
    cfg_test_lines, cfg_test_module_declarations, enclosing_function, flatten_tokens,
    module_path_of, pattern_match_lines, walk_rs_files, CallPattern,
};
use crate::error::{Code, Error, Result};
use crate::graph::{classify, Repo};

/// The QSL crates of the qualified core (ADR-029 CB-2), by package name.
/// Each crate's source root is `<name>/src` under the QSL root.
pub(crate) const CORE_CRATES: [&str; 11] = [
    "quire-exact",
    "quire-semantic-value",
    "qsl-foundation",
    "qsl-cst",
    "qsl-source",
    "qsl-forms",
    "qsl-semantics",
    "qsl-package",
    "qsl-eval",
    "qsl-route",
    "qsl-replay",
];

/// QSL crates above the core (ADR-029 CB-3 item 3), refused under a core
/// crate even when they are not members of the scanned workspace.
pub(crate) const QSL_ABOVE_CORE: &[&str] = &[
    "quire-spec-language",
    "qsl-analyze",
    "qsl-inspect",
    "qsl-jit",
];

/// The driver repository's crates: the driver library and the crates above
/// the core it holds (CB-3 item 3). The driver depends on QSL, so no QSL
/// crate depends on any of them.
pub(crate) const DRIVER: &[&str] = &[
    "quire-driver",
    "quire-plugin-host",
    "quire-cache",
    "quire-aot",
    "quire-cli",
];

/// The one dependency path the direction check does not follow:
/// `qsl-semantics` depends on FCD's `agent-ix-extraction-frontend` for
/// `lift`, and that crate depends on `clap` for its own command line. The
/// edge belongs to FCD. Only `qsl-semantics` may depend on the frontend, so
/// any other path from a core crate to `clap` fails.
///
/// Temporary: when FCD's extraction frontend puts `clap` behind a
/// non-default feature, this const and its tests are deleted.
pub(crate) const FCD_LIFT_CLAP: [&str; 3] =
    ["qsl-semantics", "agent-ix-extraction-frontend", "clap"];

/// The one ambient site the scan does not report, as (file under the QSL
/// root, enclosing function, category): FCD's `lift` writes its document
/// only to an output path, so `lift_document` gives it a scratch directory.
///
/// Temporary: when FCD offers a `lift` that returns the document bytes,
/// this const and its tests are deleted.
const FCD_LIFT_SCRATCH: (&str, &str, Ambient) = (
    "qsl-semantics/src/model/intake.rs",
    "lift_document",
    Ambient::Filesystem,
);

/// The `no_std` target QSL builds its shared leaves for, besides the host.
const NO_STD_TARGET: &str = "thumbv7em-none-eabi";

/// One frontend category CB-3 item 1 keeps out of the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Frontend {
    ArgumentParser,
    Terminal,
    Renderer,
    PluginHost,
    Cache,
    ExecutionBackend,
}

impl Frontend {
    fn as_str(self) -> &'static str {
        match self {
            Self::ArgumentParser => "argument parser",
            Self::Terminal => "terminal or colour crate",
            Self::Renderer => "renderer other than JSON",
            Self::PluginHost => "plugin host",
            Self::Cache => "cache",
            Self::ExecutionBackend => "execution backend",
        }
    }
}

/// Third-party crates by category, by package name.
pub(crate) const FRONTEND: &[(Frontend, &[&str])] = &[
    (
        Frontend::ArgumentParser,
        &[
            "clap",
            "clap_builder",
            "clap_lex",
            "structopt",
            "argh",
            "pico-args",
            "lexopt",
            "gumdrop",
            "bpaf",
            "getopts",
            "docopt",
            "xflags",
        ],
    ),
    (
        Frontend::Terminal,
        &[
            "anstream",
            "anstyle",
            "anstyle-parse",
            "anstyle-query",
            "anstyle-wincon",
            "colorchoice",
            "termcolor",
            "colored",
            "owo-colors",
            "ansi_term",
            "nu-ansi-term",
            "yansi",
            "console",
            "crossterm",
            "termion",
            "indicatif",
            "is-terminal",
            "is_terminal_polyfill",
            "atty",
            "supports-color",
            "ratatui",
            "dialoguer",
            "terminal_size",
        ],
    ),
    (
        Frontend::Renderer,
        &[
            "tabled",
            "prettytable-rs",
            "comfy-table",
            "cli-table",
            "term-table",
            "handlebars",
            "tera",
            "askama",
            "minijinja",
            "dot",
            "dot-writer",
            "graphviz-rust",
            "layout-rs",
            "plotters",
        ],
    ),
    (
        Frontend::PluginHost,
        &[
            "libloading",
            "dlopen",
            "dlopen2",
            "wasmtime",
            "wasmer",
            "extism",
            "abi_stable",
        ],
    ),
    (
        Frontend::Cache,
        &["cacache", "sled", "redb", "rocksdb", "heed", "lmdb"],
    ),
    (
        Frontend::ExecutionBackend,
        &[
            "cranelift-codegen",
            "cranelift-frontend",
            "cranelift-jit",
            "cranelift-module",
            "inkwell",
            "llvm-sys",
            "dynasm",
            "dynasmrt",
            "gccjit",
        ],
    ),
];

/// Why a dependency may not sit in a core crate's closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Offence {
    /// A QSL workspace member outside the core, or a [`QSL_ABOVE_CORE`]
    /// crate.
    AboveCore,
    /// A [`DRIVER`] crate.
    Driver,
    /// A CG or RT crate.
    Backend(Repo),
    Frontend(Frontend),
    /// An edge into FCD's `agent-ix-extraction-frontend` from a crate other
    /// than `qsl-semantics` ([`FCD_LIFT_CLAP`]).
    FcdFrontend,
}

impl fmt::Display for Offence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AboveCore => f.write_str("crate above the core"),
            Self::Driver => f.write_str("driver crate"),
            Self::Backend(repo) => write!(f, "{repo} crate"),
            Self::Frontend(category) => f.write_str(category.as_str()),
            Self::FcdFrontend => f.write_str("FCD frontend outside qsl-semantics"),
        }
    }
}

/// One resolved package of the dependency graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) source: Option<String>,
    pub(crate) workspace_member: bool,
    pub(crate) proc_macro: bool,
}

/// The resolved graph: packages, and each package's normal dependencies as
/// indices into `packages`.
#[derive(Clone, Debug, Default)]
pub(crate) struct DepGraph {
    pub(crate) packages: Vec<Package>,
    pub(crate) deps: Vec<Vec<usize>>,
}

impl DepGraph {
    fn index_of(&self, name: &str) -> Option<usize> {
        self.packages
            .iter()
            .position(|package| package.workspace_member && package.name == name)
    }
}

/// One direction finding: the QSL crate `core` (a core crate, or for the
/// CG/RT/driver rule any workspace member) reaches `dependency` through
/// `via`, the chain of crates between them (empty for a direct dependency).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct DirectionFinding {
    pub(crate) core: String,
    pub(crate) dependency: String,
    pub(crate) offence: Offence,
    pub(crate) via: Vec<String>,
}

/// The CG, RT and driver rule every QSL crate obeys (FR-280-AC-3).
fn backend_or_driver(package: &Package) -> Option<Offence> {
    let name = package.name.as_str();
    if DRIVER.contains(&name) {
        return Some(Offence::Driver);
    }
    match classify(name, package.source.as_deref()) {
        Some(repo @ (Repo::Cg | Repo::Rt)) => Some(Offence::Backend(repo)),
        _ => None,
    }
}

/// The full rule under a core crate (FR-284).
fn core_offence(package: &Package) -> Option<Offence> {
    let name = package.name.as_str();
    if CORE_CRATES.contains(&name) && package.workspace_member {
        return None;
    }
    if let Some(offence) = backend_or_driver(package) {
        return Some(offence);
    }
    if package.workspace_member || QSL_ABOVE_CORE.contains(&name) {
        return Some(Offence::AboveCore);
    }
    FRONTEND
        .iter()
        .find(|(_, names)| names.contains(&name))
        .map(|(category, _)| Offence::Frontend(*category))
}

/// What [`FCD_LIFT_CLAP`] makes of the edge `current -> next` under a core
/// crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FcdEdge {
    /// Not an edge the const names.
    Other,
    /// `agent-ix-extraction-frontend -> clap`: not followed.
    Skip,
    /// An edge into `agent-ix-extraction-frontend` from a crate other than
    /// `qsl-semantics`: refused.
    Refuse,
}

fn fcd_edge(graph: &DepGraph, current: usize, next: usize) -> FcdEdge {
    let [semantics, frontend, clap] = FCD_LIFT_CLAP;
    let (from, to) = (&graph.packages[current].name, &graph.packages[next].name);
    if to == frontend && from != semantics {
        FcdEdge::Refuse
    } else if from == frontend && to == clap {
        FcdEdge::Skip
    } else {
        FcdEdge::Other
    }
}

/// The crates between `start` and `current`, `current` included unless it
/// is `start`.
fn chain(
    graph: &DepGraph,
    previous: &BTreeMap<usize, usize>,
    start: usize,
    current: usize,
) -> Vec<String> {
    let mut via = Vec::new();
    let mut cursor = current;
    while cursor != start {
        via.push(graph.packages[cursor].name.clone());
        cursor = previous[&cursor];
    }
    via.reverse();
    via
}

/// Walk `start`'s closure breadth-first, stopping at each package `rule`
/// refuses and recording it, with the chain it was reached through. With
/// `fcd`, [`FCD_LIFT_CLAP`]'s edges are applied before any package is
/// marked seen, so they hold whatever order the walk finds them in.
fn walk(
    graph: &DepGraph,
    start: usize,
    rule: fn(&Package) -> Option<Offence>,
    fcd: bool,
    findings: &mut BTreeSet<DirectionFinding>,
) {
    let mut previous: BTreeMap<usize, usize> = BTreeMap::new();
    let mut seen = BTreeSet::from([start]);
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        for &next in &graph.deps[current] {
            let package = &graph.packages[next];
            if package.proc_macro {
                continue;
            }
            let offence = match fcd.then(|| fcd_edge(graph, current, next)) {
                Some(FcdEdge::Skip) => continue,
                Some(FcdEdge::Refuse) => Some(Offence::FcdFrontend),
                Some(FcdEdge::Other) | None => {
                    if !seen.insert(next) {
                        continue;
                    }
                    previous.insert(next, current);
                    rule(package)
                }
            };
            let Some(offence) = offence else {
                queue.push_back(next);
                continue;
            };
            findings.insert(DirectionFinding {
                core: graph.packages[start].name.clone(),
                dependency: package.name.clone(),
                offence,
                via: chain(graph, &previous, start, current),
            });
        }
    }
}

/// FR-284's direction check over `graph`: the full rule under each core
/// crate, and the CG, RT and driver rule under every other workspace
/// member (FR-280-AC-3). A core crate absent from the graph is a usage
/// error: the check never passes over a tree it did not see.
pub(crate) fn check_direction(graph: &DepGraph) -> Result<Vec<DirectionFinding>> {
    let mut findings = BTreeSet::new();
    for core in CORE_CRATES {
        let start = graph.index_of(core).ok_or_else(|| {
            Error::new(
                Code::Usage,
                format!("core crate {core} is not a member of the --qsl workspace"),
            )
        })?;
        walk(graph, start, core_offence, true, &mut findings);
    }
    for (start, package) in graph.packages.iter().enumerate() {
        if package.workspace_member && !CORE_CRATES.contains(&package.name.as_str()) {
            walk(graph, start, backend_or_driver, false, &mut findings);
        }
    }
    Ok(findings.into_iter().collect())
}

/// The host target triple, from `rustc -vV`.
fn host_triple() -> Result<String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| Error::new(Code::Io, format!("cannot run rustc -vV: {error}")))?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| Error::new(Code::Io, "rustc -vV printed no host triple"))
}

/// `cargo metadata` over the workspace at `qsl_root`, every feature on and
/// filtered to the host and [`NO_STD_TARGET`], so a dependency gated on
/// either counts and one behind a `cfg` neither sets (`cfg(loom)`) does
/// not.
pub(crate) fn workspace_graph(qsl_root: &Path) -> Result<DepGraph> {
    let manifest = qsl_root.join("Cargo.toml");
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version=1",
            "--locked",
            "--all-features",
        ])
        .arg("--filter-platform")
        .arg(host_triple()?)
        .args(["--filter-platform", NO_STD_TARGET])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .map_err(|error| Error::io(&manifest, error))?;
    if !output.status.success() {
        return Err(Error::new(
            Code::CargoMetadata,
            format!(
                "cargo metadata failed for {}: {}",
                manifest.display(),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    let document: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        Error::new(
            Code::InvalidMetadata,
            format!("cargo metadata produced invalid JSON: {error}"),
        )
    })?;
    parse_graph(&document)
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Build a [`DepGraph`] from a `cargo metadata` document: every package,
/// and every resolved dependency with a normal (`kind: null`) edge.
pub(crate) fn parse_graph(document: &Value) -> Result<DepGraph> {
    let invalid = || Error::new(Code::InvalidMetadata, "unexpected cargo metadata shape");
    let members: BTreeSet<&str> = document
        .get("workspace_members")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?
        .iter()
        .map(|member| member.as_str().ok_or_else(invalid))
        .collect::<Result<_>>()?;
    let mut ids: BTreeMap<&str, usize> = BTreeMap::new();
    let mut graph = DepGraph::default();
    for package in document
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?
    {
        let id = str_at(package, "id").ok_or_else(invalid)?;
        let name = str_at(package, "name").ok_or_else(invalid)?;
        let proc_macro = package
            .get("targets")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?
            .iter()
            .any(|target| {
                target
                    .get("kind")
                    .and_then(Value::as_array)
                    .is_some_and(|kinds| {
                        kinds.iter().any(|kind| kind.as_str() == Some("proc-macro"))
                    })
            });
        ids.insert(id, graph.packages.len());
        graph.packages.push(Package {
            name: name.to_owned(),
            source: str_at(package, "source").map(str::to_owned),
            workspace_member: members.contains(id),
            proc_macro,
        });
        graph.deps.push(Vec::new());
    }
    for node in document
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(invalid)?
    {
        let from = *ids
            .get(str_at(node, "id").ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        for dep in node
            .get("deps")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?
        {
            let normal = dep
                .get("dep_kinds")
                .and_then(Value::as_array)
                .ok_or_else(invalid)?
                .iter()
                .any(|kind| kind.get("kind").is_some_and(Value::is_null));
            if normal {
                let to = *ids
                    .get(str_at(dep, "pkg").ok_or_else(invalid)?)
                    .ok_or_else(invalid)?;
                graph.deps[from].push(to);
            }
        }
    }
    Ok(graph)
}

/// One ambient-input category CB-3 item 2 keeps out of core source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Ambient {
    Environment,
    Clock,
    Filesystem,
    GlobalState,
    ProcessStream,
    ProcessExit,
}

impl Ambient {
    fn as_str(self) -> &'static str {
        match self {
            Self::Environment => "environment read",
            Self::Clock => "clock read",
            Self::Filesystem => "filesystem or search-path access",
            Self::GlobalState => "mutable global state",
            Self::ProcessStream => "process stream access",
            Self::ProcessExit => "process exit",
        }
    }
}

/// How a pattern is spelled: a path, optionally ending in a call `(`, or a
/// macro invocation `name!`.
#[derive(Clone, Copy)]
enum Spelling {
    Path(&'static str),
    Macro(&'static str),
    /// A method call `.name(`.
    Method(&'static str),
}

/// The ambient-input patterns, by category.
const AMBIENT: &[(Ambient, &[Spelling])] = {
    use Spelling::{Macro, Method, Path};
    &[
        (
            Ambient::Environment,
            &[
                Path("std::env"),
                Path("env::var("),
                Path("env::var_os("),
                Path("env::vars("),
                Path("env::vars_os("),
                Path("env::args("),
                Path("env::args_os("),
                Path("env::current_dir("),
                Path("env::current_exe("),
                Path("env::home_dir("),
                Path("env::temp_dir("),
            ],
        ),
        (
            Ambient::Clock,
            &[
                Path("SystemTime::now("),
                Path("Instant::now("),
                Path("Utc::now("),
                Path("Local::now("),
                Path("OffsetDateTime::now_utc("),
            ],
        ),
        (
            Ambient::Filesystem,
            &[
                Path("std::fs"),
                Path("fs::read("),
                Path("fs::read_to_string("),
                Path("fs::read_dir("),
                Path("fs::metadata("),
                Path("fs::write("),
                Path("fs::create_dir_all("),
                Path("File::open("),
                Path("File::create("),
                Path("OpenOptions"),
                Path("tempfile::"),
                Path("dirs::"),
                Method("exists"),
                Method("try_exists"),
                Method("is_file"),
                Method("is_dir"),
                Method("canonicalize"),
                Method("read_dir"),
                Method("metadata"),
                Method("read_link"),
            ],
        ),
        (
            Ambient::GlobalState,
            &[
                Macro("thread_local"),
                Macro("lazy_static"),
                Path("inventory::submit"),
                Path("linkme::distributed_slice"),
            ],
        ),
        (
            Ambient::ProcessStream,
            &[
                Macro("println"),
                Macro("print"),
                Macro("eprintln"),
                Macro("eprint"),
                Macro("dbg"),
                Path("io::stdout("),
                Path("io::stderr("),
                Path("io::stdin("),
            ],
        ),
        (
            Ambient::ProcessExit,
            &[Path("process::exit("), Path("process::abort(")],
        ),
    ]
};

/// Type names whose `static` is a mutable global that can hold a registry:
/// a lock or cell, or a once-cell or lazy value filled at run time. An
/// atomic counter is not one: it holds a number, not entries.
const MUTABLE_STATIC_TYPES: &[&str] = &[
    "OnceLock",
    "OnceCell",
    "LazyLock",
    "LazyCell",
    "Lazy",
    "Mutex",
    "RwLock",
    "Cell",
    "RefCell",
    "UnsafeCell",
];

/// One ambient-input finding in a core crate's shipped source.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct AmbientFinding {
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) ambient: Ambient,
}

/// The line of every `static mut` and every `static` whose type names one
/// of [`MUTABLE_STATIC_TYPES`].
fn mutable_static_lines(parsed: &syn::File) -> BTreeSet<usize> {
    struct Idents(Vec<String>);
    impl<'ast> Visit<'ast> for Idents {
        fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
            self.0.push(ident.to_string());
        }
    }
    struct Statics(BTreeSet<usize>);
    impl<'ast> Visit<'ast> for Statics {
        fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
            let mutable = matches!(node.mutability, syn::StaticMutability::Mut(_));
            let mut idents = Idents(Vec::new());
            idents.visit_type(&node.ty);
            let type_idents = idents.0;
            if mutable
                || type_idents
                    .iter()
                    .any(|name| MUTABLE_STATIC_TYPES.contains(&name.as_str()))
            {
                self.0.insert(node.static_token.span.start().line);
            }
            syn::visit::visit_item_static(self, node);
        }
    }
    let mut visitor = Statics(BTreeSet::new());
    visitor.visit_file(parsed);
    visitor.0
}

/// Scan one file. `exempt` names a function whose findings of one category
/// are not reported ([`FCD_LIFT_SCRATCH`]).
fn scan_file(
    path: &Path,
    compiled: &[(Ambient, Vec<CallPattern>)],
    exempt: Option<(&str, Ambient)>,
) -> Result<Vec<AmbientFinding>> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let parsed = syn::parse_file(&text).map_err(|source| Error::source_parse(path, source))?;
    let stream: proc_macro2::TokenStream = text
        .parse()
        .map_err(|source| Error::source_parse(path, source))?;
    let mut tokens = Vec::new();
    flatten_tokens(stream, &mut tokens);
    let excluded = cfg_test_lines(&parsed);
    let mut by_line: BTreeMap<usize, Ambient> = BTreeMap::new();
    for (ambient, patterns) in compiled {
        for line in pattern_match_lines(&tokens, patterns) {
            by_line.entry(line).or_insert(*ambient);
        }
    }
    for line in mutable_static_lines(&parsed) {
        by_line.entry(line).or_insert(Ambient::GlobalState);
    }
    Ok(by_line
        .into_iter()
        .filter(|(line, _)| !excluded.contains(line))
        .filter(|(line, ambient)| {
            exempt.is_none_or(|(function, category)| {
                *ambient != category || enclosing_function(&parsed, *line) != function
            })
        })
        .map(|(line, ambient)| AmbientFinding {
            file: path.to_path_buf(),
            line,
            ambient,
        })
        .collect())
}

/// FR-284's ambient-input scan over the shipped source of every core crate
/// under `qsl_root`.
pub(crate) fn scan_ambient(qsl_root: &Path) -> Result<Vec<AmbientFinding>> {
    let compiled: Vec<(Ambient, Vec<CallPattern>)> = AMBIENT
        .iter()
        .map(|(ambient, spellings)| {
            let patterns = spellings
                .iter()
                .map(|spelling| match spelling {
                    Spelling::Path(path) => CallPattern::compile(path),
                    Spelling::Macro(name) => CallPattern::macro_invocation(name),
                    Spelling::Method(name) => CallPattern::method_call(name),
                })
                .collect();
            (*ambient, patterns)
        })
        .collect();
    let mut findings = Vec::new();
    for core in CORE_CRATES {
        let src_root = qsl_root.join(core).join("src");
        if !src_root.is_dir() {
            return Err(Error::new(
                Code::Usage,
                format!(
                    "core crate source root does not exist: {}",
                    src_root.display()
                ),
            ));
        }
        let mut files = Vec::new();
        walk_rs_files(&src_root, &mut files)?;
        files.sort();
        let modules: Vec<(PathBuf, String)> = files
            .into_iter()
            .map(|file| {
                let module = file
                    .strip_prefix(&src_root)
                    .map(module_path_of)
                    .unwrap_or_default();
                (file, module)
            })
            .collect();
        let test_modules = cfg_test_module_declarations(&modules)?;
        for (file, module) in &modules {
            let under_test = test_modules
                .iter()
                .any(|ancestor| module == ancestor || module.starts_with(&format!("{ancestor}::")));
            if !under_test {
                let (scratch_file, function, category) = FCD_LIFT_SCRATCH;
                let exempt = (file.strip_prefix(qsl_root) == Ok(Path::new(scratch_file)))
                    .then_some((function, category));
                findings.extend(scan_file(file, &compiled, exempt)?);
            }
        }
    }
    Ok(findings)
}

/// The report `arch-lint qualified-core` prints, and whether it passed.
pub(crate) fn report(
    qsl_root: &Path,
    direction: &[DirectionFinding],
    ambient: &[AmbientFinding],
) -> (String, bool) {
    let mut summary = String::from("FR-284 qualified-core check (ADR-029 CB-2, CB-3)\n");
    if direction.is_empty() {
        summary.push_str(
            "  Direction: PASS (no core crate depends on a crate above the core or a frontend)\n",
        );
    } else {
        summary.push_str("  Direction: FAIL\n");
        for finding in direction {
            summary.push_str(&format!(
                "    ({}, {}) {}",
                finding.core, finding.dependency, finding.offence
            ));
            if !finding.via.is_empty() {
                summary.push_str(&format!(" via {}", finding.via.join(" -> ")));
            }
            summary.push('\n');
        }
    }
    if ambient.is_empty() {
        summary.push_str(
            "  Ambient input: PASS (no core crate reads ambient input or writes to the process)\n",
        );
    } else {
        summary.push_str("  Ambient input: FAIL\n");
        for finding in ambient {
            let file = finding.file.strip_prefix(qsl_root).unwrap_or(&finding.file);
            summary.push_str(&format!(
                "    {}:{} {}\n",
                file.display(),
                finding.line,
                finding.ambient.as_str()
            ));
        }
    }
    (summary, direction.is_empty() && ambient.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// A synthetic workspace with the core's real layering and no
    /// violation: every core crate, the root crate and `qsl-analyze` above
    /// them, an IR crate and third-party leaves, and the `qsl-attrs` proc
    /// macro under `qsl-foundation`.
    struct Fixture {
        graph: DepGraph,
    }

    impl Fixture {
        fn clean() -> Self {
            let mut fixture = Self {
                graph: DepGraph::default(),
            };
            for core in CORE_CRATES {
                fixture.add(core, None, true, false);
            }
            // The walker toolkit is its own repository: a git dependency
            // of the core, not a workspace member.
            fixture.add(
                "quire-walk",
                Some("git+https://github.com/agent-ix/quire-walk?branch=main"),
                false,
                false,
            );
            fixture.add("quire-spec-language", None, true, false);
            fixture.add("qsl-analyze", None, true, false);
            fixture.add("qsl-attrs", None, true, true);
            fixture.add(
                "quire-contract-model",
                Some("git+https://github.com/agent-ix/quire-contract-ir?branch=main#ea63488"),
                false,
                false,
            );
            fixture.add(
                "serde",
                Some("registry+https://github.com/rust-lang/crates.io-index"),
                false,
                false,
            );
            for (from, to) in [
                ("quire-semantic-value", "quire-exact"),
                ("qsl-foundation", "quire-exact"),
                ("qsl-foundation", "qsl-attrs"),
                ("qsl-cst", "qsl-foundation"),
                ("qsl-forms", "qsl-cst"),
                ("qsl-semantics", "qsl-forms"),
                ("qsl-semantics", "quire-semantic-value"),
                ("qsl-semantics", "serde"),
                ("qsl-package", "qsl-semantics"),
                ("qsl-package", "quire-contract-model"),
                ("qsl-eval", "qsl-package"),
                ("qsl-route", "qsl-semantics"),
                ("qsl-replay", "qsl-eval"),
                ("qsl-replay", "quire-walk"),
                ("qsl-source", "qsl-foundation"),
                ("qsl-analyze", "qsl-replay"),
                ("quire-spec-language", "qsl-replay"),
                ("quire-spec-language", "qsl-analyze"),
            ] {
                fixture.edge(from, to);
            }
            fixture
        }

        fn add(&mut self, name: &str, source: Option<&str>, member: bool, proc_macro: bool) {
            self.graph.packages.push(Package {
                name: name.to_owned(),
                source: source.map(str::to_owned),
                workspace_member: member,
                proc_macro,
            });
            self.graph.deps.push(Vec::new());
        }

        fn index(&self, name: &str) -> usize {
            self.graph
                .packages
                .iter()
                .position(|package| package.name == name)
                .unwrap_or_else(|| panic!("no package {name}"))
        }

        /// A test manifest that adds `to` to `from`'s dependencies.
        fn edge(&mut self, from: &str, to: &str) {
            let (from, to) = (self.index(from), self.index(to));
            self.graph.deps[from].push(to);
        }

        fn pairs(&self) -> Vec<(String, String)> {
            check_direction(&self.graph)
                .unwrap()
                .into_iter()
                .map(|finding| (finding.core, finding.dependency))
                .collect()
        }
    }

    fn pair(core: &str, dependency: &str) -> (String, String) {
        (core.to_owned(), dependency.to_owned())
    }

    /// TC-767 step 1: the direction check passes over a workspace with the
    /// core's layering, an IR dependency and a proc macro under the core.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_clean_workspace_passes() {
        assert_eq!(Fixture::clean().pairs(), Vec::<(String, String)>::new());
    }

    /// TC-767 step 2: `qsl-eval` depending on `qsl-analyze` fails naming
    /// `(qsl-eval, qsl-analyze)`, and every core crate above `qsl-eval`
    /// that reaches it names its own pair.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_qsl_eval_depending_on_analyze_fails() {
        let mut fixture = Fixture::clean();
        fixture.edge("qsl-eval", "qsl-analyze");
        assert_eq!(
            fixture.pairs(),
            [
                pair("qsl-eval", "qsl-analyze"),
                pair("qsl-replay", "qsl-analyze")
            ]
        );
        let replay = check_direction(&fixture.graph)
            .unwrap()
            .into_iter()
            .find(|finding| finding.core == "qsl-replay")
            .unwrap();
        assert_eq!(replay.via, ["qsl-eval"]);
        assert_eq!(replay.offence, Offence::AboveCore);
    }

    /// TC-767 step 3: an argument parser under `qsl-replay` fails naming
    /// `(qsl-replay, clap)`, also when it arrives through a third-party
    /// crate.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_argument_parser_in_qsl_replay_fails() {
        let mut fixture = Fixture::clean();
        fixture.add(
            "clap",
            Some("registry+https://github.com/rust-lang/crates.io-index"),
            false,
            false,
        );
        fixture.edge("qsl-replay", "clap");
        assert_eq!(fixture.pairs(), [pair("qsl-replay", "clap")]);

        let mut fixture = Fixture::clean();
        fixture.add("clap", None, false, false);
        fixture.add("some-frontend", None, false, false);
        fixture.edge("some-frontend", "clap");
        fixture.edge("qsl-replay", "some-frontend");
        let findings = check_direction(&fixture.graph).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].dependency, "clap");
        assert_eq!(findings[0].via, ["some-frontend"]);
        assert_eq!(
            findings[0].offence,
            Offence::Frontend(Frontend::ArgumentParser)
        );
    }

    /// TC-767 step 4 and FR-280-AC-3: a CG crate under `qsl-route` fails
    /// naming `(qsl-route, quire-contract-codegen)`; so does a driver crate.
    #[trace("TC-767", "FR-284-AC-1", "FR-280-AC-3")]
    #[test]
    fn tc_767_cg_or_driver_crate_in_qsl_route_fails() {
        let mut fixture = Fixture::clean();
        fixture.add(
            "quire-contract-codegen",
            Some("git+https://github.com/agent-ix/quire-contract-codegen?branch=main#1"),
            false,
            false,
        );
        fixture.edge("qsl-route", "quire-contract-codegen");
        assert_eq!(
            fixture.pairs(),
            [pair("qsl-route", "quire-contract-codegen")]
        );
        assert_eq!(
            check_direction(&fixture.graph).unwrap()[0].offence,
            Offence::Backend(Repo::Cg)
        );

        let mut fixture = Fixture::clean();
        fixture.add("quire-cli", None, false, false);
        fixture.edge("qsl-route", "quire-cli");
        assert_eq!(fixture.pairs(), [pair("qsl-route", "quire-cli")]);
    }

    /// FR-280-AC-3: every QSL workspace member outside the core gets the
    /// CG, RT and driver rule, and only that rule: `qsl-analyze` depending
    /// on a CG crate fails, and so does the root crate through it, while its
    /// argument parser is not refused.
    #[trace("TC-767", "FR-280-AC-3")]
    #[test]
    fn tc_767_cg_crate_in_a_non_core_member_fails() {
        let mut fixture = Fixture::clean();
        fixture.add(
            "quire-contract-codegen",
            Some("git+https://github.com/agent-ix/quire-contract-codegen?branch=main#1"),
            false,
            false,
        );
        fixture.add("clap", None, false, false);
        fixture.edge("qsl-analyze", "quire-contract-codegen");
        fixture.edge("qsl-analyze", "clap");
        assert_eq!(
            fixture.pairs(),
            [
                pair("qsl-analyze", "quire-contract-codegen"),
                pair("quire-spec-language", "quire-contract-codegen"),
            ]
        );
        let root = check_direction(&fixture.graph)
            .unwrap()
            .into_iter()
            .find(|finding| finding.core == "quire-spec-language")
            .unwrap();
        assert_eq!(root.via, ["qsl-analyze"]);

        let mut fixture = Fixture::clean();
        fixture.add("quire-driver", None, false, false);
        fixture.edge("quire-spec-language", "quire-driver");
        assert_eq!(
            fixture.pairs(),
            [pair("quire-spec-language", "quire-driver")]
        );
    }

    /// A proc macro is not entered: its own dependencies link nothing into
    /// the core crate.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_proc_macro_dependencies_are_not_walked() {
        let mut fixture = Fixture::clean();
        fixture.add("clap", None, false, false);
        fixture.edge("qsl-attrs", "clap");
        assert_eq!(fixture.pairs(), Vec::<(String, String)>::new());
    }

    /// The clean fixture plus FCD's `agent-ix-extraction-frontend` under
    /// `qsl-semantics`, with its `clap` dependency.
    fn with_fcd_frontend() -> Fixture {
        let mut fixture = Fixture::clean();
        fixture.add(
            "agent-ix-extraction-frontend",
            Some("git+https://github.com/agent-ix/filament-core-data?rev=1#1"),
            false,
            false,
        );
        fixture.add(
            "clap",
            Some("registry+https://github.com/rust-lang/crates.io-index"),
            false,
            false,
        );
        fixture.edge("qsl-semantics", "agent-ix-extraction-frontend");
        fixture.edge("agent-ix-extraction-frontend", "clap");
        fixture
    }

    /// The core crates that reach `qsl-semantics`, and so its `clap`.
    const ABOVE_SEMANTICS: [&str; 5] = [
        "qsl-eval",
        "qsl-package",
        "qsl-replay",
        "qsl-route",
        "qsl-semantics",
    ];

    /// The one FCD edge, `qsl-semantics -> agent-ix-extraction-frontend ->
    /// clap`, is not followed; `qsl-semantics` reaching `clap` directly or
    /// through another crate still fails for it and every core crate above
    /// it, and any edge into the frontend from a crate other than
    /// `qsl-semantics` fails.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_only_the_fcd_frontend_clap_edge_is_skipped() {
        assert_eq!(with_fcd_frontend().pairs(), Vec::<(String, String)>::new());

        let every_core_above_semantics: Vec<(String, String)> = ABOVE_SEMANTICS
            .iter()
            .map(|core| pair(core, "clap"))
            .collect();

        let mut direct = with_fcd_frontend();
        direct.edge("qsl-semantics", "clap");
        assert_eq!(direct.pairs(), every_core_above_semantics);

        let mut other = with_fcd_frontend();
        other.add("some-frontend", None, false, false);
        other.edge("some-frontend", "clap");
        other.edge("qsl-semantics", "some-frontend");
        assert_eq!(other.pairs(), every_core_above_semantics);
        let semantics = check_direction(&other.graph)
            .unwrap()
            .into_iter()
            .find(|finding| finding.core == "qsl-semantics")
            .unwrap();
        assert_eq!(semantics.via, ["some-frontend"]);

        let frontend = "agent-ix-extraction-frontend";
        let mut eval = with_fcd_frontend();
        eval.edge("qsl-eval", frontend);
        assert_eq!(
            eval.pairs(),
            [pair("qsl-eval", frontend), pair("qsl-replay", frontend)]
        );
        let findings = check_direction(&eval.graph).unwrap();
        assert!(findings
            .iter()
            .all(|finding| finding.offence == Offence::FcdFrontend));

        // A second route into the frontend beside the sanctioned one fails
        // whichever route the walk discovers first.
        let mut second = with_fcd_frontend();
        second.add("y", None, false, false);
        second.edge("qsl-semantics", "y");
        second.edge("y", frontend);
        let every_core_reaching_the_frontend: Vec<(String, String)> = ABOVE_SEMANTICS
            .iter()
            .map(|core| pair(core, frontend))
            .collect();
        assert_eq!(second.pairs(), every_core_reaching_the_frontend);
        let semantics = check_direction(&second.graph)
            .unwrap()
            .into_iter()
            .find(|finding| finding.core == "qsl-semantics")
            .unwrap();
        assert_eq!(semantics.via, ["y"]);
        assert_eq!(semantics.offence, Offence::FcdFrontend);
    }

    /// A core crate missing from the graph is an error, never a pass.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_missing_core_crate_is_an_error() {
        let mut fixture = Fixture::clean();
        fixture.graph.packages[0].workspace_member = false;
        let error = check_direction(&fixture.graph).unwrap_err();
        assert_eq!(error.code, Code::Usage);
    }

    /// `parse_graph` keeps normal edges only, marks workspace members and
    /// proc macros from the `cargo metadata` document.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_parse_graph_reads_normal_edges_members_and_proc_macros() {
        let document = serde_json::json!({
            "workspace_members": ["path+file:///q/qsl-eval#0.1.0"],
            "packages": [
                {"id": "path+file:///q/qsl-eval#0.1.0", "name": "qsl-eval", "source": null,
                 "targets": [{"kind": ["lib"]}]},
                {"id": "registry+x#clap@4", "name": "clap", "source": "registry+x",
                 "targets": [{"kind": ["lib"]}]},
                {"id": "registry+x#tempfile@3", "name": "tempfile", "source": "registry+x",
                 "targets": [{"kind": ["lib"]}]},
                {"id": "registry+x#serde_derive@1", "name": "serde_derive", "source": "registry+x",
                 "targets": [{"kind": ["proc-macro"]}]}
            ],
            "resolve": {"nodes": [
                {"id": "path+file:///q/qsl-eval#0.1.0", "deps": [
                    {"pkg": "registry+x#clap@4", "dep_kinds": [{"kind": "build", "target": null}]},
                    {"pkg": "registry+x#tempfile@3", "dep_kinds": [{"kind": "dev", "target": null}, {"kind": null, "target": null}]},
                    {"pkg": "registry+x#serde_derive@1", "dep_kinds": [{"kind": null, "target": null}]}
                ]},
                {"id": "registry+x#clap@4", "deps": []},
                {"id": "registry+x#tempfile@3", "deps": []},
                {"id": "registry+x#serde_derive@1", "deps": []}
            ]}
        });
        let graph = parse_graph(&document).unwrap();
        assert!(graph.packages[0].workspace_member);
        assert!(!graph.packages[1].workspace_member);
        assert!(graph.packages[3].proc_macro);
        assert!(!graph.packages[1].proc_macro);
        assert_eq!(graph.deps[0], [2, 3]);
    }

    fn core_tree(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for core in CORE_CRATES {
            fs::create_dir_all(dir.path().join(core).join("src")).unwrap();
        }
        for (relative, contents) in files {
            let path = dir.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        dir
    }

    fn found(dir: &tempfile::TempDir) -> Vec<(String, usize, Ambient)> {
        scan_ambient(dir.path())
            .unwrap()
            .into_iter()
            .map(|finding| {
                let file = finding
                    .file
                    .strip_prefix(dir.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                (file, finding.line, finding.ambient)
            })
            .collect()
    }

    /// The ambient-input scan finds each category in shipped core source.
    #[trace("TC-768", "FR-284-AC-4")]
    #[test]
    fn tc_768_ambient_reads_and_process_writes_in_core_source_fail() {
        let dir = core_tree(&[(
            "qsl-eval/src/lib.rs",
            "fn a() { let _ = std::env::var(\"HOME\"); }\n\
             fn b() { let _ = std::time::Instant::now(); }\n\
             fn c() { let _ = std::fs::read(\"config.toml\"); }\n\
             static REGISTRY: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());\n\
             fn d() { println!(\"x\"); }\n\
             fn e() { std::process::exit(1); }\n\
             fn f() { let _ = tempfile::tempdir(); }\n\
             static mut COUNT: u8 = 0;\n\
             fn g() { let _ = env::current_dir(); }\n\
             fn h(p: &Path) -> bool { p.exists() }\n\
             fn i(p: &Path) -> bool { p.is_file() }\n\
             fn j(p: &Path) { let _ = p.canonicalize(); }\n\
             fn k(p: &Path) { let _ = p.read_dir(); }\n\
             fn l(p: &Path) { let _ = p.metadata(); }\n\
             static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();\n\
             static TABLE: LazyLock<BTreeMap<u8, u8>> = LazyLock::new(BTreeMap::new);\n",
        )]);
        let file = "qsl-eval/src/lib.rs".to_owned();
        assert_eq!(
            found(&dir),
            [
                (file.clone(), 1, Ambient::Environment),
                (file.clone(), 2, Ambient::Clock),
                (file.clone(), 3, Ambient::Filesystem),
                (file.clone(), 4, Ambient::GlobalState),
                (file.clone(), 5, Ambient::ProcessStream),
                (file.clone(), 6, Ambient::ProcessExit),
                (file.clone(), 7, Ambient::Filesystem),
                (file.clone(), 8, Ambient::GlobalState),
                (file.clone(), 9, Ambient::Environment),
                (file.clone(), 10, Ambient::Filesystem),
                (file.clone(), 11, Ambient::Filesystem),
                (file.clone(), 12, Ambient::Filesystem),
                (file.clone(), 13, Ambient::Filesystem),
                (file.clone(), 14, Ambient::Filesystem),
                (file.clone(), 15, Ambient::GlobalState),
                (file, 16, Ambient::GlobalState),
            ]
        );
    }

    /// The scratch directory `lift_document` hands FCD's `lift` is the one
    /// site not reported. Filesystem access in another function of that
    /// file, any other category inside `lift_document`, and a same-named
    /// function in another file all still fail.
    #[trace("TC-768", "FR-284-AC-4")]
    #[test]
    fn tc_768_only_the_fcd_lift_scratch_directory_is_skipped() {
        let dir = core_tree(&[
            (
                "qsl-semantics/src/model/intake.rs",
                "pub fn lift_document() {\n\
                 let _ = tempfile::tempdir();\n\
                 let _ = std::env::var(\"X\");\n\
                 }\n\
                 fn other() { let _ = tempfile::tempdir(); }\n",
            ),
            (
                "qsl-semantics/src/model/other.rs",
                "pub fn lift_document() { let _ = tempfile::tempdir(); }\n",
            ),
        ]);
        let intake = "qsl-semantics/src/model/intake.rs".to_owned();
        assert_eq!(
            found(&dir),
            [
                (intake.clone(), 3, Ambient::Environment),
                (intake, 5, Ambient::Filesystem),
                (
                    "qsl-semantics/src/model/other.rs".to_owned(),
                    1,
                    Ambient::Filesystem
                ),
            ]
        );
    }

    /// Test code (a `#[cfg(test)]` item macro included), comments, string
    /// literals, compile-time `env!`, an immutable `static`, an atomic
    /// counter, locals named `dirs` and `tempfile` and the comparison
    /// `print != x` do not match, and a crate outside the core is not
    /// scanned.
    #[trace("TC-768", "FR-284-AC-4")]
    #[test]
    fn tc_768_test_code_comments_and_constants_pass() {
        let dir = core_tree(&[
            (
                "qsl-replay/src/lib.rs",
                "// std::env::var is not read here\n\
                 const VERSION: &str = env!(\"CARGO_PKG_VERSION\");\n\
                 static TABLE: &[u8] = b\"println!\";\n\
                 pub fn f() -> &'static str { \"std::process::exit(1)\" }\n\
                 #[cfg(test)]\n\
                 mod tests { fn t() { println!(\"{}\", std::env::var(\"X\").is_ok()); } }\n\
                 #[cfg(test)]\n\
                 mod golden;\n\
                 #[cfg(test)]\n\
                 thread_local! { static CALLS: std::cell::Cell<u8> = const { std::cell::Cell::new(0) }; }\n\
                 static NEXT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);\n\
                 fn m(x: bool, print: bool) -> bool { let dirs = 1; let tempfile = dirs; print != x && tempfile > 0 }\n",
            ),
            (
                "qsl-replay/src/golden.rs",
                "fn g() { eprintln!(\"{:?}\", std::fs::read(\"x\")); }\n",
            ),
            ("qsl-analyze/src/lib.rs", "fn h() { println!(\"x\"); }\n"),
        ]);
        assert_eq!(found(&dir), Vec::new());
    }

    /// The report names each pair and each site, and passes only when both
    /// checks are clean.
    #[trace("TC-767", "FR-284-AC-1")]
    #[test]
    fn tc_767_report_names_pairs_and_sites() {
        let root = Path::new("/q");
        let (text, passed) = report(root, &[], &[]);
        assert!(passed, "{text}");
        let direction = [DirectionFinding {
            core: "qsl-replay".to_owned(),
            dependency: "clap".to_owned(),
            offence: Offence::Frontend(Frontend::ArgumentParser),
            via: vec!["some-frontend".to_owned()],
        }];
        let ambient = [AmbientFinding {
            file: PathBuf::from("/q/qsl-eval/src/lib.rs"),
            line: 3,
            ambient: Ambient::Clock,
        }];
        let (text, passed) = report(root, &direction, &ambient);
        assert!(!passed);
        assert!(
            text.contains("(qsl-replay, clap) argument parser via some-frontend\n"),
            "{text}"
        );
        assert!(
            text.contains("qsl-eval/src/lib.rs:3 clock read\n"),
            "{text}"
        );
    }
}
