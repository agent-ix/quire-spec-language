// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 §8's ring buffer refined to a queue (FR-139, FR-140; TC-544,
//! TC-545): two domain packages and the `RingIsQueue` refinement over them,
//! assembled and checked with the rest of its unit.
//!
//! The abstract package `example/ring-queue-spec` holds `Queue { items,
//! used, busy, cells, lone, ghost, solid }` and `Cell { value }`; the
//! concrete package `example/ring-queue-impl` holds `Ring { head, size }` and
//! `Slot { ring, index, value }` with the populations `rings` and `slots`.
//! Every row of `RingIsQueue` reads `R::slots` whole.

#![allow(
    dead_code,
    reason = "each test binary uses the part of the fixture it needs"
)]

use std::collections::BTreeMap;

use qsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

/// The abstract package.
pub const QUEUE: &str = "example/ring-queue-spec";
/// The concrete package.
pub const RING: &str = "example/ring-queue-impl";

const INTEGER: &str = "ix://quire/native/Integer";
const BOOLEAN: &str = "ix://quire/native/Boolean";
const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// The identity text of `path` in `package`.
pub fn node(package: &str, path: &str) -> String {
    format!("ix://{package}/{path}")
}

/// The declaration key of `path` in `package`.
pub fn key(package: &str, path: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: node(package, path),
    }
}

