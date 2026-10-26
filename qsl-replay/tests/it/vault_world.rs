// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-023 section 8's secure vault as a checked world for the hyper identity
//! tests: the `test/vault` domain package (object type `Vault`, secret `h` and
//! public output `l` over `Int[0, 1]`, population `vaults`, operations
//! `step(i)` and `reset()`), the same package again as a second model under
//! alias `N`, and a unit that declares one hyper clause.

use std::collections::BTreeMap;

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use qsl_foundation::bound::ProofBound;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    check_clauses, check_hyper_clauses, CheckedHyper, CheckedHyperClause, CheckedTemporalClause,
    PackageDeclarations,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_checked::key::DeclarationKey;
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

const PACKAGE: &str = "test/vault";
const OTHER: &str = "test/vault-other";
const OTHER_ALIAS: &str = "N";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

/// ADR-023 section 1's `NonInterference`, with `match` when asked.
pub fn non_interference(matched: &str, fairness: &str) -> String {
    with_body(matched, fairness, "always holds(v.l @ a = v.l @ b)")
}

/// `NonInterference` with `body`.
pub fn with_body(matched: &str, fairness: &str, body: &str) -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::Vault) \
         forall trace a of V {fairness} forall trace b of V {fairness} {matched} \
         {{ {body} }}"
    )
}

/// The body of FR-315-AC-43: in the safety fragment, not `always` over a
/// state predicate.
pub const MONITORED: &str =
    "always (holds(v.h @ a = v.h @ b) implies always holds(v.l @ a = v.l @ b))";

/// How the world's `Vault` is declared: with `reset` and the contract of
/// `step`.
pub struct Build<'a> {
    /// Whether `Vault` has FR-176-AC-3's `reset`.
    pub reset: bool,
    /// The postcondition of `step`, over `self` and `i`.
    pub post: &'a str,
    /// Further declarations of the unit.
    pub extra: &'a str,
}

/// ADR-023 section 8's secure vault: `step(i)` sets `l` to `i`.
pub const SECURE: Build<'static> = Build {
    reset: false,
    post: "self.l = i",
    extra: "",
};

/// The `match` of `NonInterference`: equal operations with equal inputs.
pub const EQUAL_INPUTS: &str =
    "match { a.step.op = b.step.op and a.step.Vault::step.i = b.step.Vault::step.i }";

/// ADR-023 section 1's `SavesPower`, over two models.
pub const SAVES_POWER: &str =
    "hyper SavesPower using inf over behaviours (d: V::Vault, e: N::Vault) \
     forall trace s of V forall trace n of N \
     match { s.step.Vault::step.i = n.step.Vault::step.i } \
     { always holds(d.l @ s = e.l @ n) }";

/// An HP-3 clause.
pub const FORALL_EXISTS: &str = "hyper Exists using inf over behaviours (v: V::Vault) \
     forall trace a of V exists trace b of V { always holds(v.l @ a = v.l @ b) }";

/// A liveness body (FR-176-AC-3).
pub const LIVENESS: &str = "hyper Live using inf over behaviours (v: V::Vault) \
     forall trace a of V { always eventually holds(v.l @ a = 0) }";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

pub(crate) fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

