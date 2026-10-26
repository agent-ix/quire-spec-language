// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-475-AC-1: every checked state-graph form is emitted as its typed claim
//! node, with its claim occurrence and body references preserved.

use qsl_forms::{
    Expression, ExpressionForm, StateGraphClaimForm, StateGraphClaimKind, StateGraphPredicateForm,
    UsingAlias,
};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::Span;
use qsl_semantics::check::{PackageDeclarations, StateGraphDeclaration};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::Value;

use super::{source, whole_unit, CheckedPackage, GRAPH_V2};
use crate::emit::{emit_checked, emit_package, Emission};

fn predicate(value: bool) -> StateGraphPredicateForm {
    let span = Span { start: 0, end: 1 };
    StateGraphPredicateForm {
        operands: vec![ExpressionForm {
            expression: Expression::boolean(value),
            spans: qsl_forms::ExpressionSpans::new(span).expect("one Boolean span"),
        }],
        temporal_operators: Vec::new(),
        scopes: vec![Vec::new()],
    }
}

fn claim(index: usize, name: &str, kind: StateGraphClaimKind) -> StateGraphDeclaration {
    StateGraphDeclaration {
        index,
        form: StateGraphClaimForm {
            name: qsl_forms::DeclaredName {
                name: name.to_owned(),
                span: Span { start: 0, end: 1 },
            },
            profile: UsingAlias {
                alias: "v".to_owned(),
                span: Span { start: 0, end: 1 },
            },
            over: None,
            fairness: Vec::new(),
            kind,
            span: Span { start: 0, end: 1 },
        },
        selection: ProfileSelection {
            alias: "v".to_owned(),
            identity: "quire.value.complete/v1".to_owned(),
            span: Span { start: 0, end: 1 },
            identity_span: Span { start: 0, end: 1 },
        },
        context: None,
        context_population: None,
    }
}

fn state_graph_package() -> CheckedPackage {
    let declarations = PackageDeclarations {
        state_graph_claims: vec![
            claim(
                0,
                "PossibleClaim",
                StateGraphClaimKind::Possible {
                    target: predicate(true),
                },
            ),
            claim(
                1,
                "AlwaysClaim",
                StateGraphClaimKind::AlwaysPossible {
                    from: None,
                    target: predicate(false),
                },
            ),
            claim(
                2,
                "UniqueClaim",
                StateGraphClaimKind::UniquePath {
                    from: predicate(true),
                    to: predicate(false),
                },
            ),
        ],
        ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
    };
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the state-graph forms check and lower"),
    )
}

/// FR-165-AC-1's parsed state-graph forms over the admitted ConfigVersion model.
fn parsed_state_graph_package() -> (CheckedPackage, Vec<u8>) {
    let document = super::admission_corpus::config_version_unit::DOMAIN_PACKAGE
        .as_bytes()
        .to_vec();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one ConfigVersion package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let unit = format!(
        "{}\n\
         possible ReachesTwo using v over (c: Config::ConfigVersion) {{ c.versionNumber = 2 }}\n\
         always possible CanStillWinWithoutFrom using v over (c: Config::ConfigVersion) \
         {{ c.versionNumber = 2 }}\n\
         always possible CanStillWin using v over (c: Config::ConfigVersion) \
         from (c.versionNumber != 1000) {{ c.versionNumber = 2 }}\n\
         unique path InOneWay using v over (c: Config::ConfigVersion) \
         from (c.versionNumber = 0) to (c.versionNumber = 2);\n",
        super::admission_corpus::config_version_unit::unit_text(&digest),
    );
    let declarations = super::assemble_with_models(unit.as_bytes(), &packages)
        .expect("the state-graph claims assemble against ConfigVersion");
    let graph = declarations
        .check(CheckingLimits::default())
        .expect("the state-graph claims pass S3 and S4 checks");
    (CheckedPackage::link(graph), document)
}

fn wire(emission: &Emission) -> Value {
    serde_json::from_slice(emission.package.bytes()).expect("the emitted wire is JSON")
}

