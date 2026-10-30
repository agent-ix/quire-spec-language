// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-24/25: cyclic protocol references use the checked graph's
//! recursion-group encoding and read back through I2.

use super::*;

const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";

fn package_text(declarations: &str) -> String {
    package_text_with_named_function("refers", "protocol Flow", declarations, true)
}

fn package_text_with_named_function(
    function_name: &str,
    function_body: &str,
    declarations: &str,
    function_first: bool,
) -> String {
    let fixture = spine_model_without(&[FIELD_ACCESS, CONFORMING_EQUALITY]);
    let fixture = fixture.replace(
        "profile v = \"quire.model.complete/v1\";\n",
        &format!("profile v = \"quire.model.complete/v1\";\nprofile p = \"{PROTOCOL_PROFILE}\";\n"),
    );
    let function = format!(
        "function {function_name} using v(left: Boolean, right: Boolean): Boolean pure {{ {function_body} }}\n"
    );
    if function_first {
        format!("{fixture}{function}{declarations}")
    } else {
        format!("{fixture}{declarations}{function}")
    }
}

fn protocol_declarations(cyclic: bool) -> String {
    protocol_declarations_with_predicate(cyclic, "refers")
}

fn protocol_declarations_with_predicate(cyclic: bool, predicate: &str) -> String {
    let join = if cyclic {
        format!("join predicate({predicate}) [west, east];")
    } else {
        "join all [west, east];".to_owned()
    };
    let flow = "protocol Flow using v over (input: M::Gadget) on origin {\n\
        role R on M::Gadget;\n\
        run parallel Main {\n\
          branch west sequence West { }\n\
          branch east sequence East { }\n\
        } $JOIN\n\
        finish End as (outcome: Boolean) { true };\n\
        }\n";
    format!(
        "{}protocol Idle using v over (input: M::Gadget) on origin {{\n\
         role R on M::Gadget;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n",
        flow.replace("$JOIN", &join)
    )
}

fn checked(text: &str) -> (qsl_semantics::check::CheckedGraph, String) {
    checked_with_limits(text, CheckingLimits::default())
}

fn checked_with_limits(
    text: &str,
    limits: CheckingLimits,
) -> (qsl_semantics::check::CheckedGraph, String) {
    let (graph, digest) =
        check_result(text, limits).unwrap_or_else(|refusals| panic!("unit checks: {refusals:?}"));
    (graph, digest)
}

fn check_result(
    text: &str,
    limits: CheckingLimits,
) -> Result<(qsl_semantics::check::CheckedGraph, String), Vec<qsl_semantics::check::CheckRefusal>> {
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied model document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let declarations = assemble_with_models(text.as_bytes(), &packages).expect("unit assembles");
    declarations.check(limits).map(|graph| (graph, digest))
}

fn declaration_is_idle(node: &qsl_semantics::check::SemanticNode) -> bool {
    node.declaration()
        .is_some_and(|segments| segments.len() == 1 && segments[0].as_str() == "Idle")
}

