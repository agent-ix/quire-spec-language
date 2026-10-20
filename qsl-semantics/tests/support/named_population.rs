// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1076's fixture (FR-391): the `Counter` unit of FR-155-AC-1, a
//! `test/counter` domain package with `Counter` (`v: Int[0, 2]`, `bump()`)
//! in `counters`, `Widget` (`grow()`) in `widgets`, the field-less `Watch` in
//! `watchers` and `Other` in `others`, and, when asked for, a second
//! population `archive` of `Counter`. A unit over it selects the package as
//! `M`; each test adds its own declarations.

#![allow(
    dead_code,
    reason = "each including test module uses a different part of the fixture"
)]

use std::collections::BTreeMap;

use qsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, meaning, package_input};
use qsl_semantics::model::key::{hex, DeclarationKey};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value as Json};

/// The package's identity.
pub const PACKAGE: &str = "test/counter";
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// `ix://test/counter/<suffix>`.
pub fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

/// The declaration key of `suffix` in the package.
pub fn key(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident(suffix),
    }
}

fn origin(identity: &str) -> Json {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn one() -> Json {
    json!({"lower": 1, "upper": 1, "ordered": false, "unique": true})
}

fn object_type(name: &str, fields: Vec<Json>, operations: Vec<Json>) -> Json {
    let identity = ident(name);
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(&identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": [], "fields": fields, "operations": operations, "relationships": [],
    })
}

fn operation(owner: &str, name: &str, modifies: &[&str]) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    let modifies: Vec<String> = modifies
        .iter()
        .map(|field| ident(&format!("{owner}/{field}")))
        .collect();
    json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [], "creates": [], "deletes": [],
            "modifies_receiver": modifies},
    })
}

fn level() -> Json {
    let identity = ident("Level");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&identity),
        })
    };
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "value_type"}, "roles": [],
        "origin": origin(&identity), "extensions": [], "unknownPolicy": "reject",
        "scalar": "integer",
        "constraints": [constraint("min", "0"), constraint("max", "2")],
    })
}

/// The domain package, with the second population `archive` of `Counter`
/// when `archive`.
pub fn document(archive: bool) -> Vec<u8> {
    let field = {
        let identity = ident("Counter/v");
        json!({
            "identity": identity, "name": "v", "typeRef": ident("Level"),
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": one(), "origin": origin(&identity),
        })
    };
    let types = vec![
        level(),
        object_type(
            "Counter",
            vec![field],
            vec![operation("Counter", "bump", &["v"])],
        ),
        object_type("Widget", vec![], vec![operation("Widget", "grow", &[])]),
        object_type("Watch", vec![], vec![]),
        object_type("Other", vec![], vec![]),
    ];
    let mut populations = vec![
        ("counters", "Counter"),
        ("widgets", "Widget"),
        ("watchers", "Watch"),
        ("others", "Other"),
    ];
    if archive {
        populations.push(("archive", "Counter"));
    }
    let populations: Vec<Json> = populations
        .iter()
        .map(|(name, member)| {
            let identity = ident(name);
            json!({
                "identity": identity, "displayName": identity,
                "kind": {"module": PACKAGE, "name": "population"},
                "members": [ident(member)], "extent": "closed", "origin": origin(&identity),
            })
        })
        .collect();
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
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle",
            "digest": PLACEHOLDER},
        "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": PLACEHOLDER,
            "mappingVersions": [], "profileVersions": [], "lockDigest": PLACEHOLDER},
        "occurrences": [], "extensions": [],
        "constructs": constructs, "types": types, "populations": populations,
    })
    .to_string()
    .into_bytes()
}

/// A refusal: its catalog code and cause, the text of the unit at its
/// region, and the check refusal when S3 made it.
#[derive(Debug)]
pub struct Refused {
    /// The catalog code.
    pub code: String,
    /// The catalog cause.
    pub cause: String,
    /// The text at its region.
    pub at: String,
    /// The refusal, when S3 made it.
    pub refusal: Option<CheckRefusal>,
}

/// The checked unit and the packages it was checked against.
pub struct Checked {
    /// The checked graph.
    pub graph: CheckedGraph,
    /// The domain package, by digest.
    pub packages: BTreeMap<[u8; 32], Vec<u8>>,
}

/// The unit over the package (with `archive` when asked) holding
/// `declarations`, checked, or its refusals each with its text.
pub fn check(archive: bool, declarations: &str) -> Result<Checked, Vec<Refused>> {
    let document = document(archive);
    let packages = package_input([document.as_slice()]);
    let digest = *packages.keys().next().expect("one model");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model M = \"{PACKAGE}\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {declarations}\n",
        hex(&digest),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1076", "fixture", "1"),
        "tc-1076.native",
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
    .expect("the model selection admits");
    let assembled =
        PackageDeclarations::assemble(parsed.source().reference().clone(), built, models, vec![]);
    let declarations = match assembled {
        Ok(declarations) => declarations,
        Err(refusal) => {
            return Err(refusal
                .errors
                .iter()
                .map(|error| Refused {
                    code: error.cause.code().as_str().to_owned(),
                    cause: error.cause.catalog_code().cause().to_owned(),
                    at: text[error.span.start..error.span.end].to_owned(),
                    refusal: None,
                })
                .collect())
        }
    };
    let regions = declarations.regions();
    match declarations.check(CheckingLimits::default()) {
        Ok(graph) => Ok(Checked { graph, packages }),
        Err(refusals) => Err(refusals
            .into_iter()
            .map(|refusal| {
                let at = regions
                    .refusal_region(&refusal)
                    .map_or_else(String::new, |region| {
                        let start = usize::try_from(region.start()).expect("a start");
                        let end = usize::try_from(region.end()).expect("an end");
                        text[start..end].to_owned()
                    });
                Refused {
                    code: refusal.cause.code().as_str().to_owned(),
                    cause: refusal.cause.cause().unwrap_or_default().to_owned(),
                    at,
                    refusal: Some(refusal),
                }
            })
            .collect()),
    }
}

/// [`check`], which must succeed.
pub fn checked(archive: bool, declarations: &str) -> Checked {
    check(archive, declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// The one refusal of [`check`]: its catalog code, its cause and its text.
pub fn refused(archive: bool, declarations: &str) -> (String, String, String) {
    let refusals = match check(archive, declarations) {
        Ok(_) => panic!("{declarations}: expected a refusal"),
        Err(refusals) => refusals,
    };
    let [refused] = &refusals[..] else {
        panic!("{declarations}: one refusal: {refusals:?}");
    };
    (
        refused.code.clone(),
        refused.cause.clone(),
        refused.at.clone(),
    )
}
