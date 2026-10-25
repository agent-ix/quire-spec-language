// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 (TC-900): `encode_smt_query`, the canonical SMT-LIB 2 encoding of a
//! model subject's transition relation and of a temporal item's proof
//! queries, over the `Counter` and `Slot` subjects.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use qsl_eval::smt::{
    encode_smt_query, Measure, Refusal, SmtEncodingLimits, SmtEncodingRefusal, SmtItem,
    SmtQueryKind,
};
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedTemporalClause, PackageDeclarations, TemporalProfile};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::hex;
use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::fairness::PropertyForm;
use serde_json::{json, Value as Json};

const PACKAGE: &str = "test/smt";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const BOUNDED: &str = "quire.temporal.event-position.false-extension/v1";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
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

/// `Level`: `Int[0, 3]`.
fn level() -> Json {
    let identity = ident("Level");
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
            "constraints": [constraint("min", "0"), constraint("max", "3")]}),
    )
}

fn field(object: &str, name: &str, type_ref: &str, presence: &str) -> Json {
    let identity = ident(&format!("{object}/{name}"));
    json!({
        "identity": identity, "name": name, "typeRef": type_ref, "presence": presence,
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&identity),
    })
}

fn operation(object: &str, name: &str, modifies: &[&str]) -> Json {
    operation_with(object, name, &[], None, modifies, false)
}

/// An operation with `params` (name, type), an optional `result` type and a
/// frame that modifies `modifies` and, when `creates`, creates its object.
fn operation_with(
    object: &str,
    name: &str,
    params: &[(&str, String)],
    result: Option<String>,
    modifies: &[&str],
    creates: bool,
) -> Json {
    let identity = ident(&format!("{object}/{name}"));
    let params: Vec<Json> = params
        .iter()
        .map(|(parameter, type_ref)| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({
                "identity": parameter_identity, "name": parameter, "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&parameter_identity),
            })
        })
        .collect();
    let mut node = json!({
        "identity": identity, "name": name, "params": params, "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": modifies.iter()
            .map(|field| ident(&format!("{object}/{field}"))).collect::<Vec<_>>(),
            "creates": if creates { vec![ident(object)] } else { Vec::new() },
            "deletes": []},
    });
    if let Some(type_ref) = result {
        node["returns"] = json!({"typeRef": type_ref, "nullable": false,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}});
    }
    node
}

/// The domain package: one object type `object` with `fields` and
/// `operations`, and the population `objects` of it.
fn document(object: &str, fields: Vec<Json>, operations: Vec<Json>) -> Vec<u8> {
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
    let object_identity = ident(object);
    let population = ident("objects");
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
            level(),
            wire_type(&object_identity, "object_type", json!({
                "supertypes": [], "fields": fields, "operations": operations,
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [object_identity], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// A checked package over a domain, its clauses and the snapshot of its
/// initial state.
struct World {
    /// The keys of the start objects: the universe of the default query.
    keys: Vec<String>,
    package: CheckedPackage,
    clauses: Vec<CheckedTemporalClause>,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    initial: DocumentRef,
}

/// The unit selecting `document` as `M` with `contracts` and, under the
/// profile `profile`, the clauses `formulas` over `variable: M::object`,
/// from one initial snapshot of `start`, each `(key, fields)`.
fn world(
    (object, variable): (&str, &str),
    document: &[u8],
    contracts: &str,
    profile: &str,
    formulas: &[&str],
    start: &[(&str, Json)],
) -> World {
    let packages = package_input([document]);
    let digest = *packages.keys().next().expect("one model");
    let clauses: String = formulas
        .iter()
        .enumerate()
        .map(|(index, formula)| {
            format!(
                "temporal C{index} using t over ({variable}: M::{object}) clock \"model-steps\" on origin {{ {formula} }}\n"
            )
        })
        .collect();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{contracts}{clauses}",
        hex(&digest)
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "smt", "git", "1"),
        "smt.native",
        unit.as_bytes(),
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
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .with_unwritten_terminals()
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{unit}: the unit checks: {refusals:?}"));
    let clauses: Vec<CheckedTemporalClause> = graph
        .temporal_clauses()
        .iter()
        .map(|node| node.clause().clone())
        .collect();
    assert_eq!(clauses.len(), formulas.len(), "{unit}");

    let objects: Vec<Json> = start
        .iter()
        .map(|(key, fields)| json!({"key": key, "type": ident(object), "fields": fields}))
        .collect();
    let header = json!({"identity": PACKAGE, "version": "1.0.0",
        "digest": format!("sha256-jcs:{}", hex(&digest))});
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0", "revision_namespace": "fixture",
            "revision": "fixture:1"},
        "observation": "current", "model": header,
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("objects"), "complete": true,
            "objects": objects}],
    });
    let bytes = snapshot.to_string().into_bytes();
    let read = quire_canonical::read(&bytes, u64::MAX).expect("JSON fixture");
    let snapshot_digest = *quire_canonical::sha256(&read, quire_canonical::Limits::new(u64::MAX))
        .expect("canonical fixture")
        .as_bytes();
    World {
        keys: start.iter().map(|(key, _)| (*key).to_owned()).collect(),
        package: CheckedPackage::link(graph),
        clauses,
        packages,
        snapshots: BTreeMap::from([(snapshot_digest, bytes)]),
        initial: DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "fixture:1".to_owned(),
            digest: snapshot_digest,
        },
    }
}

const UNLIMITED: ScalarLimits = ScalarLimits {
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

impl World {
    fn subject(&self, keys: &[&str]) -> ModelSubject<'_> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package: &self.package,
                packages: &self.packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &self.snapshots,
                initial: vec![self.initial.clone()],
                universes: vec![PopulationUniverse {
                    population: DeclarationKey {
                        package: PACKAGE.to_owned(),
                        node: ident("objects"),
                    },
                    keys: keys.iter().map(|key| (*key).to_owned()).collect(),
                }],
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
            },
            proof_bounds: Vec::new(),
        }
    }

    fn query(&self, clause: usize, kind: SmtQueryKind) -> Result<String, SmtEncodingRefusal> {
        self.query_with_limits(clause, kind, &SmtEncodingLimits::default())
    }

    fn query_with_limits(
        &self,
        clause: usize,
        kind: SmtQueryKind,
        limits: &SmtEncodingLimits,
    ) -> Result<String, SmtEncodingRefusal> {
        let keys: Vec<&str> = self.keys.iter().map(String::as_str).collect();
        encode_smt_query(
            &self.subject(&keys),
            SmtItem::Clause(&self.clauses[clause]),
            kind,
            limits,
        )
        .map(|script| String::from_utf8(script).expect("a script is UTF-8"))
    }

    fn without_initial_state(&self, keys: &[&str]) -> ModelSubject<'_> {
        let mut subject = self.subject(keys);
        subject.simulation.initial.clear();
        subject
    }

    fn deadlock(&self, kind: SmtQueryKind) -> Result<String, SmtEncodingRefusal> {
        let keys: Vec<&str> = self.keys.iter().map(String::as_str).collect();
        encode_smt_query(
            &self.subject(&keys),
            SmtItem::DeadlockFreedom,
            kind,
            &SmtEncodingLimits::default(),
        )
        .map(|script| String::from_utf8(script).expect("a script is UTF-8"))
    }

    fn query_with(
        &self,
        keys: &[&str],
        clause: usize,
        kind: SmtQueryKind,
    ) -> Result<String, SmtEncodingRefusal> {
        encode_smt_query(
            &self.subject(keys),
            SmtItem::Clause(&self.clauses[clause]),
            kind,
            &SmtEncodingLimits::default(),
        )
        .map(|script| String::from_utf8(script).expect("a script is UTF-8"))
    }
}