/// The declaration `suffix` of the second model.
fn other_key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: OTHER.to_owned(),
        node: format!("ix://{OTHER}/{suffix}"),
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
    second: Option<&[(&str, (i64, i64))]>,
) -> DocumentRef {
    let digest = *package_input([document]).keys().next().expect("one model");
    let header = json!({"identity": PACKAGE, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    let population = |package: &str, vaults: &[(&str, (i64, i64))]| {
        let ident = |suffix: &str| format!("ix://{package}/{suffix}");
        json!({"population": ident("vaults"), "complete": true,
            "objects": vaults.iter().map(|(key, (h, l))| {
                let mut fields = json!({"h": {"integer": h.to_string()},
                    "l": {"integer": l.to_string()}});
                if let Some(busy) = busy {
                    fields["busy"] = json!({"integer": busy.to_string()});
                }
                json!({"key": key, "type": ident("Vault"), "fields": fields})
            }).collect::<Vec<_>>()})
    };
    let populations: Vec<Json> = std::iter::once(population(PACKAGE, vaults))
        .chain(second.map(|second| population(OTHER, second)))
        .collect();
    let body = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": name, "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": populations,
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

/// The unit text: the header selecting the domain package as `V` (and the
/// second as `N`), the secure `step` contract, and `body`.
fn unit_text(document: &[u8], other: Option<&[u8]>, build: &Build<'_>, body: &str) -> String {
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
         {second}function Noop using v(): Boolean pure {{ true }}\n\
         post Step using v on V::Vault::step {{ {} }}\n{}\n{body}",
        hex(&digest),
        build.post,
        build.extra,
    )
}

/// A checked vault world with one hyper clause.
pub struct World {
    package: CheckedPackage,
    temporal_clause: Option<CheckedTemporalClause>,
    /// The same clause as S3 checked it, before any package assembled it.
    pub unassembled: CheckedHyperClause,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    /// A provision that holds no document.
    no_snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: Vec<DocumentRef>,
    /// The keys of the universe of `vaults`.
    pub universe: Vec<String>,
    /// The keys of the universe of the second model's `vaults`, when it has
    /// one.
    pub other_universe: Option<Vec<String>>,
    /// The subject of alias `N` of a world built by [`World::two_aliases`]:
    /// its own universes and initial states.
    alias_n: Option<AliasN>,
}

/// The universes and initial states of the second alias's subject.
struct AliasN {
    universe: Vec<String>,
    other_universe: Vec<String>,
    initial: Vec<DocumentRef>,
}

impl World {
    /// The vault started from `(h, l) = (0, 0)` and `(1, 0)`, with `vaults`
    /// the universe, and the hyper clause `declaration`; the vault is also the
    /// second model under alias `N` when `second`.
    pub fn new(universe: &[&str], second: bool, declaration: &str) -> Self {
        let build = Build {
            reset: true,
            ..SECURE
        };
        Self::build(universe, second, declaration, &build)
    }

    /// As [`World::new`], with the vault declared as `build` says.
    pub fn build(universe: &[&str], second: bool, declaration: &str, build: &Build<'_>) -> Self {
        Self::assemble(
            universe,
            second.then_some(universe),
            None,
            declaration,
            build,
        )
    }

    /// A clause over two aliases whose subjects have disjoint universes:
    /// alias `V`'s subject holds the vaults `v` of `V` and `n` of `N`, alias
    /// `N`'s subject the vaults `v` of `V` and `m` of `N`, each from
    /// `(h, l) = (0, 0)` and `(1, 0)`.
    pub fn two_aliases(declaration: &str, build: &Build<'_>) -> Self {
        Self::assemble(
            &["v"],
            Some(&["n"]),
            Some((&["v"], &["m"])),
            declaration,
            build,
        )
    }

    fn assemble(
        universe: &[&str],
        other_universe: Option<&[&str]>,
        alias_n: Option<(&[&str], &[&str])>,
        declaration: &str,
        build: &Build<'_>,
    ) -> Self {
        let second = other_universe.is_some();
        let document = document(false, build.reset);
        let other = second.then(|| other_document(&document));
        let other = other.as_deref();
        let documents: Vec<&[u8]> = std::iter::once(document.as_slice()).chain(other).collect();
        let text = unit_text(&document, other, build, &format!("{declaration}\n"));
        let assembled = declarations(&documents, &text);
        let temporal_clause = check_clauses(&assembled)
            .into_iter()
            .next()
            .map(|checked| checked.expect("the temporal clause checks"));
        let graph = assembled
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
        let package = CheckedPackage::link(graph);
        let unassembled = match check_hyper_clauses(&declarations(&documents, &text))
            .into_iter()
            .next()
        {
            Some(Ok(CheckedHyper::Behaviours(clause))) => *clause,
            other => panic!("{declaration}: a checked hyper clause, not {other:?}"),
        };
        let mut snapshots = BTreeMap::new();
        let starts = [(0, 0), (1, 0)];
        let mut initials = |name: &str, universe: &[&str], other_universe: Option<&[&str]>| {
            starts
                .iter()
                .enumerate()
                .map(|(index, start)| {
                    let vaults: Vec<(&str, (i64, i64))> =
                        universe.iter().map(|key| (*key, *start)).collect();
                    let others: Option<Vec<(&str, (i64, i64))>> =
                        other_universe.map(|keys| keys.iter().map(|key| (*key, *start)).collect());
                    snapshot(
                        &document,
                        &mut snapshots,
                        &format!("{name}{index}"),
                        &vaults,
                        None,
                        others.as_deref(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let initial = initials("s", universe, other_universe);
        let alias_n = alias_n.map(|(universe, other_universe)| AliasN {
            universe: universe.iter().map(|key| (*key).to_owned()).collect(),
            other_universe: other_universe.iter().map(|key| (*key).to_owned()).collect(),
            initial: initials("n", universe, Some(other_universe)),
        });
        let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>();
        Self {
            package,
            temporal_clause,
            unassembled,
            packages: package_input(documents.iter().copied()),
            snapshots,
            no_snapshots: BTreeMap::new(),
            initial,
            universe: keys(universe),
            other_universe: other_universe.map(keys),
            alias_n,
        }
    }

    /// The world with an empty initial list.
    pub fn without_initial_states(mut self) -> Self {
        self.initial.clear();
        self
    }

    /// The clause as the world's package assembled it.
    pub fn clause(&self) -> &CheckedHyperClause {
        &self.package.graph().hyper_clauses()[0]
    }

    /// The package the subjects are over.
    pub fn package(&self) -> &CheckedPackage {
        &self.package
    }

    /// The vault's subject over `package`, with `bounds` as its proof bounds.
    pub fn subject_over<'a>(
        &'a self,
        package: &'a CheckedPackage,
        bounds: Vec<ProofBound>,
    ) -> ModelSubject<'a> {
        self.subject_with(
            package,
            bounds,
            &self.initial,
            &self.universe,
            self.other_universe.as_ref(),
        )
    }

    /// The subject of alias `N` of a [`World::two_aliases`] world.
    pub fn subject_n(&self) -> ModelSubject<'_> {
        let alias = self.alias_n.as_ref().expect("a two-alias world");
        self.subject_with(
            &self.package,
            Vec::new(),
            &alias.initial,
            &alias.universe,
            Some(&alias.other_universe),
        )
    }

    fn subject_with<'a>(
        &'a self,
        package: &'a CheckedPackage,
        bounds: Vec<ProofBound>,
        initial: &[DocumentRef],
        universe: &'a Vec<String>,
        other_universe: Option<&'a Vec<String>>,
    ) -> ModelSubject<'a> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: initial.to_vec(),
                universes: [(key("vaults"), Some(universe))]
                    .into_iter()
                    .chain([(other_key("vaults"), other_universe)])
                    .filter_map(|(population, keys)| {
                        keys.map(|keys| PopulationUniverse {
                            population,
                            keys: keys.clone(),
                        })
                    })
                    .collect(),
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
            },
            proof_bounds: bounds,
        }
    }

    /// The vault's subject over a `snapshots` provision that lacks the
    /// document an initial state names, which FR-120 does not admit.
    pub fn unadmitted_subject(&self) -> ModelSubject<'_> {
        let mut subject = self.subject();
        subject.simulation.snapshots = &self.no_snapshots;
        subject
    }

    /// The vault's subject.
    pub fn subject(&self) -> ModelSubject<'_> {
        self.subject_over(&self.package, Vec::new())
    }
}

