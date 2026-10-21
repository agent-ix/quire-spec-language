// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-023 section 8's vault as a hyper world: the `test/vault` domain
//! package (object type `Vault`, secret `h` and public output `l` over
//! `Int[0, 1]`, population `vaults`, operation `step(i: Int[0, 1])` with frame
//! `modifies [l]`, and `reset()`), the units that give it the leaky and the
//! secure `step` contract, the two initial snapshots `(h, l) = (0, 0)` and
//! `(1, 0)`, and the seams a hyper run reads its letters and `match` through.
//!
//! A hyper clause's `holds` atoms and `match` conjuncts are typed at S3 but
//! not lowered to checked nodes the clause evaluator can run, so the seams
//! read each from its source text over the components' states, as
//! `qsl-eval`'s tuple tests read the body: the vault's atoms are
//! `holds(v.l @ x = v.l @ y)` and `holds(v.l @ x = k)`, its conjuncts the
//! forms `HyperLetters` and `StepMatch` below name.

use std::collections::{BTreeMap, BTreeSet};

use qsl_analyze::model_check::{
    check_hyper, retain, HyperCheckRequest, ModelCheckRefusal, Reductions, RetainedGraph,
};
use qsl_eval::fairness::{analyze, EdgeLabel, Graph, Identity, Outcome as FairnessOutcome, Vertex};
use qsl_eval::hyper::{
    HyperFinding, HyperInputs, HyperLetters, HyperProduct, HyperState, HyperTransition,
    MatchReading, ParameterBinding, Property, StepLabel, StepMatch, WitnessInputs, WitnessProduct,
};
use qsl_eval::model_subject::{ModelInstance, ModelSubject};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::IdentityLimits;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_hyper_trace_tuple, HyperCounterexample, HyperOutcome, HyperReplay, ModelCheckLimits,
    ModelTraceSettlement, ReplayRefusal, DEFAULT_MAX_WITNESS_SET,
};
use qsl_semantics::check::{
    check_hyper_clauses, CheckedHyper, CheckedHyperClause, FormulaNode, FormulaNodeId,
    PackageDeclarations,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{Cancel, Incomplete, Meter, Undefined, Value};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

use crate::counter::UNLIMITED;

const PACKAGE: &str = "test/vault";
/// The package of the second model of a two-alias world, under alias
/// [`OTHER_ALIAS`].
const OTHER: &str = "test/vault-other";
const OTHER_ALIAS: &str = "N";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The key of the vault in `vaults`.
pub(crate) const KEY: &str = "v";

/// Which `step` contract the vault has (ADR-023 section 8).
#[derive(Clone, Copy)]
pub(crate) enum Vault {
    /// `self.l = (i + self.h) mod 2`: `l` is the exclusive or of the input and
    /// the secret, written as a disjunction because the checker types `=`
    /// between a bounded field and arithmetic as a mismatch.
    Leaky,
    /// `self.l = i`.
    Secure,
    /// `self.l = i`, enabled only while `self.l = 0`: after one `step` the
    /// vault is terminal, so a trace takes its stutter step from there.
    Terminal,
    /// The contracts as written: the `step` and `reset` contract text.
    Custom(&'static str),
    /// ADR-023 section 13's vault: a field `busy` and an internal operation
    /// `mix`. `step(i)` requires `busy = 0`, sets `l = i` and sets `busy` to
    /// `h`; `mix` requires `busy = 1` and clears it.
    Busy,
    /// [`Self::Busy`] whose `step` leaks as [`Self::Leaky`] does:
    /// `l = (i + h) mod 2`.
    BusyLeaky,
}

impl Vault {
    /// Whether the vault has the `busy` field and the `mix` operation.
    fn busy(self) -> bool {
        matches!(self, Self::Busy | Self::BusyLeaky)
    }

    fn contracts(self, reset: bool) -> String {
        if let Self::Custom(text) = self {
            return text.to_owned();
        }
        let step = match self {
            Self::Leaky => "post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
            Self::Secure => "post Step using v on V::Vault::step { self.l = i }\n",
            Self::Terminal => {
                "pre Fresh using v on V::Vault::step { self.l = 0 }\n\
                 post Step using v on V::Vault::step { self.l = i }\n"
            }
            Self::Busy => {
                "pre Idle using v on V::Vault::step { self.busy = 0 }\n\
                 post Step using v on V::Vault::step { self.l = i and self.busy = self.h }\n\
                 pre Busy using v on V::Vault::mix { self.busy = 1 }\n\
                 post Mixed using v on V::Vault::mix { self.busy = 0 }\n"
            }
            Self::BusyLeaky => {
                "pre Idle using v on V::Vault::step { self.busy = 0 }\n\
                 post Step using v on V::Vault::step { ((i = self.h and self.l = 0) or (i != self.h and self.l = 1)) and self.busy = self.h }\n\
                 pre Busy using v on V::Vault::mix { self.busy = 1 }\n\
                 post Mixed using v on V::Vault::mix { self.busy = 0 }\n"
            }
            Self::Custom(_) => unreachable!("returned above"),
        };
        let reset = if reset {
            "post Reset using v on V::Vault::reset { self.l = 0 }\n"
        } else {
            ""
        };
        format!("{step}{reset}")
    }
}

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

pub(crate) fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn wire_type(identity: &str, kind: &str, extra: Json) -> Json {
    let mut node = json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": kind}, "roles": [],
        "origin": origin(identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
    });
    for (name, value) in extra.as_object().expect("extra members") {
        node[name] = value.clone();
    }
    node
}

/// `Int[0, 1]`.
fn bit() -> Json {
    let identity = ident("Bit");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    wire_type(
        &identity,
        "value_type",
        json!({"scalar": "integer",
            "constraints": [constraint("min", "0"), constraint("max", "1")]}),
    )
}

fn field(name: &str) -> Json {
    let identity = ident(&format!("Vault/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": ident("Bit"),
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

/// An operation of `Vault` with frame `modifies_receiver` the fields `modifies`
/// (the vault it is applied to; the same as `modifies [l]` over one vault),
/// taking `i: Bit` when `input`.
fn operation(name: &str, input: bool, modifies: &[&str]) -> Json {
    let identity = ident(&format!("Vault/{name}"));
    let params: Vec<Json> = input
        .then(|| {
            let parameter = format!("{identity}/i");
            json!({
                "identity": parameter, "name": "i", "typeRef": ident("Bit"),
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&parameter),
            })
        })
        .into_iter()
        .collect();
    json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [],
            "modifies_receiver": modifies.iter().map(|field| ident(&format!("Vault/{field}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": []},
    })
}

/// The domain package, with the `busy` field and the `mix` operation when
/// `busy`.
fn document(busy: bool, reset: bool) -> Vec<u8> {
    let vault = ident("Vault");
    let constructs: Vec<Json> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let mut operations = vec![operation(
        "step",
        true,
        if busy { &["l", "busy"] } else { &["l"] },
    )];
    if reset {
        operations.push(operation("reset", false, &["l"]));
    }
    let mut fields = vec![field("h"), field("l")];
    if busy {
        operations.push(operation("mix", false, &["busy"]));
        fields.push(field("busy"));
    }
    let population = ident("vaults");
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            bit(),
            wire_type(&vault, "object_type", json!({
                "supertypes": [],
                "fields": fields,
                "operations": operations,
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [vault], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// The unit text: the header selecting the domain package as `V`, the `step`
/// contract of `vault`, and `body`.
fn unit_text(
    document: &[u8],
    other: Option<&[u8]>,
    vault: Vault,
    reset: bool,
    body: &str,
) -> String {
    let digest = *package_input([document]).keys().next().expect("one model");
    let second = other.map_or_else(String::new, |other| {
        let digest = *package_input([other]).keys().next().expect("one model");
        format!(
            "model {OTHER_ALIAS} = \"{OTHER}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n",
            hex(&digest)
        )
    });
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         model V = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {second}function Noop using v(): Boolean pure {{ true }}\n{}{body}",
        hex(&digest),
        vault.contracts(reset),
    )
}

/// The vault again under another package identity: the second model of a
/// clause over two aliases.
fn other_document(document: &[u8]) -> Vec<u8> {
    String::from_utf8(document.to_vec())
        .expect("the document is UTF-8")
        .replace(PACKAGE, OTHER)
        .into_bytes()
}

fn declarations(documents: &[&[u8]], text: &str) -> PackageDeclarations {
    let packages = package_input(documents.iter().copied());
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "hyper", "git", "1"),
        "hyper.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the operation clauses parse");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    // `div` reads truncating integer division.
    let lock = DefinitionLock::pinned();
    declarations.integer_division = Some(
        lock.admit_integer_division(
            &[lock
                .entry(CatalogRole::IntegerDivisionTruncating)
                .reference()],
            None,
        )
        .expect("the division profile admits"),
    );
    declarations
}

/// One snapshot of the vaults `(key, (h, l))`, provisioned in `snapshots`;
/// each vault of a busy world has `busy` as its flag.
fn snapshot(
    document: &[u8],
    snapshots: &mut BTreeMap<[u8; 32], Vec<u8>>,
    name: &str,
    vaults: &[(&str, (i64, i64))],
    busy: Option<i64>,
) -> DocumentRef {
    let digest = *package_input([document]).keys().next().expect("one model");
    let header = json!({"identity": PACKAGE, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    let body = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("vaults"), "complete": true,
            "objects": vaults.iter().map(|(key, (h, l))| {
                let mut fields = json!({"h": {"integer": h.to_string()},
                    "l": {"integer": l.to_string()}});
                if let Some(busy) = busy {
                    fields["busy"] = json!({"integer": busy.to_string()});
                }
                json!({"key": key, "type": ident("Vault"), "fields": fields})
            }).collect::<Vec<_>>()}],
    });
    let bytes = body.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let hash = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    snapshots.insert(hash, bytes);
    DocumentRef {
        authority: "test".to_owned(),
        identity: name.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "fixture:1".to_owned(),
        digest: hash,
    }
}

/// The `l` of the vault at `state`.
pub(crate) fn low(state: &ModelSimState) -> i64 {
    field_of(state, KEY, "l")
}

/// The `l` of the vault `key` at `state`.
pub(crate) fn low_of(state: &ModelSimState, key: &str) -> i64 {
    field_of(state, key, "l")
}

/// The `busy` of the vault at `state`, for a busy vault.
pub(crate) fn busy_of(state: &ModelSimState) -> i64 {
    field_of(state, KEY, "busy")
}

/// The `h` of the vault at `state`.
pub(crate) fn high(state: &ModelSimState) -> i64 {
    field_of(state, KEY, "h")
}

fn field_of(state: &ModelSimState, vault: &str, field: &str) -> i64 {
    match state.state().populations[&key("vaults")][vault]
        .fields
        .get(&key(&format!("Vault/{field}")))
    {
        Some(Value::Integer(value)) => value.to_string().parse().expect("a small value"),
        other => panic!("{field} is not an integer: {other:?}"),
    }
}

/// A checked vault world with one hyper clause.
pub(crate) struct World {
    pub(crate) text: String,
    pub(crate) package: CheckedPackage,
    pub(crate) clause: CheckedHyperClause,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
    pub(crate) max_candidates: u64,
    /// The search horizon of an HP-6 run (`max_depth`).
    pub(crate) max_depth: Option<usize>,
    /// The keys of the universe of `vaults`.
    pub(crate) universe: Vec<String>,
}

impl World {
    /// The vault under `vault`'s contract, with `reset` when asked, started
    /// from `starts` and the hyper clause `declaration`.
    pub(crate) fn new(vault: Vault, reset: bool, starts: &[(i64, i64)], declaration: &str) -> Self {
        let starts: Vec<_> = starts.iter().copied().map(Some).collect();
        Self::started(vault, reset, &starts, declaration)
    }

    /// [`Self::new`] from `starts`, each a vault `(h, l)` or, for `None`, an
    /// initial state whose population holds no object.
    pub(crate) fn started(
        vault: Vault,
        reset: bool,
        starts: &[Option<(i64, i64)>],
        declaration: &str,
    ) -> Self {
        let snapshots = starts
            .iter()
            .map(|start| start.map(|start| (KEY, start)).into_iter().collect())
            .collect();
        Self::build(
            vault,
            reset,
            snapshots,
            vec![KEY.to_owned()],
            false,
            declaration,
        )
    }

    /// One initial state holding the vaults `objects`, `(key, (h, l))`, which
    /// are also the universe: the clause has one instance for each.
    pub(crate) fn with_vaults(
        vault: Vault,
        objects: &[(&str, (i64, i64))],
        declaration: &str,
    ) -> Self {
        let universe = objects.iter().map(|(key, _)| (*key).to_owned()).collect();
        Self::build(
            vault,
            false,
            vec![objects.to_vec()],
            universe,
            false,
            declaration,
        )
    }

    /// One initial state for each of `snapshots`, each holding its vaults
    /// `(key, (h, l))`; `universe` are the keys the clause has an instance
    /// for.
    pub(crate) fn with_snapshots(
        vault: Vault,
        snapshots: Vec<Vec<(&str, (i64, i64))>>,
        universe: &[&str],
        declaration: &str,
    ) -> Self {
        let universe = universe.iter().map(|key| (*key).to_owned()).collect();
        Self::build(vault, false, snapshots, universe, false, declaration)
    }

    /// The vault, started from `starts`, with the vault again as a second
    /// model under alias `N`: the world of a clause over two aliases.
    pub(crate) fn with_second_model(
        vault: Vault,
        starts: &[(i64, i64)],
        declaration: &str,
    ) -> Self {
        let snapshots = starts.iter().map(|start| vec![(KEY, *start)]).collect();
        Self::build(
            vault,
            false,
            snapshots,
            vec![KEY.to_owned()],
            true,
            declaration,
        )
    }

    /// A busy vault (ADR-023 section 13) started from each of `starts`, a
    /// vault `(h, l, busy)`, and the hyper clause `declaration`.
    pub(crate) fn busy(vault: Vault, starts: &[(i64, i64, i64)], declaration: &str) -> Self {
        let snapshots = starts
            .iter()
            .map(|(h, l, _)| vec![(KEY, (*h, *l))])
            .collect();
        let flags = starts.iter().map(|(_, _, busy)| *busy).collect();
        Self::assemble(
            vault,
            false,
            snapshots,
            flags,
            vec![KEY.to_owned()],
            false,
            declaration,
        )
    }

    fn build(
        vault: Vault,
        reset: bool,
        starts: Vec<Vec<(&str, (i64, i64))>>,
        universe: Vec<String>,
        second: bool,
        declaration: &str,
    ) -> Self {
        Self::assemble(
            vault,
            reset,
            starts,
            Vec::new(),
            universe,
            second,
            declaration,
        )
    }

    /// The world over `starts`, whose snapshot `index` has the flag
    /// `flags[index]` (0 when `flags` holds none) in a busy vault, with the
    /// vault again as a second model under alias `N` when `second`.
    fn assemble(
        vault: Vault,
        reset: bool,
        starts: Vec<Vec<(&str, (i64, i64))>>,
        flags: Vec<i64>,
        universe: Vec<String>,
        second: bool,
        declaration: &str,
    ) -> Self {
        let document = document(vault.busy(), reset);
        let other = second.then(|| other_document(&document));
        let other = other.as_deref();
        let documents: Vec<&[u8]> = std::iter::once(document.as_slice()).chain(other).collect();
        let base = unit_text(&document, other, vault, reset, "");
        let graph = declarations(&documents, &base)
            .with_unwritten_terminals()
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{base}: the unit checks: {refusals:?}"));
        let package = CheckedPackage::link(graph);
        let text = unit_text(&document, other, vault, reset, &format!("{declaration}\n"));
        let mut checked = check_hyper_clauses(&declarations(&documents, &text));
        let clause = match checked.pop() {
            Some(Ok(CheckedHyper::Behaviours(clause))) if checked.is_empty() => *clause,
            other => panic!("{declaration}: one checked hyper clause, not {other:?}"),
        };
        let mut snapshots = BTreeMap::new();
        let initial = starts
            .iter()
            .enumerate()
            .map(|(index, start)| {
                let busy = vault.busy().then(|| flags.get(index).copied().unwrap_or(0));
                snapshot(&document, &mut snapshots, &format!("s{index}"), start, busy)
            })
            .collect();
        Self {
            text,
            package,
            clause,
            packages: package_input(documents.iter().copied()),
            snapshots,
            initial,
            max_candidates: 1_000_000,
            max_depth: None,
            universe,
        }
    }

    /// The vault's subject.
    pub(crate) fn subject(&self) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: self.initial.clone(),
                universes: vec![PopulationUniverse {
                    population: key("vaults"),
                    keys: self.universe.clone(),
                }],
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: self.max_candidates,
                },
                meter: UNLIMITED,
            },
            proof_bounds: Vec::new(),
        }
    }

    /// The letters and the `match` of this world's clause.
    pub(crate) fn seams(&self) -> Seams<'_> {
        Seams { world: self }
    }

    /// The run of the clause under `limits`.
    pub(crate) fn run(
        &self,
        limits: ModelCheckLimits,
        cancel: &Cancel,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        let seams = self.seams();
        self.run_through(&seams, &seams, limits, cancel)
    }

    /// The run of the clause when the request selects `reductions`, with one
    /// subject for each alias of the clause.
    pub(crate) fn run_selecting(
        &self,
        reductions: Reductions,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        let seams = self.seams();
        check_hyper(
            HyperCheckRequest {
                clause: &self.clause,
                subjects: (0..self.clause.aliases().len())
                    .map(|_| self.subject())
                    .collect(),
                letters: &seams,
                matcher: &seams,
                limits: ModelCheckLimits::default(),
                meter: UNLIMITED,
                reductions,
                max_depth: self.max_depth,
            },
            &Cancel::new(),
        )
    }

    /// The run of the clause reading its atoms through `letters` and its
    /// `match` through `matcher`.
    pub(crate) fn run_through(
        &self,
        letters: &dyn HyperLetters,
        matcher: &dyn StepMatch,
        limits: ModelCheckLimits,
        cancel: &Cancel,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        check_hyper(
            HyperCheckRequest {
                clause: &self.clause,
                subjects: vec![self.subject()],
                letters,
                matcher,
                limits,
                max_depth: self.max_depth,
                meter: UNLIMITED,
                reductions: Reductions::default(),
            },
            cancel,
        )
    }

    /// The source text of the body atom `atom`.
    pub(crate) fn atom_text(&self, atom: FormulaNodeId) -> &str {
        let Some(FormulaNode::Holds { span }) = self.clause.body().nodes().get(atom.index()) else {
            panic!("the letter reads only holds atoms");
        };
        &self.text[span.start..span.end]
    }

    /// The source text of the universal `match` conjunct `index`.
    fn conjunct_text(&self, index: usize) -> &str {
        let span = self.clause.mu_universal()[index].span();
        &self.text[span.start..span.end]
    }

    /// The source text of the existential `match` conjunct `index`.
    fn existential_text(&self, index: usize) -> &str {
        let span = self.clause.mu_existential()[index].span();
        &self.text[span.start..span.end]
    }

    /// The position of trace variable `variable` among the clause's
    /// quantifiers: its component in a joint position.
    fn component(&self, variable: &str) -> usize {
        self.clause
            .quantifiers()
            .iter()
            .position(|quantifier| quantifier.variable == variable)
            .unwrap_or_else(|| panic!("unmodelled trace variable {variable}"))
    }
}