/// FR-315-AC-44: temporal query encoding reads caller-supplied limits,
/// including zero, exact and one-short boundaries.
#[trace("TC-1250", "FR-315-AC-44")]
#[test]
fn encoding_limits_apply_at_zero_exact_and_one_short_values() {
    let world = counter(BOUNDED, &["always[0,3] holds(c.value <= 3)"]);
    let kind = SmtQueryKind::Unrolling { depth: 3 };
    let default = world.query(0, kind).expect("default limits encode");
    let mut limits = SmtEncodingLimits {
        max_constants: 8,
        ..SmtEncodingLimits::default()
    };
    assert_eq!(
        world.query_with_limits(0, kind, &limits),
        Ok(default.clone())
    );

    limits.max_applications = 1;
    assert_eq!(
        world.query_with_limits(0, kind, &limits),
        Ok(default.clone())
    );
    limits.max_applications = 0;
    assert!(matches!(
        world.query_with_limits(0, kind, &limits),
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::Applications,
            value: 1,
            limit: 0,
            setting: "smt_encoding.max_applications",
        }))
    ));

    limits.max_applications = 1 << 16;
    limits.max_candidates = 4;
    assert_eq!(
        world.query_with_limits(0, kind, &limits),
        Ok(default.clone())
    );
    limits.max_candidates = 3;
    assert!(matches!(
        world.query_with_limits(0, kind, &limits),
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::Candidates,
            value: 4,
            limit: 3,
            setting: "smt_encoding.max_candidates",
        }))
    ));

    for max_constants in [7, 0] {
        limits.max_constants = max_constants;
        assert!(matches!(
            world.query_with_limits(0, kind, &limits),
            Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
                measure: Measure::Constants,
                value: 8,
                limit,
                setting: "smt_encoding.max_constants",
            })) if limit == max_constants
        ));
    }

    let script_bytes = u64::try_from(default.len()).expect("script length fits u64");
    limits = SmtEncodingLimits {
        max_script_bytes: 0,
        ..SmtEncodingLimits::default()
    };
    assert!(matches!(
        world.query_with_limits(0, kind, &limits),
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::ScriptBytes,
            value: 1,
            limit: 0,
            setting: "smt_encoding.max_script_bytes",
        }))
    ));
    limits.max_script_bytes = script_bytes;
    assert_eq!(
        world.query_with_limits(0, kind, &limits),
        Ok(default.clone())
    );
    limits.max_script_bytes -= 1;
    assert!(matches!(
        world.query_with_limits(0, kind, &limits),
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::ScriptBytes,
            limit,
            ..
        })) if limit == script_bytes - 1
    ));
}

/// FR-315-AC-44: monitor atom and state caps are supplied by the same request.
#[trace("TC-1250", "FR-315-AC-44")]
#[test]
fn monitor_encoding_limits_accept_exact_and_reject_zero_or_one_short() {
    let world = counter(
        INFINITE,
        &["always (holds(c.value = 3) implies always holds(c.value = 3))"],
    );
    let kind = SmtQueryKind::Unrolling { depth: 1 };
    let default = world.query(0, kind).expect("default monitor limits encode");
    let mut limits = SmtEncodingLimits {
        max_monitor_atoms: 2,
        ..SmtEncodingLimits::default()
    };
    assert_eq!(
        world.query_with_limits(0, kind, &limits),
        Ok(default.clone())
    );
    for max_monitor_atoms in [1, 0] {
        limits.max_monitor_atoms = max_monitor_atoms;
        assert!(matches!(
            world.query_with_limits(0, kind, &limits),
            Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
                measure: Measure::MonitorAtoms,
                value: 2,
                limit,
                setting: "smt_encoding.max_monitor_atoms",
            })) if limit == max_monitor_atoms
        ));
    }

    let exact_states = 3;
    limits.max_monitor_atoms = 2;
    limits.max_monitor_states = exact_states;
    assert_eq!(
        world.query_with_limits(0, kind, &limits),
        Ok(default.clone())
    );
    limits.max_monitor_states = exact_states - 1;
    assert!(matches!(
        world.query_with_limits(0, kind, &limits),
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::MonitorStates,
            limit,
            setting: "smt_encoding.max_monitor_states",
            ..
        })) if limit == exact_states - 1
    ));
}

/// `Counter`: `value: Int[0, 3]`, operation `inc` modifying it, guarded by
/// `value < 3`.
const COUNTER_CONTRACT: &str = "pre CanInc using v on M::Counter::inc { self.value < 3 }\n\
     post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

fn counter(profile: &str, formulas: &[&str]) -> World {
    counter_with(COUNTER_CONTRACT, profile, formulas)
}