/// The TP-4 closure replay fixtures (FR-338-AC-24, FR-338-AC-25).
mod tp4 {
    use std::collections::{BTreeMap, BTreeSet};

    use qsl_eval::automaton::{
        translate, BuchiState, BuchiStates, ContextBinding, Letter, StateContext, SubjectKind,
    };
    use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
    use qsl_eval::simulation::model::ModelSimState;
    use qsl_eval::simulation::model::ModelSystem;
    use qsl_eval::temporal::AtomReading;
    use qsl_foundation::digest::WireNodeId;
    use qsl_foundation::source::provenance::OccurrenceKey;
    use qsl_replay::ClauseIdentity;
    use qsl_replay::{
        check_components, CertificateLocus, CertificateRequest, CertificateRule,
        ClosureCertificate, ComponentCertificate, ModelCheckItem, ModelCheckLimits,
        TemporalCertificateCheck,
    };
    use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
    use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
    use quire_exact::{Cancel, Meter, Origin, Role};
    use quire_exact::{Value, ValueType};

    use super::{World, FORALL_EXISTS};

    /// Every `holds` atom is true at every position, so the engine walks a
    /// path whose states differ by the past history it has retained.
    struct TrueLetters;

    impl ModelLetters for TrueLetters {
        fn atom(
            &self,
            _atom: FormulaNodeId,
            _instance: &ModelInstance,
            _state: &ModelSimState,
            _meter: &mut Meter,
        ) -> Result<AtomReading, AtomStop> {
            Ok(AtomReading::True)
        }