/// The seams to the vault world's atoms and `match` conjuncts.
pub(crate) struct Seams<'w> {
    world: &'w World,
}

impl HyperLetters for Seams<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let vault = instance[0].instance.key.as_str();
        let text = self.world.atom_text(atom);
        let inner = text
            .strip_prefix("holds(")
            .and_then(|rest| rest.strip_suffix(')'))
            .unwrap_or_else(|| panic!("{text} is not a holds atom"));
        let words: Vec<&str> = inner.split_whitespace().collect();
        let read = |name: &str| low_of(states[self.world.component(name)], vault);
        let secret = |name: &str| field_of(states[self.world.component(name)], vault, "h");
        let holds = match words.as_slice() {
            ["v.l", "@", x, "=", "v.l", "@", y] => read(x) == read(y),
            ["v.l", "@", x, "=", constant] => {
                read(x) == constant.parse::<i64>().expect("an integer")
            }
            ["v.h", "@", x, "!=", "v.h", "@", y] => secret(x) != secret(y),
            other => panic!("unmodelled atom {other:?}"),
        };
        Ok(if holds {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// The operation a step label names: its operation text, `None` for the
/// stutter step.
fn operation_of<'a>(label: &StepLabel<'a>) -> Option<&'a str> {
    match label {
        StepLabel::Operation(transition) => Some(transition.operation()),
        StepLabel::Stutter => None,
    }
}