fn counter_with(contracts: &str, profile: &str, formulas: &[&str]) -> World {
    let document = document(
        "Counter",
        vec![field("Counter", "value", &ident("Level"), "required")],
        vec![operation("Counter", "inc", &["value"])],
    );
    world(
        ("Counter", "c"),
        &document,
        contracts,
        profile,
        formulas,
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

// ---------------------------------------------------------------------------
// A solver-free reading of a script: satisfiability by search over small
// domains, so a test can tell an unsat query from a sat one.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

fn parse(text: &str) -> Vec<Sexp> {
    let spaced = text.replace('(', " ( ").replace(')', " ) ");
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    for token in spaced.split_whitespace() {
        match token {
            "(" => stack.push(Vec::new()),
            ")" => {
                let done = stack.pop().expect("balanced");
                stack.last_mut().expect("balanced").push(Sexp::List(done));
            }
            atom => stack
                .last_mut()
                .expect("balanced")
                .push(Sexp::Atom(atom.to_owned())),
        }
    }
    stack.pop().expect("balanced")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Val {
    Bool(bool),
    Int(i64),
}

fn constants(term: &Sexp, names: &BTreeMap<String, usize>, found: &mut Vec<usize>) {
    match term {
        Sexp::Atom(name) => {
            if let Some(index) = names.get(name) {
                found.push(*index);
            }
        }
        Sexp::List(items) => items.iter().for_each(|item| constants(item, names, found)),
    }
}

fn eval(term: &Sexp, names: &BTreeMap<String, usize>, env: &[Val]) -> Val {
    let int = |value: Val| match value {
        Val::Int(value) => value,
        Val::Bool(_) => panic!("an integer was expected"),
    };
    let boolean = |value: Val| match value {
        Val::Bool(value) => value,
        Val::Int(_) => panic!("a Boolean was expected"),
    };
    match term {
        Sexp::Atom(atom) => match atom.as_str() {
            "true" => Val::Bool(true),
            "false" => Val::Bool(false),
            _ => match names.get(atom) {
                Some(index) => env[*index],
                None => Val::Int(atom.parse().expect("a numeral")),
            },
        },
        Sexp::List(items) => {
            let Some(Sexp::Atom(head)) = items.first() else {
                panic!("an application")
            };
            let operands: Vec<Val> = items[1..]
                .iter()
                .map(|item| eval(item, names, env))
                .collect();
            match (head.as_str(), operands.as_slice()) {
                ("not", [only]) => Val::Bool(!boolean(*only)),
                ("and", all) => Val::Bool(all.iter().all(|value| boolean(*value))),
                ("or", all) => Val::Bool(all.iter().any(|value| boolean(*value))),
                ("=>", [left, right]) => Val::Bool(!boolean(*left) || boolean(*right)),
                ("=", [left, right]) => Val::Bool(left == right),
                ("<=", [left, right]) => Val::Bool(int(*left) <= int(*right)),
                ("<", [left, right]) => Val::Bool(int(*left) < int(*right)),
                (">=", [left, right]) => Val::Bool(int(*left) >= int(*right)),
                (">", [left, right]) => Val::Bool(int(*left) > int(*right)),
                ("+", all) => Val::Int(all.iter().map(|value| int(*value)).sum()),
                ("-", [only]) => Val::Int(-int(*only)),
                ("-", [left, right]) => Val::Int(int(*left) - int(*right)),
                ("*", [left, right]) => Val::Int(int(*left) * int(*right)),
                ("mod", [left, right]) => Val::Int(int(*left).rem_euclid(int(*right))),
                ("ite", [test, then, other]) => {
                    if boolean(*test) {
                        *then
                    } else {
                        *other
                    }
                }
                other => panic!("an unknown operator {other:?}"),
            }
        }
    }
}

/// The values an Integer constant ranges over in the search below. Every
/// ranged constant of the fixtures lies inside it (`Int[0, 3]`), so an
/// unsatisfiable answer is exact for a script whose Integers are all ranged
/// ([`assert_unsat`] checks that); a satisfiable answer is a witness for any.
const SEARCH_VALUES: std::ops::Range<i64> = 0..8;

/// Whether some assignment of the declared constants satisfies the script,
/// Integers ranging over [`SEARCH_VALUES`].
fn satisfiable(script: &str) -> bool {
    let mut names = BTreeMap::new();
    let mut kinds = Vec::new();
    let mut assertions = Vec::new();
    for command in parse(script) {
        let Sexp::List(items) = command else { continue };
        match items.as_slice() {
            [Sexp::Atom(head), Sexp::Atom(name), Sexp::Atom(sort)] if head == "declare-const" => {
                names.insert(name.clone(), kinds.len());
                kinds.push(sort == "Bool");
            }
            [Sexp::Atom(head), term] if head == "assert" => assertions.push(term.clone()),
            _ => {}
        }
    }
    // Check each assertion as soon as its last constant is assigned.
    let mut by_last: Vec<Vec<&Sexp>> = vec![Vec::new(); kinds.len() + 1];
    for assertion in &assertions {
        let mut found = Vec::new();
        constants(assertion, &names, &mut found);
        by_last[found.into_iter().max().map_or(0, |last| last + 1)].push(assertion);
    }
    fn search(
        at: usize,
        kinds: &[bool],
        names: &BTreeMap<String, usize>,
        by_last: &[Vec<&Sexp>],
        env: &mut Vec<Val>,
    ) -> bool {
        if !by_last[at]
            .iter()
            .all(|term| eval(term, names, env) == Val::Bool(true))
        {
            return false;
        }
        if at == kinds.len() {
            return true;
        }
        let domain: Vec<Val> = if kinds[at] {
            vec![Val::Bool(false), Val::Bool(true)]
        } else {
            SEARCH_VALUES.map(Val::Int).collect()
        };
        for value in domain {
            env.push(value);
            let found = search(at + 1, kinds, names, by_last, env);
            env.pop();
            if found {
                return true;
            }
        }
        false
    }
    search(0, &kinds, &names, &by_last, &mut Vec::new())
}

fn sat(world: &World, clause: usize, kind: SmtQueryKind) -> bool {
    satisfiable(&world.query(clause, kind).expect("encodes"))
}

/// The script is unsatisfiable, and the search is exact for it: every
/// Integer constant it declares carries a range inside the searched values.
fn assert_unsat(script: &str) {
    let max = SEARCH_VALUES.end - 1;
    for line in script.lines() {
        if let Some(rest) = line.strip_prefix("(declare-const ") {
            if let Some(name) = rest.strip_suffix(" Int)") {
                let ranged = script.lines().any(|candidate| {
                    candidate == format!("(assert (and (<= 0 {name}) (<= {name} {max})))")
                        || (0..=max).any(|upper| {
                            candidate == format!("(assert (and (<= 0 {name}) (<= {name} {upper})))")
                        })
                });
                assert!(ranged, "`{name}` is searched over 0..{max} but not ranged");
            }
        }
    }
    assert!(!satisfiable(script));
}

/// Regression for SR-12356 FND-001: a state whose guard holds is no reason
/// to cut a path. `value <= 2` is violated at step 3 of the chain, so the
/// unrolling and the base case are satisfiable at depth 4 too, and the
/// stutter keeps the path alive. (The step case at depth 4 needs five pairwise
/// distinct states of a four-valued field and is unsatisfiable on its own.)
#[trace("TC-900")]
#[test]
fn a_violation_at_step_three_is_found_at_depth_four() {
    let world = counter(INFINITE, &["always holds(c.value <= 2)"]);
    for kind in [
        SmtQueryKind::Unrolling { depth: 3 },
        SmtQueryKind::Unrolling { depth: 4 },
        SmtQueryKind::InductionBase { depth: 4 },
        SmtQueryKind::InductionStep { depth: 3 },
    ] {
        assert!(sat(&world, 0, kind), "{kind:?}");
    }
    assert!(!sat(
        &counter(INFINITE, &["always holds(c.value <= 3)"]),
        0,
        SmtQueryKind::Unrolling { depth: 4 }
    ));
}

/// Regression for SR-12356 FND-002: a finite trace that ends before `depth`
/// reads every atom false past its end, so a violation found at depth 3 is
/// found at depth 5, and the horizon past the last state of the chain is a
/// violation of `always[0,4]`.
#[trace("TC-900")]
#[test]
fn a_finite_trace_may_end_before_the_depth() {
    let world = counter(
        BOUNDED,
        &[
            "always[0,5] holds(c.value <= 2)",
            "always[0,4] holds(c.value <= 3)",
        ],
    );
    assert!(sat(&world, 0, SmtQueryKind::Unrolling { depth: 3 }));
    assert!(sat(&world, 0, SmtQueryKind::Unrolling { depth: 5 }));
    assert!(sat(&world, 1, SmtQueryKind::Unrolling { depth: 4 }));
}

/// A bounded `always` whose atom holds at every position of a trace that is
/// as long as its horizon has no violation when the subject cannot end early:
/// an operation without a postcondition that is always enabled.
#[trace("TC-900")]
#[test]
fn a_finite_trace_cannot_end_where_an_operation_is_certainly_enabled() {
    let document = document(
        "Counter",
        vec![field("Counter", "value", &ident("Level"), "required")],
        vec![operation("Counter", "poke", &["value"])],
    );
    let world = world(
        ("Counter", "c"),
        &document,
        "",
        BOUNDED,
        &["always[0,2] holds(c.value <= 3)"],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    assert!(!sat(&world, 0, SmtQueryKind::Unrolling { depth: 2 }));
}

fn slot(formulas: &[&str], held: Json) -> World {
    let document = document(
        "Slot",
        vec![field("Slot", "held", &ident("Level"), "optional")],
        vec![
            operation("Slot", "drop", &["held"]),
            operation("Slot", "take", &["held"]),
        ],
    );
    world(
        ("Slot", "s"),
        &document,
        "post Dropped using v on M::Slot::drop { not present(self.held) }\n\
         pre Has using v on M::Slot::take { present(self.held) implies value(self.held) <= 3 }\n\
         post Took using v on M::Slot::take { present(self.held) and value(self.held) = 1 }\n",
        INFINITE,
        formulas,
        &[("s", json!({ "held": held }))],
    )
}

fn has(script: &str, line: &str) {
    assert!(
        script.lines().any(|candidate| candidate == line),
        "missing `{line}` in\n{script}"
    );
}

fn declared(script: &str) -> Vec<&str> {
    script
        .lines()
        .filter_map(|line| line.strip_prefix("(declare-const "))
        .collect()
}

/// FR-315-AC-1: the bounded `Counter` item declares the four copies in
/// order, each followed by its end flag, asserts the initial predicate, one
/// transition per step and the negated property; equal inputs give byte-equal
/// scripts. A finite trace may end, so each step also has the end disjuncts.
#[trace("TC-900", "FR-315-AC-1")]
#[test]
fn an_unrolling_declares_the_copies_and_asserts_the_negated_property() {
    let world = counter(BOUNDED, &["always[0,3] holds(c.value <= 3)"]);
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 3 })
        .expect("encodes");
    assert_eq!(
        declared(&script),
        [
            "s0.c.value Int)",
            "h0 Bool)",
            "s1.c.value Int)",
            "h1 Bool)",
            "s2.c.value Int)",
            "h2 Bool)",
            "s3.c.value Int)",
            "h3 Bool)"
        ]
    );
    assert!(script.starts_with("(set-logic QF_LIA)\n"));
    assert!(script.ends_with("(check-sat)\n"));
    has(&script, "(assert (= s0.c.value 0))");
    has(&script, "(assert (not h0))");
    has(
        &script,
        "(assert (and (= h1 (or h0 (not (and (< s0.c.value 3) (or (= (- 0 s0.c.value) 1) (= (- 1 s0.c.value) 1) (= (- 2 s0.c.value) 1) (= (- 3 s0.c.value) 1)))))) (or h1 (and (< s0.c.value 3) (= (- s1.c.value s0.c.value) 1)))))",
    );
    has(
        &script,
        "(assert (or h0 (not (<= s0.c.value 3)) h1 (not (<= s1.c.value 3)) h2 (not (<= s2.c.value 3)) h3 (not (<= s3.c.value 3))))",
    );
    let again = world
        .query(0, SmtQueryKind::Unrolling { depth: 3 })
        .expect("encodes");
    assert_eq!(script, again);
}

/// FR-315-AC-2: the step case asserts no initial predicate, the property at
/// position 0 and its violation at position 1; the base case asserts the
/// initial predicate on copy 0.
#[trace("TC-900", "FR-315-AC-2")]
#[test]
fn the_step_case_has_no_initial_state_and_the_base_case_has_one() {
    let world = counter(INFINITE, &["always holds(c.value <= 3)"]);
    let step = world
        .query(0, SmtQueryKind::InductionStep { depth: 1 })
        .expect("encodes");
    assert!(!step.contains("(= s0.c.value 0)"), "{step}");
    has(&step, "(assert (not (not (<= s0.c.value 3))))");
    has(&step, "(assert (not (<= s1.c.value 3)))");
    let base = world
        .query(0, SmtQueryKind::InductionBase { depth: 1 })
        .expect("encodes");
    has(&base, "(assert (= s0.c.value 0))");
    // Under infinite-trace the terminal stutter is one more disjunct.
    has(
        &base,
        "(assert (or (and (< s0.c.value 3) (= (- s1.c.value s0.c.value) 1)) (= s1.c.value s0.c.value)))",
    );
}

/// FR-315-AC-3: a field whose sort has no SMT form refuses, naming the
/// object and the field, and writes no script.
#[trace("TC-900", "FR-315-AC-3")]
#[test]
fn a_field_with_no_smt_sort_is_refused_by_name() {
    let document = document(
        "Counter",
        vec![
            field("Counter", "value", &ident("Level"), "required"),
            field("Counter", "label", "ix://quire/native/String", "required"),
        ],
        vec![operation("Counter", "inc", &["value"])],
    );
    let world = world(
        ("Counter", "c"),
        &document,
        COUNTER_CONTRACT,
        INFINITE,
        &["always holds(c.value <= 3)"],
        &[(
            "c",
            json!({"value": {"integer": "0"}, "label": {"kind": "string", "text": "a"}}),
        )],
    );
    assert_eq!(
        world.query(0, SmtQueryKind::Unrolling { depth: 1 }),
        Err(SmtEncodingRefusal::Refused(Refusal::FieldSort {
            object: "c".to_owned(),
            field: "label".to_owned(),
            sort: "a String",
        }))
    );
}

/// FR-315-AC-4 (the declarations and the presence reading): an optional field
/// is a value constant and a `Bool` presence constant, the frame and the
/// initial predicate read presence, and every term is linear. S3 refuses a
/// `value` read it cannot prove present (`Unproved(Presence)`), so the
/// spec's `always holds(value(s.held) <= 3)` does not check; the guarded
/// claim stands in for it.
#[trace("TC-900")]
#[test]
fn an_optional_field_is_a_value_and_a_presence_constant() {
    let world = slot(
        &["always holds(present(s.held) implies value(s.held) <= 3)"],
        json!({"present": {"integer": "0"}}),
    );
    let script = world
        .query_with(&["s"], 0, SmtQueryKind::Unrolling { depth: 2 })
        .expect("encodes");
    assert_eq!(
        declared(&script),
        [
            "s0.s.held Int)",
            "s0.s.held.present Bool)",
            "s1.s.held Int)",
            "s1.s.held.present Bool)",
            "s2.s.held Int)",
            "s2.s.held.present Bool)"
        ]
    );
    has(&script, "(assert (and s0.s.held.present (= s0.s.held 0)))");
    has(&script, "(assert (and (<= 0 s1.s.held) (<= s1.s.held 3)))");
    assert!(!script.contains("(* "), "{script}");
}

/// FR-315-AC-5: the step case asserts the copies pairwise distinct; a TP-3
/// item declares a monitor state per position and asserts its steps, the
/// property at a position being the monitor not rejecting there.
#[trace("TC-900", "FR-315-AC-5")]
#[test]
fn the_step_case_asserts_distinct_copies_and_the_monitor_steps() {
    let world = counter(
        INFINITE,
        &[
            "always holds(c.value <= 3)",
            "always[0,1] holds(c.value <= 3)",
        ],
    );
    let plain = world
        .query(0, SmtQueryKind::InductionStep { depth: 2 })
        .expect("encodes");
    for assertion in [
        "(assert (not (= s1.c.value s0.c.value)))",
        "(assert (not (= s2.c.value s0.c.value)))",
        "(assert (not (= s2.c.value s1.c.value)))",
    ] {
        has(&plain, assertion);
    }
    let monitored = world
        .query(1, SmtQueryKind::InductionStep { depth: 2 })
        .expect("encodes");
    assert_eq!(
        declared(&monitored)
            .into_iter()
            .filter(|line| line.starts_with('m'))
            .collect::<Vec<_>>(),
        ["m0 Int)", "m1 Int)", "m2 Int)"]
    );
    assert!(monitored.contains("(=> (and (= m0 "), "{monitored}");
    has(
        &monitored,
        "(assert (not (and (= s1.c.value s0.c.value) (= m1 m0))))",
    );
    // The property at a position is the monitor not rejecting there.
    has(&monitored, "(assert (not (= m0 1)))");
    has(&monitored, "(assert (= m2 1))");
}

/// A TP-3 base case steps the initial monitor state over the letter at
/// position 0.
#[trace("TC-900", "FR-315-AC-5")]
#[test]
fn the_base_case_steps_the_initial_monitor_over_position_zero() {
    let world = counter(INFINITE, &["always[0,1] holds(c.value <= 3)"]);
    let base = world
        .query(0, SmtQueryKind::InductionBase { depth: 1 })
        .expect("encodes");
    has(&base, "(assert (=> (not (<= s0.c.value 3)) (= m0 1)))");
    has(&base, "(assert (=> (<= s0.c.value 3) (= m0 2)))");
}

/// An expression outside `QF_LIA` refuses, naming the construct and the
/// clause it is in.
#[trace("TC-900")]
#[test]
fn a_nonlinear_atom_is_refused_with_its_locus() {
    let world = counter(INFINITE, &["always holds(c.value * c.value <= 3)"]);
    match world.query(0, SmtQueryKind::Unrolling { depth: 1 }) {
        Err(SmtEncodingRefusal::Refused(Refusal::OutOfFragment { form, locus })) => {
            assert_eq!(form, "nonlinear multiplication");
            assert_eq!(locus.clause, "C0");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Items and kinds the proof queries do not cover refuse without a script.
#[trace("TC-900")]
#[test]
fn items_without_a_proof_query_are_refused() {
    let world = counter(
        BOUNDED,
        &[
            "always[0,3] holds(c.value <= 3)",
            "eventually[0,3] holds(c.value = 3)",
        ],
    );
    assert!(matches!(
        world.query(0, SmtQueryKind::InductionStep { depth: 1 }),
        Err(SmtEncodingRefusal::Refused(Refusal::QueryKind {
            kind: "induction-step",
            ..
        }))
    ));
    let live = counter(INFINITE, &["eventually holds(c.value = 3)"]);
    assert!(matches!(
        live.query(0, SmtQueryKind::Unrolling { depth: 1 }),
        Err(SmtEncodingRefusal::Refused(Refusal::ItemForm { .. }))
    ));
}

/// A bounded `eventually` is violated where its atom is false at every
/// position of the horizon.
#[trace("TC-900")]
#[test]
fn a_bounded_eventually_is_violated_where_every_position_is_false() {
    let world = counter(BOUNDED, &["eventually[0,2] holds(c.value = 3)"]);
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 2 })
        .expect("encodes");
    has(
        &script,
        "(assert (and (or h0 (not (= s0.c.value 3))) (or h1 (not (= s1.c.value 3))) (or h2 (not (= s2.c.value 3)))))",
    );
}

/// Queries too large to write refuse with the measure, value and limit.
#[trace("TC-900")]
#[test]
fn a_query_too_large_to_write_is_refused_with_its_measure() {
    let deep = counter(INFINITE, &["always holds(c.value <= 3)"]);
    match deep.query(0, SmtQueryKind::Unrolling { depth: u64::MAX }) {
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::Constants,
            ..
        })) => {}
        other => panic!("expected too-large, got {other:?}"),
    }
}