fn origin(package: &str, path: &str) -> Value {
    let identity = node(package, path);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn construct(package: &str, name: &str, semantic_meaning: &str) -> Value {
    json!({
        "kind": {"module": package, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": DIGEST,
        "construct": {
            "identity": "none", "shape": "record", "members": {},
            "meaning": semantic_meaning,
        },
    })
}

fn declaration(package: &str, path: &str, kind: &str, members: Value) -> Value {
    let mut value = json!({
        "identity": node(package, path), "displayName": node(package, path),
        "kind": {"module": package, "name": kind},
        "roles": [], "origin": origin(package, path),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    value
        .as_object_mut()
        .expect("an object")
        .extend(members.as_object().expect("an object").clone());
    value
}

/// `Int[0, upper]` as a declared value type.
fn bounded(package: &str, name: &str, upper: u32) -> Value {
    let constraints: Vec<Value> = [("min", "0".to_owned()), ("max", upper.to_string())]
        .iter()
        .map(|(keyword, value)| {
            let path = format!("{name}/constraints/{keyword}");
            json!({
                "identity": node(package, &path), "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": INTEGER,
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": origin(package, &path),
            })
        })
        .collect();
    declaration(
        package,
        name,
        "value_type",
        json!({"scalar": "integer", "constraints": constraints}),
    )
}

/// A field with the multiplicity `lower..upper`: `(1, 1, false)` is a single
/// value, `(0, n, true)` a sequence and `(0, n, false)` a set.
fn field(
    package: &str,
    owner: &str,
    name: &str,
    type_ref: &str,
    (lower, upper, ordered): (u32, u32, bool),
) -> Value {
    let path = format!("{owner}/{name}");
    json!({
        "identity": node(package, &path), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": lower, "upper": upper, "ordered": ordered, "unique": !ordered},
        "origin": origin(package, &path),
    })
}

fn single(package: &str, owner: &str, name: &str, type_ref: &str) -> Value {
    field(package, owner, name, type_ref, (1, 1, false))
}

/// The operation `name` of `owner`, taking `parameters` (name, type) and
/// returning `returns`, whose frame modifies `modifies`.
fn operation(
    package: &str,
    owner: &str,
    name: &str,
    parameters: &[(&str, &str)],
    returns: Option<&str>,
    modifies: &[&str],
) -> Value {
    let path = format!("{owner}/{name}");
    let mut value = json!({
        "identity": node(package, &path), "name": name,
        "params": parameters
            .iter()
            .map(|(parameter_name, type_ref)| {
                let parameter_path = format!("{path}/{parameter_name}");
                json!({
                    "identity": node(package, &parameter_path), "name": parameter_name,
                    "typeRef": type_ref, "presence": "required", "nullable": false,
                    "defaultKind": "none",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    "origin": origin(package, &parameter_path),
                })
            })
            .collect::<Vec<_>>(),
        "pre": [], "post": [],
        "origin": {"source": {
            "sourceIdentity": node(package, "spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1,
        }},
        "frame": {
            "modifies": modifies
                .iter()
                .map(|field| node(package, &format!("{owner}/{field}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": [],
        },
    });
    if let Some(type_ref) = returns {
        value["returns"] = json!({
            "typeRef": type_ref,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        });
    }
    value
}

fn object_type(package: &str, name: &str, fields: Vec<Value>, operations: Vec<Value>) -> Value {
    declaration(
        package,
        name,
        "object_type",
        json!({
            "supertypes": [],
            "fields": fields,
            "operations": operations,
            "relationships": [],
        }),
    )
}

fn population(package: &str, name: &str, member: &str) -> Value {
    json!({
        "identity": node(package, name), "displayName": node(package, name),
        "kind": {"module": package, "name": "population"},
        "members": [node(package, member)], "extent": "closed",
        "origin": origin(package, name),
    })
}

fn document(package: &str, types: Vec<Value>, populations: Vec<Value>) -> Vec<u8> {
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": node(package, "spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": DIGEST,
        },
        "package": {
            "identity": package, "version": "1.0.0", "manifestDigest": DIGEST,
            "mappingVersions": [], "profileVersions": [], "lockDigest": DIGEST,
        },
        "occurrences": [], "extensions": [],
        "constructs": [
            construct(package, "object_type", meaning::OBJECT_TYPE),
            construct(package, "population", meaning::POPULATION),
            construct(package, "value_type", meaning::VALUE_TYPE),
        ],
        "types": types,
        "populations": populations,
    })
    .to_string()
    .into_bytes()
}

/// The abstract package: `Queue` with `enq(x)` and `deq()`, and `Cell`, over
/// `Digit = Int[0, 1]` and `Two = Int[0, 2]`.
pub fn queue_spec() -> Vec<u8> {
    let digit = node(QUEUE, "Digit");
    let two = node(QUEUE, "Two");
    document(
        QUEUE,
        vec![
            bounded(QUEUE, "Digit", 1),
            bounded(QUEUE, "Two", 2),
            object_type(
                QUEUE,
                "Queue",
                vec![
                    field(QUEUE, "Queue", "items", &digit, (0, 2, true)),
                    single(QUEUE, "Queue", "used", &two),
                    single(QUEUE, "Queue", "busy", BOOLEAN),
                    field(QUEUE, "Queue", "cells", &node(QUEUE, "Cell"), (0, 2, false)),
                    single(QUEUE, "Queue", "lone", &digit),
                    single(QUEUE, "Queue", "ghost", &digit),
                    single(QUEUE, "Queue", "solid", BOOLEAN),
                ],
                vec![
                    operation(QUEUE, "Queue", "enq", &[("x", &digit)], None, &["items"]),
                    operation(QUEUE, "Queue", "deq", &[], Some(&digit), &["items"]),
                ],
            ),
            object_type(
                QUEUE,
                "Cell",
                vec![single(QUEUE, "Cell", "value", &digit)],
                Vec::new(),
            ),
        ],
        vec![
            population(QUEUE, "queues", "Queue"),
            population(QUEUE, "cells", "Cell"),
        ],
    )
}

/// The concrete package: `Ring { head, size }` and `Slot { ring, index,
/// value }` with `put(x)` and `take()`, the populations `rings` and `slots`.
pub fn ring_buffer() -> Vec<u8> {
    let digit = node(RING, "Digit");
    document(
        RING,
        vec![
            bounded(RING, "Digit", 1),
            bounded(RING, "Two", 2),
            object_type(
                RING,
                "Ring",
                vec![
                    single(RING, "Ring", "head", &digit),
                    single(RING, "Ring", "size", &node(RING, "Two")),
                ],
                Vec::new(),
            ),
            object_type(
                RING,
                "Slot",
                vec![
                    single(RING, "Slot", "ring", &node(RING, "Ring")),
                    single(RING, "Slot", "index", &digit),
                    single(RING, "Slot", "value", &digit),
                ],
                vec![
                    operation(RING, "Slot", "put", &[("x", &digit)], None, &["value"]),
                    operation(RING, "Slot", "take", &[], Some(&digit), &["value"]),
                ],
            ),
        ],
        vec![
            population(RING, "rings", "Ring"),
            population(RING, "slots", "Slot"),
        ],
    )
}

/// The `items` row of ADR-020 §8. `s.index + 0` is `s.index` as an
/// `Integer`, which is what an equality with the `mod` result types.
pub const ITEMS: &str = "items = seq(i in 0 .. self.size - 1: \
     only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";

/// The `Queue` object row's field rows: the `items` row of ADR-020 §8 and
/// one row for each other form that reads the slots whole.
pub fn all_rows() -> String {
    format!(
        "{ITEMS}\n\
         used = count(s in R::slots where s.ring = self);\n\
         busy = exists(s in R::slots where s.value = 1);\n\
         cells = R::slots;\n\
         lone = only(s in R::slots where s.ring = self: s.value);\n\
         ghost = only(s in R::slots where s.ring = self and s.index + 0 = self.head + 2: s.value);\n\
         solid = all(s in R::slots where s.value = 1);"
    )
}

/// `RingIsQueue` with the `Queue` object row's field rows `rows`, after a
/// state clause under the complete value profile, which is what admits `mod`
/// in the rows (the profile is the unit's, not the row's).
pub fn ring_is_queue(rows: &str) -> String {
    format!(
        "invariant Anything using v on R::Ring at current {{ true }}\n\
         refinement RingIsQueue using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         population Q::cells from R::slots;\n\
         object Q::Queue from R::Ring {{ {rows} }}\n\
         object Q::Cell from R::Slot {{ value = self.value; }}\n\
         step R::Slot::put -> Q::Queue::enq(self.ring, x);\n\
         step R::Slot::take -> Q::Queue::deq(self.ring);\n\
         }}"
    )
}

/// A refusal with the text of the unit it is located at.
pub struct Refused {
    /// The refusal.
    pub refusal: CheckRefusal,
    /// The text of the unit at its region.
    pub at: String,
}

/// A checked unit and what built it.
pub struct Checked {
    /// The checked graph.
    pub graph: CheckedGraph,
    /// The domain packages, by digest.
    pub packages: BTreeMap<[u8; 32], Vec<u8>>,
}

/// The unit holding `body` over both models, checked, or its refusals.
pub fn check(body: &str) -> Result<Checked, Vec<Refused>> {
    let (queue, ring) = (queue_spec(), ring_buffer());
    let mut packages = BTreeMap::new();
    let mut selections = String::new();
    for (alias, identity, bytes) in [("Q", QUEUE, &queue), ("R", RING, &ring)] {
        let single = package_input([bytes.as_slice()]);
        let [(digest, _)] = single.iter().collect::<Vec<_>>()[..] else {
            panic!("one supplied document");
        };
        selections.push_str(&format!(
            "model {alias} = {identity:?} version \"1.0.0\" digest \"sha256-jcs:{}\";\n",
            hex(digest)
        ));
        packages.extend(single);
    }
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile inf = \"quire.temporal.infinite-trace/v1\";\n\
         {selections}{body}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-544", "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), [], "the unit is admissible: {text}");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"));
    let declarations =
        PackageDeclarations::assemble(parsed.source().reference().clone(), built, selected, vec![])
            .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"))
            .with_unwritten_refinements();
    let regions = declarations.regions();
    match declarations.check(CheckingLimits::default()) {
        Ok(graph) => Ok(Checked { graph, packages }),
        Err(refusals) => Err(refusals
            .into_iter()
            .map(|refusal| {
                let region = regions
                    .refusal_region(&refusal)
                    .unwrap_or_else(|| panic!("{refusal:?} names a region"));
                let (start, end) = (
                    usize::try_from(region.start()).expect("a start"),
                    usize::try_from(region.end()).expect("an end"),
                );
                Refused {
                    at: text[start..end].to_owned(),
                    refusal,
                }
            })
            .collect()),
    }
}

/// [`check`], which must succeed.
pub fn checked(body: &str) -> Checked {
    check(body).unwrap_or_else(|refusals| {
        panic!(
            "{body}: {:?}",
            refusals
                .iter()
                .map(|refused| (&refused.refusal, &refused.at))
                .collect::<Vec<_>>()
        )
    })
}

/// [`check`], which must be refused.
pub fn refused(body: &str) -> Vec<Refused> {
    match check(body) {
        Ok(_) => panic!("{body}: expected a refusal"),
        Err(refusals) => refusals,
    }
}