/// Whether the operation text names `Vault::step`: the text is the canonical
/// document of the operation's declaration identity.
fn is_step(operation: &str) -> bool {
    operation.contains("/Vault/step\"")
}

/// The argument `i` of a `step` label: `Some` exactly when the step is a
/// `step`, and its integer.
pub(crate) fn input_of(label: &StepLabel<'_>) -> Option<i64> {
    match label {
        StepLabel::Operation(transition) if is_step(transition.operation()) => {
            let [argument] = transition.arguments() else {
                panic!("step takes one argument: {:?}", transition.arguments());
            };
            let value = argument
                .split("\"value\":\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .unwrap_or_else(|| panic!("{argument} is not an integer"));
            Some(value.parse().expect("an integer input"))
        }
        _ => None,
    }
}

impl Seams<'_> {
    /// The reading of the `match` conjuncts `texts` over `steps`, in order:
    /// the first that fails or has no value decides it.
    fn read(&self, texts: &[&str], steps: &[StepLabel<'_>]) -> MatchReading {
        for (index, text) in texts.iter().enumerate() {
            let words: Vec<&str> = text.split_whitespace().collect();
            let label = |name: &str| {
                &steps[self
                    .world
                    .component(name.split('.').next().expect("a variable"))]
            };
            let holds = match words.as_slice() {
                [x, "=", y] if x.ends_with(".step.op") && y.ends_with(".step.op") => {
                    operation_of(label(x)) == operation_of(label(y))
                }
                [x, "=", y]
                    if x.ends_with(".step.Vault::step.i") && y.ends_with(".step.Vault::step.i") =>
                {
                    input_of(label(x)) == input_of(label(y))
                }
                [x, "!=", y]
                    if x.ends_with(".step.Vault::step.i") && y.ends_with(".step.Vault::step.i") =>
                {
                    input_of(label(x)) != input_of(label(y))
                }
                [x, "=", "V::Vault::step"] if x.ends_with(".step.op") => {
                    operation_of(label(x)).is_some_and(is_step)
                }
                [x, "=", "stutter"] if x.ends_with(".step.op") => operation_of(label(x)).is_none(),
                [x, "!=", "stutter"] if x.ends_with(".step.op") => operation_of(label(x)).is_some(),
                // The inputs agree where both steps are `step`s (the conjunct is the
                // parenthesised conditional, which the span reads without them).
                ["if", present_x, "and", present_y, "then", value_x, "=", value_y, "else", "true"]
                    if present_x.starts_with("present(")
                        && present_y.starts_with("present(")
                        && value_x.starts_with("value(")
                        && value_y.starts_with("value(") =>
                {
                    let variable = |present: &str| {
                        present
                            .trim_start_matches("present(")
                            .split('.')
                            .next()
                            .expect("a variable")
                            .to_owned()
                    };
                    match (
                        input_of(label(&variable(present_x))),
                        input_of(label(&variable(present_y))),
                    ) {
                        (Some(left), Some(right)) => left == right,
                        _ => true,
                    }
                }
                // The input is 0 where there is one.
                ["(if", present, "then", value, "else", "0)", "=", "0"]
                    if present.starts_with("present(") && value.starts_with("value(") =>
                {
                    let variable = present
                        .trim_start_matches("present(")
                        .split('.')
                        .next()
                        .expect("a variable");
                    input_of(label(variable)).is_none_or(|input| input == 0)
                }
                ["(if", present, "then", "1", "/", value, "else", "1)", "=", "1"]
                    if present.starts_with("present(") && value.starts_with("value(") =>
                {
                    let variable = present
                        .trim_start_matches("present(")
                        .split('.')
                        .next()
                        .expect("a variable");
                    match input_of(label(variable)) {
                        Some(0) => {
                            return MatchReading::Undefined {
                                conjunct: index,
                                cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
                            }
                        }
                        Some(input) => 1 / input == 1,
                        None => true,
                    }
                }
                other => panic!("unmodelled match conjunct {other:?}"),
            };
            if !holds {
                return MatchReading::Fails;
            }
        }
        MatchReading::Holds
    }
}

impl StepMatch for Seams<'_> {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        let texts: Vec<&str> = (0..self.world.clause.mu_universal().len())
            .map(|index| self.world.conjunct_text(index))
            .collect();
        Ok(self.read(&texts, steps))
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        let texts: Vec<&str> = (0..self.world.clause.mu_existential().len())
            .map(|index| self.world.existential_text(index))
            .collect();
        Ok(self.read(&texts, steps))
    }
}