/// DL-7: the deadlock-freedom item is violated at a terminal state no
/// declaration intends, and holds where the terminal state is intended.
#[trace("TC-900")]
#[test]
fn a_deadlock_is_a_terminal_state_no_declaration_intends() {
    let undeclared = counter_with(COUNTER_CONTRACT, INFINITE, &[]);
    let sat_at = |world: &World, depth| {
        satisfiable(
            &world
                .deadlock(SmtQueryKind::Unrolling { depth })
                .expect("encodes"),
        )
    };
    // The chain reaches value 3, where `inc` is disabled, at position 3.
    assert!(!sat_at(&undeclared, 2));
    assert!(sat_at(&undeclared, 3));
    let any = counter_with(
        &format!("{COUNTER_CONTRACT}terminal Halt using v on M any;\n"),
        INFINITE,
        &[],
    );
    assert!(!sat_at(&any, 4));
    let at_three = counter_with(
        &format!("{COUNTER_CONTRACT}terminal Done using v on M when {{ self.value = 3 }};\n"),
        INFINITE,
        &[],
    );
    assert!(!sat_at(&at_three, 4));
    let at_two = counter_with(
        &format!("{COUNTER_CONTRACT}terminal Done using v on M when {{ self.value = 2 }};\n"),
        INFINITE,
        &[],
    );
    assert!(sat_at(&at_two, 4));
}

