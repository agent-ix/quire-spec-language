// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185: a package whose random parameters, rewards or workloads check is
//! refused at emission, since FR-322's v2 vocabulary has no node for them,
//! rather than written without them.

use super::admission_corpus::config_version_unit;
use super::*;

/// The ConfigVersion unit with a workload and a reward on `attemptUpdate`.
fn probabilistic_package() -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = format!(
        "{}workload Steady on Config {{ weight ConfigVersion::attemptUpdate = 1; }}\n\
         reward cost on ConfigVersion::attemptUpdate = 1;\n",
        config_version_unit::unit_text()
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    assert!(!graph.probabilistic_model().is_empty());
    CheckedPackage::link(graph)
}

/// FR-185 "Package emission": the checked workload and reward are recorded
/// on the graph, and the emitter refuses
/// `unsupported_construct`/`not-yet-implemented`, naming the first of them in
/// source order (the workload `Steady`), its feature and the unit's profile.
#[trace("TC-620")]
#[test]
fn a_checked_probabilistic_model_refuses_emission() {
    let refusal = emit_checked(&probabilistic_package()).expect_err("emission refuses");
    assert_eq!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration: "Steady".to_owned(),
            feature: qsl_semantics::check::ProbabilisticFeature::Workload,
            profile: Some("quire.model.complete/v1"),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}

/// The ConfigVersion unit with the `ev` profile and one `under every
/// scheduler` claim per `(name, bound)`; such a claim names no workload, so
/// no declaration the emitter still refuses (FR-185 "Package emission").
fn claims_package(claims: &[(&str, &str)]) -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let mut unit = config_version_unit::unit_text().replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile ev = \"quire.temporal.event-position.false-extension/v1\";\n",
    );
    for (name, bound) in claims {
        unit.push_str(&format!(
            "probabilistic {name} using ev on Config over (c: Config::ConfigVersion) \
             under every scheduler on origin {{ probability >= {bound} [ holds(true) ] }}\n"
        ));
    }
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    assert_eq!(graph.probabilistic_model().claims().len(), claims.len());
    CheckedPackage::link(graph)
}

/// QSL's full I2 read of `emission`, with the lock's required features and
/// the ConfigVersion domain package as evidence.
fn read_with_domain(emission: &Emission) -> Read {
    let mut evidence = read_evidence(emission);
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    evidence
        .insert_domain_package_document(package_identity(document), document.to_vec())
        .expect("a fresh evidence key");
    read_with(emission, &evidence)
}

/// The emitted `probabilistic_claims` rows.
fn rows(emission: &Emission) -> Vec<Value> {
    wire(emission)["checked_term_data"]["probabilistic_claims"]
        .as_array()
        .expect("a package with claims carries checked_term_data rows")
        .clone()
}