#[trace("FR-383-AC-24", "FR-383-AC-25")]
#[test]
fn a_cyclic_predicate_join_protocol_is_grouped_and_reads_back() {
    let cyclic_text = package_text(&protocol_declarations(true));
    let (graph, model_digest) = checked(&cyclic_text);
    let cycle_nodes: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.recursion().is_some())
        .collect();
    assert!(
        !cycle_nodes.is_empty(),
        "the checked graph has a cycle group"
    );
    let groups: BTreeSet<_> = cycle_nodes
        .iter()
        .map(|node| *node.recursion().unwrap().group())
        .collect();
    assert_eq!(groups.len(), 1, "one shared cyclic component");
    assert!(
        cycle_nodes
            .iter()
            .any(|node| node.semantic_form() == "protocol_clause"),
        "the protocol clause belongs to the component"
    );
    assert!(
        cycle_nodes
            .iter()
            .any(|node| node.semantic_form() == "protocol_reference"),
        "the protocol reference carrier belongs to the component"
    );
    let group_size = cycle_nodes.len();
    let ordinals: BTreeSet<_> = cycle_nodes
        .iter()
        .map(|node| {
            let recursion = node.recursion().unwrap();
            assert_eq!(recursion.size(), group_size);
            recursion.ordinal()
        })
        .collect();
    assert_eq!(ordinals, (0..group_size).collect());
    let idle_cyclic = graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "protocol_clause" && declaration_is_idle(node))
        .expect("acyclic protocol clause in the mixed unit")
        .key();
    let (acyclic_graph, _) = checked(&package_text(&protocol_declarations(false)));
    let idle_acyclic = acyclic_graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "protocol_clause" && declaration_is_idle(node))
        .expect("acyclic protocol clause in acyclic unit")
        .key();
    assert_eq!(
        idle_cyclic, idle_acyclic,
        "acyclic key inputs stay unchanged"
    );

    let emission = emit_checked(&CheckedPackage::link(graph)).expect("package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(model_digest.as_str(), SPINE_MODEL_DOCUMENT)
        .expect("model evidence admitted");
    let pinned =
        qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id());
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &pinned,
    );
    assert!(matches!(read, Read::Verified { .. }), "I2 read: {read:?}");
    let wire_nodes = nodes(&wire);
    let members: Vec<_> = wire_nodes
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .collect();
    assert_eq!(members.len(), group_size);
    assert!(
        members
            .iter()
            .all(|node| node["recursion_group"] == members[0]["recursion_group"])
    );
    let member_refs: Vec<_> = members.iter().copied().collect();
    let label = members[0]["recursion_group"].clone();
    let mut saw_group_reference = false;
    for (ordinal, member) in members.iter().enumerate() {
        let preimage = super::rebuilt_preimage(member, &member_refs, true);
        assert_eq!(preimage["recursion"]["ordinal"], ordinal);
        assert_eq!(preimage["recursion"]["size"], group_size);
        if preimage["recursion"].get("group").is_some() {
            assert_eq!(preimage["recursion"]["group"], label);
        }
        fn group_reference_ordinals(value: &Value, found: &mut Vec<usize>) {
            match value {
                Value::Object(fields) => {
                    if fields.get("term") == Some(&json!("group_reference")) {
                        found.push(
                            fields["ordinal"]
                                .as_u64()
                                .and_then(|ordinal| usize::try_from(ordinal).ok())
                                .expect("group reference ordinal"),
                        );
                    }
                    fields
                        .values()
                        .for_each(|value| group_reference_ordinals(value, found));
                }
                Value::Array(items) => items
                    .iter()
                    .for_each(|value| group_reference_ordinals(value, found)),
                _ => {}
            }
        }
        let mut references = Vec::new();
        group_reference_ordinals(&preimage["body"], &mut references);
        assert!(references.iter().all(|target| *target < group_size));
        saw_group_reference |= !references.is_empty();
    }
    assert!(
        saw_group_reference,
        "wire-derived preimages carry in-group edges"
    );
    let idle = wire_nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "protocol_clause"
                && node["declaration"]["qualified_name"] == json!(["Idle"])
        })
        .expect("acyclic protocol clause");
    assert!(idle["recursion_group"].is_null());
    let idle_group = wire_nodes
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .collect::<Vec<_>>();
    assert!(super::rebuilt_preimage(idle, &idle_group, true)["recursion"].is_null());
    assert_eq!(
        idle["node_id"]["digest"],
        idle_acyclic.to_string(),
        "the acyclic protocol key inputs and identity are unchanged"
    );
}

fn mixed_cycle_text(checks: usize, protocol_first: bool) -> String {
    let fixture = spine_model_without(&[FIELD_ACCESS, CONFORMING_EQUALITY]);
    let fixture = fixture.replace(
        "profile v = \"quire.model.complete/v1\";\n",
        &format!("profile v = \"quire.model.complete/v1\";\nprofile p = \"{PROTOCOL_PROFILE}\";\n"),
    );
    let function = "function refers using v(): Boolean pure { protocol Flow }\n";
    let mut protocol = String::from(
        "protocol Flow using v over (input: M::Gadget) on origin {\n\
         role R on M::Gadget;\n\
         run sequence Main {\n",
    );
    for index in 0..checks {
        use std::fmt::Write as _;
        writeln!(protocol, "check Gate{index:06} using v {{ refers() }};")
            .expect("writing to a String succeeds");
    }
    protocol.push_str("}\nfinish End as (outcome: Boolean) { true };\n}\n");
    if protocol_first {
        format!("{fixture}{protocol}{function}")
    } else {
        format!("{fixture}{function}{protocol}")
    }
}

fn checked_large_text(
    text: &str,
    limits: CheckingLimits,
) -> (qsl_semantics::check::CheckedGraph, String) {
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied model document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let mut parse_limits = qsl_cst::Limits::default();
    parse_limits.source_bytes = parse_limits.source_bytes.max(text.len());
    parse_limits.tokens = parse_limits.tokens.max(text.len());
    parse_limits.nodes = parse_limits.nodes.max(text.len().saturating_mul(16));
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        text.as_bytes(),
        parse_limits,
    )
    .expect("S1 admits the caller-sized unit");
    assert!(
        parsed.diagnostics().is_empty(),
        "S1 diagnostics: {:?}",
        parsed.diagnostics()
    );
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the spine model admits");
    let graph = PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .expect("the unit assembles")
        .check(limits)
        .unwrap_or_else(|refusals| panic!("unit checks: {refusals:?}"));
    (graph, digest)
}