// ---------------------------------------------------------------------------
// The `Tally` subject of FR-315-AC-4 and the scripts read as text.
// ---------------------------------------------------------------------------

const TALLY_CONTRACT: &str =
    "pre G using v on M::Tally::bump { self.tally < 3 and self.next = self.tally + 1 }\n\
     post B using v on M::Tally::bump { self.tally - pre(self.tally) = 1 }\n\
     post C using v on M::Tally::bump { self.tally < 3 and self.next = self.tally + 1 }\n";

fn tally(contracts: &str, profile: &str, formulas: &[&str]) -> World {
    let document = document(
        "Tally",
        vec![
            field("Tally", "tally", &ident("Level"), "required"),
            field("Tally", "next", &ident("Level"), "required"),
        ],
        vec![operation("Tally", "bump", &["tally", "next"])],
    );
    world(
        ("Tally", "t"),
        &document,
        &format!("{TALLY_CONTRACT}{contracts}"),
        profile,
        formulas,
        &[(
            "t",
            json!({"tally": {"integer": "0"}, "next": {"integer": "1"}}),
        )],
    )
}

fn show(term: &Sexp) -> String {
    match term {
        Sexp::Atom(atom) => atom.clone(),
        Sexp::List(items) => format!("({})", items.iter().map(show).collect::<Vec<_>>().join(" ")),
    }
}

/// The assertion lines of a script, in order, as terms.
fn assertions(script: &str) -> Vec<Sexp> {
    script
        .lines()
        .filter(|line| line.starts_with("(assert "))
        .map(|line| match parse(line).remove(0) {
            Sexp::List(mut items) => items.remove(1),
            atom => atom,
        })
        .collect()
}

/// The operands of `term` when it is an application of `head`, else the term.
fn operands(term: &Sexp, head: &str) -> Vec<String> {
    match term {
        Sexp::List(items) if matches!(items.first(), Some(Sexp::Atom(h)) if h == head) => {
            items[1..].iter().map(show).collect()
        }
        other => vec![show(other)],
    }
}

/// FR-315-AC-15: the infinite-trace step is `bump`'s disjunct and then the
/// proof-query stutter, which holds the field equalities and nothing else.
#[trace("TC-900", "FR-315-AC-15")]
#[test]
fn the_proof_query_stutter_holds_only_field_equalities() {
    let world = tally("", INFINITE, &["always holds(t.tally < 2)"]);
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 3 })
        .expect("encodes");
    let steps: Vec<Vec<String>> = assertions(&script)
        .iter()
        .filter(|term| {
            show(term).contains("s1.t.next s0.t.next")
                || show(term).contains("s2.t.next s1.t.next")
                || show(term).contains("s3.t.next s2.t.next")
        })
        .map(|term| operands(term, "or"))
        .collect();
    assert_eq!(steps.len(), 3);
    for (index, disjuncts) in steps.iter().enumerate() {
        assert_eq!(disjuncts.len(), 2, "{disjuncts:?}");
        let (to, from) = (index + 1, index);
        assert_eq!(
            disjuncts[1],
            format!("(and (= s{to}.t.next s{from}.t.next) (= s{to}.t.tally s{from}.t.tally))")
        );
    }
}

/// FR-315-AC-16, read by a search over the script: the stutter carries a
/// path to a violation where `bump` is disabled, and the induction queries of
/// a true invariant close; `Tally`'s deadlock-freedom item.
#[trace("TC-900", "FR-315-AC-16")]
#[test]
fn the_tally_queries_are_satisfiable_where_the_stutter_reaches_a_violation() {
    let world = tally(
        "",
        INFINITE,
        &["always holds(t.tally < 2)", "always holds(t.tally < 3)"],
    );
    assert!(sat(&world, 0, SmtQueryKind::Unrolling { depth: 3 }));
    assert!(sat(&world, 0, SmtQueryKind::InductionBase { depth: 3 }));
    for kind in [
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionStep { depth: 1 },
    ] {
        assert_unsat(&world.query(1, kind).expect("encodes"));
    }
    let undeclared = |depth| {
        world
            .deadlock(SmtQueryKind::InductionBase { depth })
            .expect("encodes")
    };
    assert!(satisfiable(&undeclared(2)));
    assert_unsat(&undeclared(1));
    let intended = tally(
        "terminal Done using v on M when { self.tally = 2 };\n",
        INFINITE,
        &[],
    );
    assert_unsat(
        &intended
            .deadlock(SmtQueryKind::InductionBase { depth: 3 })
            .expect("encodes"),
    );
}

/// FR-315-AC-4: the range coercion of `t.tally + 1` carries its definedness
/// condition under the `and`'s short-circuit, and `G`'s and `C`'s coercions
/// are `bump`'s precondition and postcondition failures at every position.
#[trace("TC-900", "FR-315-AC-4")]
#[test]
fn a_partial_construct_contributes_its_definedness_failure_at_every_position() {
    let world = tally(
        "",
        INFINITE,
        &["always holds(t.tally < 3 and t.next = t.tally + 1)"],
    );
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 2 })
        .expect("encodes");
    let negated = operands(assertions(&script).last().expect("negated property"), "or");
    for position in 0..=2 {
        let range =
            format!("(and (<= 0 (+ s{position}.t.tally 1)) (<= (+ s{position}.t.tally 1) 3))");
        // The atom's failure, equal to `G`'s precondition failure and printed
        // once: the guard holds and the sum lies outside the range.
        let outside = format!("(not (=> (< s{position}.t.tally 3) {range}))");
        assert!(
            negated.contains(&outside),
            "position {position}: {negated:?}"
        );
        // `C`'s postcondition failure over the 16 candidates of (tally, next).
        let post = negated
            .iter()
            .filter(|disjunct| disjunct.contains(&format!("(= (- 0 s{position}.t.tally) 1)")))
            .collect::<Vec<_>>();
        assert_eq!(post.len(), 1, "position {position}");
        assert_eq!(post[0].matches("(= (- ").count(), 16, "{}", post[0]);
    }
    // `B` holds no partial construct, and every term is linear.
    assert!(!script.contains("(* "));
}

/// FR-315-AC-19: at position 1 of the step case the negated property holds
/// `bump`'s precondition and postcondition failures over copy 1, and the
/// position-0 assertion negates the same failures over copy 0.
#[trace("TC-900", "FR-315-AC-19")]
#[test]
fn the_step_case_negates_the_failures_at_the_positions_before_the_last() {
    let world = tally("", INFINITE, &["always holds(t.tally < 3)"]);
    let script = world
        .query(0, SmtQueryKind::InductionStep { depth: 1 })
        .expect("encodes");
    let all = assertions(&script);
    let negated = operands(&all[all.len() - 1], "or");
    let before = operands(&all[all.len() - 2], "and");
    assert_eq!(negated.len(), 3, "{negated:?}");
    for disjunct in &negated {
        let at_zero = format!("(not {})", disjunct.replace("s1.", "s0."));
        assert!(before.contains(&at_zero), "{at_zero} in {before:?}");
    }
    for failure in &negated[1..] {
        assert!(
            failure.contains("s1.t") && !failure.contains("s0."),
            "{failure}"
        );
    }
}