/// The emitted wire node `declaration` names.
fn wire_node(emission: &Emission, declaration: &Value) -> Value {
    wire(emission)["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["node_id"] == *declaration)
        .cloned()
        .expect("the row's declaration is a local node")
}

/// FR-498-AC-1 (TC-1288 steps 1, 4; QSpec FR-322-AC-297 PC-01, PC-02): two
/// probabilistic claims emit two rows with distinct declaration `NodeRef`s,
/// each naming one declared `claim` occurrence of a `claim` node whose
/// `semantic_form` is `probabilistic_claim`, in ascending structural order,
/// and the strict reader admits the package. Permuting the source order
/// emits equal rows. Two claims that share a name but assert different
/// things keep two identities.
#[trace("FR-498-AC-1", "FR-498-AC-4", "TC-1288")]
#[test]
fn each_probabilistic_claim_emits_one_row_with_its_own_declaration() {
    let forward = emit_checked(&claims_package(&[("Early", "0.5"), ("Late", "0.9")]))
        .expect("claims are emitted");
    let read = read_with_domain(&forward);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let rows_forward = rows(&forward);
    assert_eq!(rows_forward.len(), 2);
    assert_ne!(
        rows_forward[0]["declaration"],
        rows_forward[1]["declaration"]
    );
    for row in &rows_forward {
        assert_eq!(row["occurrence"]["role"], "claim");
        let node = wire_node(&forward, &row["declaration"]);
        assert_eq!(node["node_tag"], "claim");
        assert_eq!(node["semantic_form"], "probabilistic_claim");
        assert!(node["occurrences"]
            .as_array()
            .unwrap()
            .contains(&row["occurrence"]));
    }
    let ascending = |rows: &[Value]| {
        let ids = rows
            .iter()
            .map(|row| node_id_of(&row["declaration"]))
            .collect::<Vec<_>>();
        ids.windows(2).all(|pair| pair[0] < pair[1])
    };
    assert!(ascending(&rows_forward), "{rows_forward:?}");

    let permuted = emit_checked(&claims_package(&[("Late", "0.9"), ("Early", "0.5")]))
        .expect("claims are emitted");
    assert_eq!(rows(&permuted), rows_forward);

    let same_name = emit_checked(&claims_package(&[("Same", "0.5"), ("Same", "0.9")]))
        .expect("claims are emitted");
    assert!(matches!(
        read_with_domain(&same_name),
        Read::Verified { .. }
    ));
    let same = rows(&same_name);
    assert_eq!(same.len(), 2);
    assert_ne!(same[0]["declaration"], same[1]["declaration"]);
}

/// The ordering key of a wire `NodeRef`.
fn node_id_of(declaration: &Value) -> CheckedNodeId {
    serde_json::from_value(declaration.clone()).expect("a typed node id")
}

/// FR-498-AC-2 (TC-1288 step 2): each `CheckedProbabilisticClaim` exposes the
/// `{declaration, occurrence}` its row carries, and claims with equal names
/// do not share an identity.
#[trace("FR-498-AC-2", "TC-1288")]
#[test]
fn a_checked_claim_exposes_the_identity_of_its_row() {
    for names in [["Early", "Late"], ["Same", "Same"]] {
        let package = claims_package(&[(names[0], "0.5"), (names[1], "0.9")]);
        let emission = emit_checked(&package).expect("claims are emitted");
        let rows = rows(&emission);
        let claims = package.graph().probabilistic_model().claims();
        let identities = claims
            .iter()
            .map(|claim| claim.identity().expect("a lowered claim has an identity"))
            .collect::<Vec<_>>();
        assert_ne!(identities[0], identities[1], "{names:?}");
        for identity in identities {
            let declaration = serde_json::to_value(node_id(identity.node())).unwrap();
            let row = rows
                .iter()
                .find(|row| row["declaration"] == declaration)
                .unwrap_or_else(|| panic!("{names:?}: no row for {declaration}"));
            assert_eq!(row["occurrence"]["ordinal"], identity.occurrence());
        }
    }
}

/// FR-498-AC-4 (TC-1288 step 4): a package has exactly one row per
/// probabilistic claim node and none without a node, and the rows leave the
/// semantic graph's other nodes and their identities as they are without
/// claims.
#[trace("FR-498-AC-4", "TC-1288")]
#[test]
fn rows_match_claim_nodes_and_leave_other_node_identities_alone() {
    let without = emit_checked(&claims_package(&[])).expect("the unit emits");
    assert_eq!(
        wire(&without)["checked_term_data"]["probabilistic_claims"],
        json!([])
    );
    let with = emit_checked(&claims_package(&[("Early", "0.5"), ("Late", "0.9")]))
        .expect("claims are emitted");
    let nodes_of = |emission: &Emission| {
        wire(emission)["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| (node["node_id"].to_string(), node.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let (before, after) = (nodes_of(&without), nodes_of(&with));
    // A node whose id or content names a pending-handle path segment is
    // numbered by creation order, which the claims' own nodes shift; every
    // other node keeps its identity and content.
    let stable = before
        .iter()
        .filter(|(_, node)| !node.to_string().contains("{\"node\":"))
        .collect::<Vec<_>>();
    assert!(stable.len() >= 5, "{} stable nodes", stable.len());
    for (id, node) in stable {
        assert_eq!(after.get(id), Some(node), "node {id} is unchanged");
    }
    let claim_nodes = after
        .iter()
        .filter(|(id, _)| !before.contains_key(*id))
        .filter(|(_, node)| node["semantic_form"] == "probabilistic_claim")
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let row_nodes = rows(&with)
        .iter()
        .map(|row| row["declaration"].to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(claim_nodes.len(), 2);
    assert_eq!(claim_nodes, row_nodes);
}