/// ADR-023 section 1's `NonInterference`, with `match` when asked.
pub(crate) fn non_interference(matched: &str) -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V {matched} \
         {{ always holds(v.l @ a = v.l @ b) }}"
    )
}

/// ADR-023 section 8.2's `Opaque`, with `conjuncts` added to its `match`.
pub(crate) fn opaque(conjuncts: &str) -> String {
    format!(
        "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match {{ a.step.Vault::step.i = b.step.Vault::step.i{conjuncts} }} \
         {{ holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }}"
    )
}

/// The `match` of `NonInterference`: equal operations with equal inputs.
pub(crate) const EQUAL_INPUTS: &str =
    "match { a.step.op = b.step.op and a.step.Vault::step.i = b.step.Vault::step.i }";

/// The two initial states of ADR-023 section 8's vault, `(h, l)`.
pub(crate) const STARTS: [(i64, i64); 2] = [(0, 0), (1, 0)];

/// The product of `world`'s clause explored in full, for a test that reads
/// the retained graph itself: the graph, and the automaton's acceptance sets
/// as the states of the graph each holds.
pub(crate) struct Explored {
    pub(crate) graph: RetainedGraph<HyperState, HyperTransition, HyperFinding>,
    pub(crate) accepting: Vec<Vec<usize>>,
}