/// FR-315-AC-20: the deadlock-freedom item declares no `m<i>` or `h<i>`,
/// keeps the stutter, and reads a violation as a terminal copy; with
/// `terminal when P` the violation conjoins the negation of `P`.
#[trace("TC-900", "FR-315-AC-20")]
#[test]
fn deadlock_freedom_reads_terminal_by_the_candidate_expansion() {
    let plain = tally("", INFINITE, &[]);
    let script = plain
        .deadlock(SmtQueryKind::InductionBase { depth: 2 })
        .expect("encodes");
    assert!(!script.contains(" m0 ") && !script.contains("(declare-const h"));
    assert!(script.contains("(= s1.t.next s0.t.next)"));
    let negated = operands(assertions(&script).last().expect("negated"), "or");
    // 16 candidates of (tally, next), `B` read with each tally literal.
    assert!(
        negated[0].matches("(= (- 0 s0.t.tally) 1)").count() == 4,
        "{}",
        negated[0]
    );
    let declared = tally(
        "terminal Done using v on M when { self.tally = 2 };\n",
        INFINITE,
        &[],
    );
    let script = declared
        .deadlock(SmtQueryKind::InductionBase { depth: 2 })
        .expect("encodes");
    let negated = operands(assertions(&script).last().expect("negated"), "or");
    assert!(
        negated[0].ends_with("(not (= s0.t.tally 2)))"),
        "{}",
        negated[0]
    );
}

/// FR-315-AC-17: under false-extension the step is `(and (= h<i+1> (or h<i>
/// T)) (or h<i+1> D))` with no stutter, the violation includes `h<p>`, and
/// the queries that are no V-2 query refuse `query-kind`.
#[trace("TC-900", "FR-315-AC-17")]
#[test]
fn the_false_extension_step_is_exactly_the_end_flag_equation() {
    let world = counter(
        BOUNDED,
        &[
            "always[0,5] holds(c.value <= 3)",
            "always[0,3] holds(c.value <= 3)",
            "eventually[0,3] holds(c.value = 3)",
        ],
    );
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 5 })
        .expect("encodes");
    has(&script, "(assert (not h0))");
    let enabled = "(and (< s0.c.value 3) (or (= (- 0 s0.c.value) 1) (= (- 1 s0.c.value) 1) (= (- 2 s0.c.value) 1) (= (- 3 s0.c.value) 1)))";
    has(
        &script,
        &format!(
            "(assert (and (= h1 (or h0 (not {enabled}))) (or h1 (and (< s0.c.value 3) (= (- s1.c.value s0.c.value) 1)))))"
        ),
    );
    let negated = operands(assertions(&script).last().expect("negated"), "or");
    for position in 0..=5 {
        assert!(negated.contains(&format!("h{position}")), "{negated:?}");
    }
    assert!(matches!(
        world.query(1, SmtQueryKind::InductionBase { depth: 3 }),
        Err(SmtEncodingRefusal::Refused(Refusal::QueryKind { .. }))
    ));
    assert!(matches!(
        world.query(2, SmtQueryKind::Unrolling { depth: 2 }),
        Err(SmtEncodingRefusal::Refused(Refusal::QueryKind { .. }))
    ));
}

/// FR-315-AC-18, read by a search over the script.
#[trace("TC-900", "FR-315-AC-18")]
#[test]
fn the_bounded_queries_are_decided_as_the_finite_behaviours_decide_them() {
    let world = counter(
        BOUNDED,
        &[
            "always[0,3] holds(c.value <= 3)",
            "always[0,5] holds(c.value <= 3)",
            "always[0,5] holds(c.value <= 2)",
            "eventually[0,3] holds(c.value = 3)",
            "eventually[0,2] holds(c.value = 3)",
        ],
    );
    assert_unsat(
        &world
            .query(0, SmtQueryKind::Unrolling { depth: 3 })
            .expect("encodes"),
    );
    assert!(sat(&world, 1, SmtQueryKind::Unrolling { depth: 5 }));
    assert!(sat(&world, 2, SmtQueryKind::Unrolling { depth: 5 }));
    assert_unsat(
        &world
            .query(3, SmtQueryKind::Unrolling { depth: 3 })
            .expect("encodes"),
    );
    assert!(sat(&world, 4, SmtQueryKind::Unrolling { depth: 2 }));
}

/// FND-011: a field the frame grants but that has no finite domain is
/// written, not frozen. `inc` raises `id` by one, so `id <= 0` is violated
/// after one step; the frame leaves `id` free.
#[trace("TC-900")]
#[test]
fn a_granted_field_without_a_finite_domain_is_not_frozen() {
    let integer = "ix://quire/native/Integer".to_owned();
    let package = document(
        "Counter",
        vec![
            field("Counter", "id", &integer, "required"),
            field("Counter", "value", &ident("Level"), "required"),
        ],
        vec![operation("Counter", "inc", &["id", "value"])],
    );
    let granted = world(
        ("Counter", "c"),
        &package,
        "pre CanInc using v on M::Counter::inc { self.value < 3 }\n\
         post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n\
         post Id using v on M::Counter::inc { self.id - pre(self.id) = 1 }\n",
        INFINITE,
        &["always holds(c.id <= 0)"],
        &[(
            "c",
            json!({"id": {"integer": "0"}, "value": {"integer": "0"}}),
        )],
    );
    for kind in [
        SmtQueryKind::Unrolling { depth: 1 },
        SmtQueryKind::Unrolling { depth: 2 },
        SmtQueryKind::InductionBase { depth: 1 },
        SmtQueryKind::InductionBase { depth: 2 },
    ] {
        assert!(sat(&granted, 0, kind), "{kind:?}");
    }
    // The false-extension query expands `inc`'s candidates and refuses
    // `field-domain` at `c.id` (FR-315-AC-24).
    let bounded = {
        let package = document(
            "Counter",
            vec![
                field("Counter", "id", &integer, "required"),
                field("Counter", "value", &ident("Level"), "required"),
            ],
            vec![operation("Counter", "inc", &["id", "value"])],
        );
        world(
            ("Counter", "c"),
            &package,
            "post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n",
            BOUNDED,
            &["always[0,2] holds(c.value <= 3)"],
            &[(
                "c",
                json!({"id": {"integer": "0"}, "value": {"integer": "0"}}),
            )],
        )
    };
    match bounded.query(0, SmtQueryKind::Unrolling { depth: 2 }) {
        Err(SmtEncodingRefusal::Refused(Refusal::FieldDomain { object, field })) => {
            assert_eq!((object.as_str(), field.as_str()), ("c", "id"));
        }
        other => panic!("expected field-domain, got {other:?}"),
    }
}

/// FR-315-AC-24: a result is a literal in each postcondition read, and the
/// script declares no constant for it.
#[trace("TC-900", "FR-315-AC-24")]
#[test]
fn an_operation_result_is_read_as_each_of_its_values() {
    let boolean = "ix://quire/native/Boolean".to_owned();
    let document = document(
        "Counter",
        vec![field("Counter", "value", &ident("Level"), "required")],
        vec![
            operation("Counter", "inc", &["value"]),
            operation_with("Counter", "isZero", &[], Some(boolean), &[], false),
        ],
    );
    let world = world(
        ("Counter", "c"),
        &document,
        "pre CanInc using v on M::Counter::inc { self.value < 3 }\n\
         post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n\
         post Z using v on M::Counter::isZero { result = (self.value = 0) }\n",
        INFINITE,
        &["always holds(c.value <= 3)"],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 1 })
        .expect("encodes");
    assert_eq!(declared(&script), ["s0.c.value Int)", "s1.c.value Int)"]);
    let step = &assertions(&script)
        .iter()
        .find(|term| show(term).contains("false"))
        .map(|term| operands(term, "or"))
        .expect("a step");
    assert!(
        step.contains(
            &"(and (or (= false (= s1.c.value 0)) (= true (= s1.c.value 0))) (= s1.c.value s0.c.value))"
                .to_owned()
        ),
        "{step:?}"
    );
}