        fn activation(
            &self,
            _instance: &ModelInstance,
            _state: &ModelSimState,
            _meter: &mut Meter,
        ) -> Result<AtomReading, AtomStop> {
            Ok(AtomReading::True)
        }

        fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
            Ok(BTreeSet::new())
        }

        fn reads(
            &self,
            _atom: FormulaNodeId,
        ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
            Ok(BTreeSet::new())
        }
    }

    static LETTERS: TrueLetters = TrueLetters;

    /// A TP-4 liveness clause whose reached automaton states carry several
    /// distinct keys: the past operand's retained history differs between
    /// the first position and the later ones.
    const BODY: &str = "always eventually (holds(v.l = 0) and once[1,1] holds(v.l = 0))";

    pub struct Case {
        world: World,
        package_id: qsl_semantics::library::PackageId,
    }

    impl Case {
        pub fn new() -> Self {
            let temporal = format!(
                "temporal Live using inf over (v: V::Vault) clock \"model-steps\" on origin {{ {BODY} }}"
            );
            let world = World::new(&["v"], false, &format!("{FORALL_EXISTS}\n{temporal}"));
            let package_id = qsl_package::emit_checked(world.package())
                .expect("the checked package emits")
                .package()
                .package_id();
            Self { world, package_id }
        }

        pub fn clause(&self) -> &CheckedTemporalClause {
            self.world
                .temporal_clause
                .as_ref()
                .expect("the checked package contains the TP-4 clause")
        }

        pub fn request(&self) -> CertificateRequest<'_> {
            CertificateRequest {
                subject: self.world.subject(),
                item: ModelCheckItem::Clause {
                    clause: self.clause(),
                    letters: &LETTERS,
                },
                smt_encoding: Default::default(),
            }
        }

        /// Check `certificate` as the closure of a component certificate: a
        /// TP-4 clause is proved only by a component certificate (FR-339), and
        /// a plain closure for it is refused `LivenessNeedsComponents`. The
        /// closure is checked first (FR-338), so a certificate with no
        /// components is rejected at the closure's own rules while the closure
        /// is incomplete, and `NotPartition` once the closure is accepted.
        pub fn check(
            &self,
            certificate: &ClosureCertificate,
        ) -> Result<TemporalCertificateCheck, qsl_replay::ClosureRefusal> {
            check_components(
                &self.request(),
                &ComponentCertificate {
                    closure: certificate.clone(),
                    components: Vec::new(),
                },
                &ModelCheckLimits::default(),
                &Cancel::new(),
            )
        }

        /// The closure the checker accepts, grown from the checker's named
        /// product states, each key of which must be one the engine replay
        /// reached. The closure is accepted when only the (absent) component
        /// partition remains to reject (`NotPartition`).
        pub fn certificate(&self) -> ClosureCertificate {
            let keys = self.expected_keys();
            let mut certificate = ClosureCertificate {
                package_id: self.package_id,
                states: Vec::new(),
            };
            // The checker names each absent product state (its model-state
            // digest is the model's identity, which this test does not
            // recompute); the automaton key of every one is checked against
            // the independent replay, and the key set is pinned by the tests.
            for _ in 0..512 {
                match self.check(&certificate).expect("the request admits") {
                    TemporalCertificateCheck::Rejected(rejection)
                        if rejection.rule == CertificateRule::NotPartition =>
                    {
                        return certificate;
                    }
                    TemporalCertificateCheck::Rejected(rejection) => {
                        let CertificateLocus::ProductState(state) = rejection.at else {
                            panic!("TP-4 closure failures name product states");
                        };
                        assert!(
                            matches!(
                                rejection.rule,
                                CertificateRule::InitialMissing | CertificateRule::SuccessorMissing
                            ),
                            "unexpected rejection {:?}",
                            rejection.rule
                        );
                        assert!(
                            keys.contains_key(state.automaton_state.as_bytes()),
                            "the checker named a key the independent replay did not reach"
                        );
                        certificate.states.push(state);
                        certificate.states.sort();
                        certificate.states.dedup();
                    }
                    other => panic!("unexpected TP-4 closure result: {other:?}"),
                }
            }
            panic!("the reachable product closure is finite");
        }

        /// The key context the checker builds for the clause's one instance:
        /// the package's own clause identity and the instance's reference.
        fn context(&self) -> StateContext {
            let subject = self.world.subject();
            let identity = ClauseIdentity::resolve(
                subject.simulation.package,
                subject.simulation.packages,
                self.clause(),
                &Cancel::new(),
            )
            .expect("the clause resolves in its package");
            let system = ModelSystem::new(subject.simulation).expect("the model admits");
            let over = &self.clause().over().1;
            let instance = system
                .instances(over)
                .expect("the population is bounded")
                .into_iter()
                .next()
                .expect("one instance");
            let ValueType::Reference(object_type) = over else {
                panic!("the clause ranges over a reference");
            };
            let reference = system
                .bound_reference(*object_type, &instance)
                .expect("the instance has a reference");
            StateContext::clause(
                self.clause(),
                identity.occurrence().clone(),
                vec![ContextBinding {
                    parameter: identity.over().expect("the clause has an over node"),
                    value: Value::Reference(reference),
                }],
            )
        }

        pub fn keys(&self) -> BuchiStates {
            BuchiStates::clause(self.clause(), &self.context(), SubjectKind::Ordinary)
                .expect("the TP-4 key context translates")
        }

        /// Every automaton key at least one letter-step from the initial
        /// state under the all-true letter, by replaying the engine alone.
        pub fn expected_keys(&self) -> BTreeMap<Vec<u8>, BuchiState> {
            let keys = self.keys();
            let mut automaton = translate(
                self.clause(),
                ModelCheckLimits::default().max_automaton_states,
            )
            .expect("the clause translates");
            let letter = Letter::from_bits((1u64 << automaton.atoms().len()) - 1);
            let mut seen = BTreeSet::new();
            let mut reached = BTreeMap::new();
            let mut pending = vec![automaton.initial().expect("initial state fits")];
            while let Some(from) = pending.pop() {
                for to in automaton
                    .successors(from, letter)
                    .expect("successors materialize")
                {
                    if seen.insert(to) {
                        let structural = automaton
                            .buchi_state(to, keys.max_key_bytes())
                            .expect("the state resolves structurally");
                        let key = keys.key(&structural).expect("the state encodes");
                        reached.insert(key.as_bytes().to_vec(), structural);
                        pending.push(to);
                    }
                }
            }
            reached
        }

        pub fn foreign_keys(&self) -> BuchiStates {
            let identity = OccurrenceKey::new(
                WireNodeId::from_digest([0x53; 32]),
                Origin::new(Role::new("temporal"), 1),
            );
            let context = StateContext::clause(self.clause(), identity, Vec::new());
            BuchiStates::clause(self.clause(), &context, SubjectKind::Ordinary)
                .expect("the foreign TP-4 key context builds")
        }
    }

    fn assert_missing(case: &Case, certificate: &ClosureCertificate, what: &str) {
        match case.check(certificate).expect("the request admits") {
            TemporalCertificateCheck::Rejected(rejection) => assert!(
                matches!(
                    rejection.rule,
                    CertificateRule::InitialMissing | CertificateRule::SuccessorMissing
                ),
                "{what}: rejected {:?}",
                rejection.rule
            ),
            other => panic!("{what}: expected a rejection, got {other:?}"),
        }
    }

    /// FR-338-AC-24: the TP-4 certificate carries the keys of the structural
    /// states an independent replay of the engine reaches, with at least two
    /// distinct keys, and the checker accepts it.
    #[ix_trace_rs::trace("TC-525", "FR-338-AC-24")]
    #[test]
    fn tp4_closure_accepts_the_independently_replayed_keys() {
        let case = Case::new();
        let expected = case.expected_keys();
        assert!(
            expected.len() >= 2,
            "the fixture must reach distinct automaton keys"
        );
        let certificate = case.certificate();
        let carried: BTreeSet<Vec<u8>> = certificate
            .states
            .iter()
            .map(|state| state.automaton_state.as_bytes().to_vec())
            .collect();
        assert_eq!(carried, expected.keys().cloned().collect::<BTreeSet<_>>());
        // The closure is accepted: the only rule left to reject is the
        // component partition this fixture does not carry.
        assert!(matches!(
            case.check(&certificate),
            Ok(TemporalCertificateCheck::Rejected(rejection))
                if rejection.rule == CertificateRule::NotPartition
        ));
        // A plain closure for the TP-4 clause is refused outright (FR-339).
        assert!(matches!(
            qsl_replay::check_closure(
                &case.request(),
                &certificate,
                &ModelCheckLimits::default(),
                &Cancel::new(),
            ),
            Err(qsl_replay::ClosureRefusal::LivenessNeedsComponents)
        ));
    }

    /// FR-338-AC-24: a certificate whose key is replaced, swapped, rotated or
    /// dropped is rejected at a missing product state, never accepted.
    #[ix_trace_rs::trace("TC-525", "FR-338-AC-24")]
    #[test]
    fn tp4_closure_refuses_each_key_tamper() {
        let case = Case::new();
        let certificate = case.certificate();
        let states = &certificate.states;
        let key = |index: usize| states[index].automaton_state.clone();
        let (first, second) = (0..states.len())
            .flat_map(|first| (0..states.len()).map(move |second| (first, second)))
            .find(|&(first, second)| key(first) != key(second))
            .expect("two product states carry distinct keys");

        let mut replaced = certificate.clone();
        replaced.states[first].automaton_state = key(second);
        assert_missing(&case, &replaced, "replacing one state's key");

        // A swap of two keys of one model state leaves the same set, so the
        // pair is one whose swap changes the set.
        let swap = |first: usize, second: usize| {
            let mut swapped = certificate.clone();
            swapped.states[first].automaton_state = key(second);
            swapped.states[second].automaton_state = key(first);
            swapped
        };
        let as_set = |certificate: &ClosureCertificate| {
            certificate.states.iter().cloned().collect::<BTreeSet<_>>()
        };
        let (a, b) = (0..states.len())
            .flat_map(|a| (0..states.len()).map(move |b| (a, b)))
            .find(|&(a, b)| as_set(&swap(a, b)) != as_set(&certificate))
            .expect("some swap changes the product-state set");
        assert_missing(&case, &swap(a, b), "swapping two states' keys");

        let mut rotated = certificate.clone();
        for (index, state) in rotated.states.iter_mut().enumerate() {
            state.automaton_state = key((index + 1) % states.len());
        }
        assert_ne!(rotated, certificate);
        assert_missing(&case, &rotated, "rotating every state's key");

        for index in 0..states.len() {
            let mut dropped = certificate.clone();
            dropped.states.remove(index);
            assert_missing(&case, &dropped, "dropping a product state");
        }
    }

    /// FR-338-AC-25: a well-formed key of another clause context is rejected
    /// `ShapeMismatch` at its product state, before membership is checked.
    #[ix_trace_rs::trace("TC-525", "FR-338-AC-25")]
    #[test]
    fn tp4_closure_refuses_a_foreign_context_key() {
        let case = Case::new();
        let certificate = case.certificate();
        let (_, structural) = case.expected_keys().into_iter().next().expect("a key");
        let foreign_key = case
            .foreign_keys()
            .key(&structural)
            .expect("the foreign key is well formed");
        let mut foreign = certificate.clone();
        foreign.states[0].automaton_state = foreign_key;
        let at = foreign.states[0].clone();
        assert!(matches!(
            case.check(&foreign),
            Ok(TemporalCertificateCheck::Rejected(rejection))
                if rejection.rule == CertificateRule::ShapeMismatch
                    && rejection.at == CertificateLocus::ProductState(at)
        ));
    }
}