/// The one instance of `NonInterference`'s object parameter `v`: the vault.
pub(crate) fn instance() -> [ParameterBinding; 1] {
    [ParameterBinding {
        parameter: "v".to_owned(),
        instance: ModelInstance {
            population: key("vaults"),
            key: KEY.to_owned(),
        },
    }]
}

impl World {
    /// Replay `counterexample` through the vault's system (FR-183).
    pub(crate) fn replay(
        &self,
        counterexample: &HyperCounterexample,
    ) -> Result<ModelTraceSettlement, ReplayRefusal> {
        self.replay_under(counterexample, DEFAULT_MAX_WITNESS_SET)
    }

    /// Replay `counterexample` with witness sets of at most `max_witness_set`
    /// members (FR-184).
    pub(crate) fn replay_under(
        &self,
        counterexample: &HyperCounterexample,
        max_witness_set: u64,
    ) -> Result<ModelTraceSettlement, ReplayRefusal> {
        let system = ModelSystem::new(self.subject().simulation).expect("the subject admits");
        let seams = self.seams();
        let instance = instance();
        replay_hyper_trace_tuple(
            &HyperReplay {
                clause: &self.clause,
                systems: vec![&system; self.clause.quantifiers().len()],
                instance: &instance,
                letters: &seams,
                matcher: &seams,
                meter: UNLIMITED,
                max_witness_set,
                cancel: &Cancel::new(),
            },
            counterexample,
        )
    }