/// FR-315-AC-25: an optional field's equality is presence-equal and, where
/// present, value-equal; the stutter and the distinctness assertion print it.
#[trace("TC-900", "FR-315-AC-25")]
#[test]
fn optional_field_equality_ignores_the_value_of_an_absent_field() {
    let world = slot(
        &["always holds(present(s.held) or not present(s.held))"],
        json!({"absent": {}}),
    );
    let script = world
        .query(0, SmtQueryKind::InductionStep { depth: 1 })
        .expect("encodes");
    let equality = "(and (= s1.s.held.present s0.s.held.present) (or (not s1.s.held.present) (= s1.s.held s0.s.held)))";
    assert!(script.contains(equality), "{script}");
    has(&script, &format!("(assert (not {equality}))"));
}

/// FR-315-AC-22: declaration, assertion and disjunct order over two objects.
#[trace("TC-900", "FR-315-AC-22")]
#[test]
fn the_canonical_order_covers_two_objects_and_parameters() {
    let document = document(
        "Counter",
        vec![field("Counter", "value", &ident("Level"), "required")],
        vec![
            operation("Counter", "inc", &["value"]),
            operation_with(
                "Counter",
                "setTo",
                &[("n", ident("Level"))],
                None,
                &["value"],
                false,
            ),
        ],
    );
    let world = world(
        ("Counter", "x"),
        &document,
        "post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n\
         post Assign using v on M::Counter::setTo { self.value = n }\n",
        INFINITE,
        &["always holds(x.value <= 2)"],
        &[
            ("d", json!({"value": {"integer": "0"}})),
            ("c", json!({"value": {"integer": "0"}})),
        ],
    );
    let script = world
        .query_with(&["d", "c"], 0, SmtQueryKind::InductionStep { depth: 2 })
        .expect("encodes");
    assert_eq!(
        declared(&script),
        [
            "s0.c.value Int)",
            "s0.d.value Int)",
            "s1.c.value Int)",
            "s1.d.value Int)",
            "s2.c.value Int)",
            "s2.d.value Int)"
        ]
    );
    let all = assertions(&script);
    // Six ranges, two steps, three distinctness, two positions, the negation.
    assert_eq!(all.len(), 6 + 2 + 3 + 2 + 1, "{script}");
    let step = operands(&all[6], "or");
    // c.inc, c.setTo(0..3), d.inc, d.setTo(0..3), the stutter.
    assert_eq!(step.len(), 1 + 4 + 1 + 4 + 1);
    let negated = operands(&all[all.len() - 1], "or");
    assert_eq!(negated[0], "(not (<= s2.d.value 2))");
    assert_eq!(negated[1], "(not (<= s2.c.value 2))");
    assert!(show(&all[8]).starts_with("(not (and "));
}

/// FR-315-AC-23: the monitor's range, its implications and its start.
#[trace("TC-900", "FR-315-AC-23")]
#[test]
fn the_monitor_implications_follow_the_step_assertions() {
    let world = counter(
        INFINITE,
        &["always (holds(c.value = 3) implies always holds(c.value = 3))"],
    );
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 2 })
        .expect("encodes");
    let all = assertions(&script);
    let implications: Vec<&Sexp> = all
        .iter()
        .filter(|term| matches!(term, Sexp::List(items) if matches!(items.first(), Some(Sexp::Atom(head)) if head == "=>")))
        .collect();
    let states: usize = (0..9)
        .find(|n| script.contains(&format!("(<= m0 {n})")))
        .map(|last| last + 1)
        .expect("a range for m0");
    // Position 0 reads each letter once; positions 1 and 2 each state and letter.
    let letters = implications
        .iter()
        .filter(|term| matches!(term, Sexp::List(items) if show(&items[2]).starts_with("(= m0 ")))
        .count();
    assert_eq!(implications.len(), letters * (1 + 2 * states));
    let last_step = all
        .iter()
        .rposition(|term| show(term).contains("s2.c.value s1.c.value"))
        .expect("a step");
    let first_implication = all
        .iter()
        .position(|term| matches!(term, Sexp::List(items) if matches!(items.first(), Some(Sexp::Atom(head)) if head == "=>")))
        .expect("an implication");
    assert!(first_implication > last_step);
    let step_case = world
        .query(0, SmtQueryKind::InductionStep { depth: 2 })
        .expect("encodes");
    assert!(
        !assertions(&step_case)
            .iter()
            .any(|term| matches!(term, Sexp::List(items) if show(&items[0]) == "=>" && show(&items[2]).starts_with("(= m0 "))),
        "m0 is constrained by its range alone"
    );
}

/// FR-315-AC-21: the refusals, with their codes.
#[trace("TC-900", "FR-315-AC-21")]
#[test]
fn each_refusal_has_its_code() {
    let code = |result: Result<String, SmtEncodingRefusal>| match result {
        Err(refusal) => refusal.code().expect("a refusal, not a fault"),
        Ok(script) => panic!("expected a refusal, got\n{script}"),
    };
    let counter_world = counter(
        INFINITE,
        &[
            "always eventually holds(c.value = 0)",
            "always holds(c.value * c.value <= 9)",
            "always holds(c.value <= 3)",
        ],
    );
    let kind = SmtQueryKind::Unrolling { depth: 1 };
    assert_eq!(code(counter_world.query(0, kind)), "item-form");
    assert_eq!(code(counter_world.query(1, kind)), "out-of-fragment");
    match counter_world.query(2, SmtQueryKind::Unrolling { depth: 1 << 20 }) {
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure,
            value,
            limit,
            setting,
        })) => assert_eq!(
            (measure, value, limit, setting),
            (
                Measure::Constants,
                1_048_577,
                1_048_576,
                "smt_encoding.max_constants"
            )
        ),
        other => panic!("expected too-large, got {other:?}"),
    }
    let nested = counter(
        BOUNDED,
        &["always[0,3] eventually[0,1] holds(c.value >= 1)"],
    );
    assert_eq!(code(nested.query(0, kind)), "item-form");
    let subject = counter_world.without_initial_state(&["c"]);
    assert_eq!(
        encode_smt_query(
            &subject,
            SmtItem::Clause(&counter_world.clauses[2]),
            kind,
            &SmtEncodingLimits::default(),
        )
        .map_err(|refusal| refusal.code()),
        Err(Some("no-initial-state"))
    );
    let none = world(
        ("Counter", "c"),
        &document(
            "Counter",
            vec![field("Counter", "value", &ident("Level"), "required")],
            vec![operation("Counter", "inc", &["value"])],
        ),
        COUNTER_CONTRACT,
        INFINITE,
        &["always holds(c.value <= 3)"],
        &[],
    );
    assert_eq!(code(none.query_with(&[], 0, kind)), "no-instance");
    let spawning = world(
        ("Counter", "c"),
        &document(
            "Counter",
            vec![field("Counter", "value", &ident("Level"), "required")],
            vec![operation_with("Counter", "spawn", &[], None, &[], true)],
        ),
        "",
        INFINITE,
        &["always holds(c.value <= 3)"],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    match spawning.query(0, kind) {
        Err(SmtEncodingRefusal::Refused(Refusal::ObjectLifecycle(locus))) => {
            assert!(format!("{locus:?}").contains("spawn"), "{locus:?}");
        }
        other => panic!("expected object-lifecycle, got {other:?}"),
    }
    let unbounded = world(
        ("Counter", "c"),
        &document(
            "Counter",
            vec![field("Counter", "value", &ident("Level"), "required")],
            vec![operation_with(
                "Counter",
                "setTo",
                &[("n", "ix://quire/native/Integer".to_owned())],
                None,
                &["value"],
                false,
            )],
        ),
        "",
        INFINITE,
        &["always holds(c.value <= 3)"],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    match unbounded.query(0, kind) {
        Err(SmtEncodingRefusal::Refused(Refusal::ParameterDomain {
            operation,
            parameter,
        })) => assert_eq!((operation.as_str(), parameter.as_str()), ("setTo", "n")),
        other => panic!("expected parameter-domain, got {other:?}"),
    }
    let several = world(
        ("Counter", "c"),
        &document(
            "Counter",
            vec![field("Counter", "value", &ident("Level"), "required")],
            vec![operation("Counter", "inc", &["value"])],
        ),
        COUNTER_CONTRACT,
        INFINITE,
        &["always (holds(c.value = 3) implies always holds(c.value = 3))"],
        &[
            ("c", json!({"value": {"integer": "0"}})),
            ("d", json!({"value": {"integer": "0"}})),
        ],
    );
    assert_eq!(
        code(several.query_with(&["c", "d"], 0, kind)),
        "several-instances"
    );
}

/// FND-005: a step case whose pairwise distinctness alone passes the byte
/// limit refuses `script-bytes` without writing the pairs.
#[trace("TC-900")]
#[test]
fn a_step_case_over_the_byte_limit_refuses_before_it_builds_the_pairs() {
    let world = counter(INFINITE, &["always holds(c.value <= 3)"]);
    match world.query(
        0,
        SmtQueryKind::InductionStep {
            depth: (1 << 20) - 1,
        },
    ) {
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge { measure, limit, .. })) => {
            assert_eq!((measure, limit), (Measure::ScriptBytes, 1 << 28));
        }
        other => panic!(
            "expected too-large, got {:?}",
            other.map(|script| script.len())
        ),
    }
}