fn recursion_group(
    graph: &qsl_semantics::check::CheckedGraph,
) -> Vec<&qsl_semantics::check::SemanticNode> {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.recursion().is_some())
        .collect()
}

fn minimum_mixed_cycle_work(calls: usize) -> u64 {
    let text = mixed_cycle_text(calls, false);
    let limits = CheckingLimits::default();
    assert!(check_result(&text, limits.with_work_budget(0)).is_err());
    let mut upper = 1_024_u64;
    loop {
        match check_result(&text, limits.with_work_budget(upper)) {
            Ok(_) => break,
            Err(refusals) if upper == u64::MAX => {
                panic!("the mixed cycle refuses even at unlimited work: {refusals:?}")
            }
            Err(_) => upper = upper.checked_mul(2).unwrap_or(u64::MAX),
        }
    }
    let mut lower = 0_u64;
    while upper - lower > 1 {
        let middle = lower + (upper - lower) / 2;
        if check_result(&text, limits.with_work_budget(middle)).is_ok() {
            upper = middle;
        } else {
            lower = middle;
        }
    }
    upper
}

#[trace("FR-383-AC-25")]
#[test]
fn mixed_protocol_cycle_work_is_linear_and_uses_the_exact_budget() {
    let measured = [32, 64, 128].map(minimum_mixed_cycle_work);
    println!("minimum mixed-cycle S3 work for 32/64/128 controls: {measured:?}");
    assert_eq!(measured, [673, 1_217, 2_305]);
    let first_increment = measured[1] - measured[0];
    let second_increment = measured[2] - measured[1];
    assert!(first_increment > 0 && second_increment > 0);
    assert!(
        second_increment <= first_increment.saturating_mul(2)
            && first_increment <= second_increment.saturating_mul(2),
        "doubling cycle nodes doubles charged work within a factor of two: {measured:?}"
    );

    let (function_first, _) = checked(&mixed_cycle_text(128, false));
    let (protocol_first, _) = checked(&mixed_cycle_text(128, true));
    let identity = |graph: &qsl_semantics::check::CheckedGraph| {
        recursion_group(graph)
            .iter()
            .map(|node| *node.recursion().unwrap().group())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        identity(&function_first),
        identity(&protocol_first),
        "permuting authored declaration order leaves the SCC identity unchanged"
    );

    let calls = 64;
    let text = mixed_cycle_text(calls, false);
    let exact = measured[1];
    let limits = CheckingLimits::default();
    assert!(check_result(&text, limits.with_work_budget(0)).is_err());
    assert!(check_result(&text, limits.with_work_budget(exact - 1)).is_err());
    assert!(check_result(&text, limits.with_work_budget(exact)).is_ok());
}

#[trace("FR-383-AC-25")]
#[test]
fn a_100000_member_function_protocol_cycle_is_budgeted_and_order_invariant() {
    const CHECKS: usize = 100_000;
    // The measured mixed-cycle charge is exactly 17 units per control plus
    // 129 fixed units at 32, 64 and 128 controls; use that measured linear
    // charge at full scale rather than an unlimited ceiling.
    const S3_WORK: u64 = 17 * CHECKS as u64 + 129;
    let text = mixed_cycle_text(CHECKS, false);
    let limits = CheckingLimits::new(500_000)
        .with_input_bytes(8_000_000)
        .with_work_budget(S3_WORK);
    let (graph, model_digest) = checked_large_text(&text, limits);
    let group = recursion_group(&graph);
    assert!(
        group.len() == CHECKS + 5,
        "the mixed function/protocol SCC has 100,000 control members plus five protocol/function members: {}",
        group.len()
    );
    assert!(
        group
            .iter()
            .any(|node| node.semantic_form() == "protocol_clause")
    );
    assert!(
        group
            .iter()
            .any(|node| node.semantic_form() == "protocol_reference")
    );

    let emission = emit_checked(&CheckedPackage::link(graph)).expect("package emits");
    assert!(emission.omitted.is_empty());
    let wire = wire(&emission);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(model_digest.as_str(), SPINE_MODEL_DOCUMENT)
        .expect("model evidence admitted");
    let pinned =
        qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id());
    let read_limits = V2ReadLimits::default()
        .with_artifact_bytes(usize::MAX)
        .with_nodes(500_000)
        .with_edges(1_000_000)
        .with_occurrences(500_000)
        .with_work(100_000_000);
    let read = read_v2(
        emission.package.bytes(),
        library(),
        read_limits,
        &evidence,
        &pinned,
    );
    assert!(matches!(read, Read::Verified { .. }), "I2 read: {read:?}");
    let wire_group = nodes(&wire)
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .collect::<Vec<_>>();
    assert_eq!(wire_group.len(), CHECKS + 5);
}