    /// Explore the self-composition product of the clause (FR-176) with the
    /// engine's own exploration, with no limit.
    pub(crate) fn explore(&self) -> Explored {
        let system = ModelSystem::new(self.subject().simulation).expect("the subject admits");
        let seams = self.seams();
        let cancel = Cancel::new();
        let instance = instance();
        let mut product = HyperProduct::new(HyperInputs {
            systems: vec![&system; self.clause.quantifiers().len()],
            instance: &instance,
            body: self.clause.body(),
            profile: self.clause.profile(),
            letters: &seams,
            matcher: (!self.clause.mu_universal().is_empty()).then_some(&seams as &dyn StepMatch),
            max_automaton_states: u64::MAX,
            meter: UNLIMITED,
            cancel: &cancel,
            copy_swap: None,
        })
        .expect("the negated body translates");
        product.start().expect("the initial states read");
        let exploration = retain(
            &product,
            Limits {
                max_states: usize::MAX,
                max_transitions: usize::MAX,
            },
            IdentityLimits::default(),
            None,
            || false,
        )
        .expect("the keys encode");
        let automaton = product.automaton();
        let accepting = (0..automaton.acceptance_set_count())
            .map(|set| {
                exploration
                    .graph
                    .states()
                    .iter()
                    .enumerate()
                    .filter(|(_, state)| {
                        matches!(&state.state.property,
                            Property::After(at) if automaton.is_accepting_in(set, *at))
                    })
                    .map(|(index, _)| index)
                    .collect()
            })
            .collect();
        drop(automaton);
        Explored {
            graph: exploration.graph,
            accepting,
        }
    }
}