/// FR-315-AC-27: an operation's postcondition failure over its one candidate
/// reads each result in order, and its disjunct is the disjunction of them.
#[trace("TC-900", "FR-315-AC-27")]
#[test]
fn a_later_result_fails_only_where_the_earlier_ones_are_rejected() {
    let boolean = "ix://quire/native/Boolean".to_owned();
    let package = document(
        "Tally",
        vec![
            field("Tally", "tally", &ident("Level"), "required"),
            field("Tally", "next", &ident("Level"), "required"),
        ],
        vec![
            operation("Tally", "bump", &["tally", "next"]),
            operation_with("Tally", "peek", &[], Some(boolean), &[], false),
        ],
    );
    let world = world(
        ("Tally", "t"),
        &package,
        &format!(
            "{TALLY_CONTRACT}post P using v on M::Tally::peek {{ result = (self.tally < 3 and self.next = self.tally + 1) }}\n"
        ),
        INFINITE,
        &["always holds(t.tally < 3)"],
        &[("t", json!({"tally": {"integer": "0"}, "next": {"integer": "1"}}))],
    );
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 1 })
        .expect("encodes");
    let all = assertions(&script);
    let negated = operands(&all[all.len() - 1], "or");
    let failure = negated
        .iter()
        .find(|disjunct| disjunct.starts_with("(or ") && disjunct.contains("(and (not "))
        .unwrap_or_else(|| panic!("{negated:?}"));
    let parsed = parse(failure).remove(0);
    let Sexp::List(items) = &parsed else {
        panic!("{failure}")
    };
    assert_eq!(items.len(), 3, "(or U1 (and (not A1) U2)): {failure}");
    let second = show(&items[2]);
    assert!(second.starts_with("(and (not "), "{second}");
    // `peek`'s postcondition part is `(or A1 A2)`.
    let step = operands(
        all.iter()
            .find(|term| show(term).contains("s1.t.next s0.t.next"))
            .expect("a step"),
        "or",
    );
    assert_eq!(step.len(), 3, "bump, peek, stutter");
    assert!(step[1].starts_with("(and (or "), "{}", step[1]);
}

fn invariant_of(result: Result<String, SmtEncodingRefusal>) -> &'static str {
    match result {
        Err(SmtEncodingRefusal::Fault(fault)) => {
            assert_eq!(fault.stage(), "smt-encoding");
            fault.invariant()
        }
        other => panic!("expected a fault, got {other:?}"),
    }
}

/// FR-315-AC-29: a TP-1 or TP-3 item that binds false-extension is no
/// well-formed request; the call returns a fault, not a refusal code.
#[trace("TC-900", "FR-315-AC-29")]
#[test]
fn an_item_other_than_tp2_under_false_extension_is_a_fault() {
    let world = counter(
        INFINITE,
        &[
            "always holds(c.value <= 3)",
            "always (holds(c.value = 3) implies always holds(c.value = 3))",
        ],
    );
    let kind = SmtQueryKind::Unrolling { depth: 1 };
    for clause in &world.clauses {
        let altered = clause
            .clone()
            .with_profile_after_s3(TemporalProfile::EventPositionFalseExtension);
        let result = encode_smt_query(
            &world.subject(&["c"]),
            SmtItem::Clause(&altered),
            kind,
            &SmtEncodingLimits::default(),
        )
        .map(|script| String::from_utf8(script).expect("UTF-8"));
        assert_eq!(
            invariant_of(result),
            "a TP-1 or TP-3 item binds infinite-trace"
        );
    }
}

/// FR-315-AC-32: a TP-1 item whose atom is gone is a fault naming the atom
/// invariant, and none of the refusals of AC-21 is a fault.
#[trace("TC-900", "FR-315-AC-32")]
#[test]
fn a_tp1_item_without_its_atom_is_a_fault_not_item_form() {
    let world = counter(
        INFINITE,
        &["always (holds(c.value = 3) implies always holds(c.value = 3))"],
    );
    let altered = world.clauses[0]
        .clone()
        .with_property_form_after_s3(Some(PropertyForm::ReachableInvariant));
    let result = encode_smt_query(
        &world.subject(&["c"]),
        SmtItem::Clause(&altered),
        SmtQueryKind::Unrolling { depth: 1 },
        &SmtEncodingLimits::default(),
    )
    .map(|script| String::from_utf8(script).expect("UTF-8"));
    assert_eq!(invariant_of(result), "a TP-1 item holds its one atom");
}

/// FR-315-AC-31 (the violation order): the violation of `always[0,3]` is
/// `h<p>` and then the negation of the atom, as two disjuncts, at positions
/// 0 to 3 and at no position past 3. S3 proves no range for AC-31's
/// `pre(self.value) + 1` form (`Unproved(Range)`), so this fixture reads the
/// subtraction form, whose postcondition holds no partial construct and has
/// no failure term; the `(and (not h<p>) F)` shape is read in the tests of
/// AC-4 and AC-27 under infinite-trace only.
#[trace("TC-900", "FR-315-AC-31")]
#[test]
fn the_always_violation_is_two_disjuncts_per_position() {
    let world = counter(BOUNDED, &["always[0,3] holds(c.value <= 3)"]);
    let script = world
        .query(0, SmtQueryKind::Unrolling { depth: 5 })
        .expect("encodes");
    let negated = operands(assertions(&script).last().expect("negated"), "or");
    let mut expected = Vec::new();
    for position in 0..=3 {
        expected.push(format!("h{position}"));
        expected.push(format!("(not (<= s{position}.c.value 3))"));
    }
    assert_eq!(negated, expected);
}

/// FR-315-AC-45 and FR-315-AC-50: the scripts of TP-3 items, which read a
/// tabulated monitor, are byte-equal to the golden bytes recorded from `main`
/// before the table was built through the `CheckedFormula` constructor.
#[trace("TC-1264", "FR-315-AC-50")]
#[test]
fn monitored_scripts_are_byte_equal_to_the_recorded_golden_bytes() {
    let world = counter(
        INFINITE,
        &[
            "always (holds(c.value = 3) implies always holds(c.value = 3))",
            "always[0,1] holds(c.value <= 3)",
        ],
    );
    let kinds = [
        ("unrolling-2", SmtQueryKind::Unrolling { depth: 2 }),
        ("base-1", SmtQueryKind::InductionBase { depth: 1 }),
        ("step-2", SmtQueryKind::InductionStep { depth: 2 }),
    ];
    for (clause, name) in [(0, "implies-always"), (1, "bounded-always")] {
        for (label, kind) in kinds {
            let script = world.query(clause, kind).expect("encodes");
            let path = format!(
                "{}/tests/golden/smt_monitor/{name}-{label}.smt2",
                env!("CARGO_MANIFEST_DIR")
            );
            let golden = std::fs::read(&path).expect("the golden file");
            assert_eq!(script.as_bytes(), golden.as_slice(), "{name}-{label}");
        }
    }
}