#[test]
#[ix_trace_rs::trace("FR-165-AC-1", "TC-590")]
fn ac1_emits_all_three_claim_forms_with_the_fr475_body_shape() {
    let package = state_graph_package();
    let emission = emit_package(&package, whole_unit).expect("state-graph claims emit");
    assert!(matches!(
        super::read_with(&emission, &super::read_evidence(&emission)),
        super::Read::Verified { .. }
    ));
    let wire = wire(&emission);
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the semantic graph has nodes");
    let claims: Vec<_> = nodes
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "state_graph")
        .collect();
    assert_eq!(claims.len(), 3);

    let mut emitted_kinds = std::collections::BTreeSet::new();
    for claim in &claims {
        let kind = claim["body"]["members"][0]["value"]["value"]
            .as_str()
            .expect("state-graph kind literal");
        emitted_kinds.insert(kind.to_owned());
        assert_eq!(claim["schema_version"], GRAPH_V2);
        assert_eq!(
            claim["semantic_type"]["digest"],
            nodes
                .iter()
                .find(|node| {
                    node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean"
                })
                .expect("the Boolean type is emitted")["node_id"]["digest"]
        );
        assert!(claim.get("declaration").is_none());
        assert!(claim.get("owner").is_none());
        assert_eq!(claim["occurrences"][0]["role"], "claim");
        let members = claim["body"]["members"]
            .as_array()
            .expect("the claim body is an aggregate");
        let names: Vec<_> = members
            .iter()
            .map(|member| member["name"].as_str().expect("a binding name"))
            .collect();
        assert_eq!(names, ["kind", "over", "from", "target"]);
        assert_eq!(members[0]["value"]["value"], kind);
        let target = members[3]["value"]["target"]["digest"]
            .as_str()
            .expect("target reference");
        assert!(nodes.iter().any(|node| {
            node["node_id"]["digest"] == target
                && node["semantic_type"]["digest"] == claim["semantic_type"]["digest"]
                && ((node["node_tag"] == "expression")
                    || (node["node_tag"] == "value" && node["semantic_form"] == "literal"))
        }));
        let dependencies = claim["dependencies"].as_array().expect("dependencies");
        assert_eq!(dependencies.len(), if kind == "possible" { 1 } else { 2 });
    }
    assert_eq!(
        emitted_kinds,
        ["possible", "always-possible", "unique-path"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );

    let possible = claims
        .iter()
        .find(|node| node["body"]["members"][0]["value"]["value"] == "possible")
        .expect("possible node");
    assert_eq!(
        possible["body"]["members"][1]["value"]["members"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        possible["body"]["members"][2]["value"]["members"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let always = claims
        .iter()
        .find(|node| node["body"]["members"][0]["value"]["value"] == "always-possible")
        .expect("always-possible node");
    let possible_target = possible["body"]["members"][3]["value"]["target"].clone();
    assert_eq!(
        always["body"]["members"][2]["value"]["target"],
        possible_target
    );
}

#[test]
#[ix_trace_rs::trace("FR-165-AC-1", "TC-590")]
fn ac1_source_claims_emit_their_over_binding_and_predicates() {
    let (package, document) = parsed_state_graph_package();
    let emission = emit_checked(&package).expect("source spans place the emitted claims");
    let mut evidence = super::read_evidence(&emission);
    let digest = qsl_semantics::model::key::hex(
        qsl_semantics::model::intake::package_input([document.as_slice()])
            .keys()
            .next()
            .expect("one domain package"),
    );
    evidence
        .insert_domain_package_document(digest, document)
        .expect("a fresh domain package evidence key");
    assert!(matches!(
        super::read_with(&emission, &evidence),
        super::Read::Verified { .. }
    ));

    let wire = wire(&emission);
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let claims: Vec<_> = nodes
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "state_graph")
        .collect();
    assert_eq!(claims.len(), 4);
    let possible = claims
        .iter()
        .find(|claim| claim["body"]["members"][0]["value"]["value"] == "possible")
        .expect("possible ReachesTwo");
    let always = claims
        .iter()
        .find(|claim| claim["body"]["members"][0]["value"]["value"] == "always-possible")
        .expect("always possible CanStillWin");
    assert_ne!(
        possible["node_id"], always["node_id"],
        "the claim form participates in its content address"
    );
    for claim in claims {
        let members = claim["body"]["members"].as_array().unwrap();
        let over = &members[1]["value"]["target"]["digest"];
        let parameter = nodes
            .iter()
            .find(|node| node["node_id"]["digest"] == *over)
            .expect("the over binding points to its parameter node");
        assert_eq!(parameter["node_tag"], "value");
        assert_eq!(parameter["semantic_form"], "parameter");
        assert_eq!(parameter["body"]["members"][0]["value"]["value"], "c");
        let kind = members[0]["value"]["value"].as_str().unwrap();
        if kind == "possible"
            || (kind == "always-possible"
                && members[2]["value"]["term"] == "aggregate"
                && members[2]["value"]["members"]
                    .as_array()
                    .unwrap()
                    .is_empty())
        {
            assert_eq!(members[2]["value"]["term"], "aggregate");
        } else {
            let target = &members[2]["value"]["target"]["digest"];
            assert_eq!(members[2]["value"]["term"], "reference");
            assert!(nodes.iter().any(|node| {
                node["node_id"]["digest"] == *target
                    && (node["node_tag"] == "expression"
                        || (node["node_tag"] == "value" && node["semantic_form"] == "literal"))
            }));
        }
        let predicate = &members[3];
        let target = &predicate["value"]["target"]["digest"];
        assert_eq!(predicate["value"]["term"], "reference");
        assert!(nodes.iter().any(|node| {
            node["node_id"]["digest"] == *target
                && (node["node_tag"] == "expression"
                    || (node["node_tag"] == "value" && node["semantic_form"] == "literal"))
        }));
        assert_eq!(claim["occurrences"][0]["role"], "claim");
    }
}