/// The witness-set product of `world`'s HP-3 clause explored in full: the
/// graph, and the number of states the safety automaton reached.
pub(crate) struct WitnessExplored {
    pub(crate) graph: RetainedGraph<HyperState, HyperTransition, HyperFinding>,
    pub(crate) automaton_states: usize,
}

impl World {
    /// Explore the witness-set product of the clause (FR-177) with the
    /// engine's own exploration, with no limit.
    pub(crate) fn explore_witness(&self) -> WitnessExplored {
        let system = ModelSystem::new(self.subject().simulation).expect("the subject admits");
        let seams = self.seams();
        let cancel = Cancel::new();
        let instance = instance();
        let mut product = WitnessProduct::new(WitnessInputs {
            systems: vec![&system; self.clause.quantifiers().len()],
            universal: self.clause.universals(),
            instance: &instance,
            body: self.clause.body(),
            profile: self.clause.profile(),
            letters: &seams,
            universal_match: (!self.clause.mu_universal().is_empty())
                .then_some(&seams as &dyn StepMatch),
            existential_match: (!self.clause.mu_existential().is_empty())
                .then_some(&seams as &dyn StepMatch),
            max_automaton_states: u64::MAX,
            max_witness_set: u64::MAX,
            meter: UNLIMITED,
            cancel: &cancel,
        })
        .expect("the body translates to a safety automaton");
        product.start().expect("the initial states read");
        let exploration = retain(
            &product,
            Limits {
                max_states: usize::MAX,
                max_transitions: usize::MAX,
            },
            IdentityLimits::default(),
            None,
            || false,
        )
        .expect("the keys encode");
        let automaton_states = product.automaton().reached_states();
        WitnessExplored {
            graph: exploration.graph,
            automaton_states,
        }
    }
}

impl Explored {
    /// The number of strongly connected components of the graph that hold a
    /// cycle through a state of every acceptance set, with no fairness
    /// constraint: ADR-023 section 8.1's accepting SCCs.
    pub(crate) fn accepting_components(&self) -> usize {
        let mut graph = Graph::new();
        let vertices: Vec<_> = self
            .graph
            .states()
            .iter()
            .map(|_| graph.add_vertex([]))
            .collect();
        for edge in self.graph.edges() {
            graph
                .add_edge(
                    vertices[edge.source.index()],
                    vertices[edge.target.index()],
                    EdgeLabel::Transition(Identity::new(0)),
                )
                .expect("both ends exist");
        }
        for set in &self.accepting {
            graph
                .add_acceptance_set(set.iter().map(|index| vertices[*index]))
                .expect("members exist");
        }
        let all: BTreeSet<Vertex> = graph.vertices().collect();
        analyze(&graph, &all, &[], &Cancel::new())
            .expect("the graph analyses")
            .iter()
            .filter(|analysis| *analysis.outcome() == FairnessOutcome::Fair)
            .count()
    }
}
