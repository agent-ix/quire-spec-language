// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-003: actual checked identities through S1, S2, S3 and S4.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::DependencyInput;

use crate::support::front_end::emitted;

#[trace("TC-931", "FR-003-AC-3", "FR-003-AC-9")]
#[test]
fn exact_format_output_bound_keeps_the_original_checked_identity() {
    let text = format!(
        "{}\n// café λ\n",
        include_str!("../fixtures/spine-compile.native")
    );
    let source = SourceIdentity::new("test", "format-output-bound", "fixture", "original");
    let parsed = qsl_cst::parse(
        source.clone(), "unit.native", text.as_bytes(), qsl_cst::Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible());
    let complete = qsl_cst::format::format_with_limits(
        &parsed, qsl_cst::format::FormatLimits::default().with_output_bytes(usize::MAX),
    )
    .unwrap();
    let packages = BTreeMap::new();
    let dependencies = DependencyInput::default();
    let original = emitted(source, "unit.native", text.as_bytes(), &packages, &dependencies)
        .expect("the original fixture reaches public S4");
    for bound in [complete.len(), complete.len() + 1] {
        let limits = qsl_cst::format::FormatLimits::default().with_output_bytes(bound);
        let formatted = qsl_cst::format::format_with_limits(&parsed, limits).unwrap();
        assert_eq!(formatted, complete);
        let identity = SourceIdentity::new("test", "format-output-bound", "fixture", "formatted");
        let reparsed = qsl_cst::parse(
            identity.clone(), "unit.native", formatted.as_bytes(), qsl_cst::Limits::default(),
        )
        .unwrap();
        assert!(reparsed.is_admissible());
        assert_eq!(qsl_cst::format::format_with_limits(&reparsed, limits).unwrap(), formatted);
        let checked = emitted(
            identity, "unit.native", formatted.as_bytes(), &packages, &dependencies,
        )
        .expect("the bounded formatted fixture reaches public S4");
        assert_eq!(checked.package().package_id(), original.package().package_id());
    }
}

#[path = "../../qsl-semantics/tests/it/model_operations.rs"]
#[expect(
    dead_code,
    reason = "two model APIs belong to the owning observation tests"
)]
mod semantic_models;

mod protocol_attempt_fixture {
    use qsl_semantics as semantics;
    include!("../../qsl-semantics/tests/fixtures/protocol_attempt.rs");
}

fn private_tc465_identity(document: Vec<u8>) {
    private_state_source_identity(document, semantic_models::TC465_CLAUSES);
}

fn private_state_source_identity(document: Vec<u8>, body: &str) {
    let declarations = semantic_models::admit_and_assemble_with_body(&document, body)
        .expect("the original private TC465 fixture assembles");
    let graph = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("original private TC465 S3: {refusals:?}"));
    let original = qsl_package::emit_checked(&qsl_package::CheckedPackage::link(graph))
        .unwrap_or_else(|refusal| panic!("original private TC465 public S4: {refusal:?}"));
    assert!(
        original.omitted().is_empty(),
        "private TC465 original S4 omissions"
    );
    let (text, packages) = semantic_models::config_unit_with_body(&document, body);
    let checked = emitted(
        SourceIdentity::new("test", "tc-459", "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .unwrap_or_else(|refusal| panic!("private TC465 formatter spine: {refusal:?}"));
    assert_eq!(
        checked.package().package_id(),
        original.package().package_id(),
        "the formatter oracle uses the owning private TC465 model view"
    );
}

macro_rules! private_tc465_cases {
    ($($name:ident => $document:expr),+ $(,)?) => {
        $(
            #[trace("FR-003-AC-9")]
            #[test]
            fn $name() { private_tc465_identity($document); }
        )+
    };
}

private_tc465_cases! {
    private_tc465_base_keeps_format_identity => semantic_models::tc465_document(),
    private_tc465_parent_only_frame_keeps_format_identity =>
        semantic_models::tc465_document_with_frame_modifies_parent_only(),
    private_tc465_set_field_keeps_format_identity => semantic_models::tc465_document_with_set_field(),
    private_tc465_note_type_keeps_format_identity => semantic_models::tc465_document_with_note_type(),
    private_tc465_tag_field_keeps_format_identity =>
        semantic_models::tc465_document_with_tag_type_sharing_a_field_name(),
    private_tc465_redefined_field_keeps_format_identity =>
        semantic_models::tc465_document_with_sub_redefining_version_number(),
    private_tc465_subtype_keeps_format_identity => semantic_models::tc465_document_with_sub_subtype(),
    private_tc465_archive_keeps_format_identity => semantic_models::tc465_document_with_archive_population(),
    private_tc465_narrow_archive_frame_keeps_format_identity =>
        semantic_models::tc465_document_with_archive_population_and_narrow_frame(),
    private_tc465_unrelated_population_keeps_format_identity =>
        semantic_models::with_unrelated_population(semantic_models::tc465_document()),
    private_tc465_version_only_frame_keeps_format_identity =>
        semantic_models::tc465_document_with(semantic_models::attempt_update_modifies_version_number()),
    private_tc465_deep_hierarchy_keeps_format_identity => semantic_models::tc465_deep_hierarchy_document(70),
    private_tc465_creates_subtype_keeps_format_identity => semantic_models::tc465_creates_subtype_document(),
}

#[trace("FR-003-AC-9")]
#[test]
fn private_frame_clause_source_keeps_format_identity() {
    private_state_source_identity(
        semantic_models::frame_test_document(),
        semantic_models::FRAME_CLAUSES,
    );
}

macro_rules! private_argument_cases {
    ($($name:ident => $factory:ident),+ $(,)?) => {
        $(
            #[trace("FR-003-AC-9")]
            #[test]
            fn $name() {
                let (document, body) = semantic_models::$factory();
                private_state_source_identity(document, &body);
            }
        )+
    };
}

private_argument_cases! {
    private_flag_clause_keeps_format_identity => tc464_flag_source,
    private_sub_target_clause_keeps_format_identity => tc464_sub_target_source,
    private_integer_argument_clause_keeps_format_identity => tc464_integer_argument_source,
}

fn protocol_attempt_identity(declarations: &str) {
    let text = format!("{}{declarations}\n", protocol_attempt_fixture::HEADER);
    let compile = |text: &str, revision: &str| {
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", revision),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .unwrap();
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let forms = qsl_forms::build_unit(&parsed).unwrap();
        let declarations = qsl_semantics::check::PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            forms,
            vec![protocol_attempt_fixture::m_actor_model()],
            Vec::new(),
        )
        .expect("the actual original SelectedModel resolves the protocol");
        let graph = declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{revision}: protocol S3 refuses: {refusals:?}"));
        let emission = qsl_package::emit_checked(&qsl_package::CheckedPackage::link(graph))
            .unwrap_or_else(|refusal| {
                panic!("{revision}: protocol public S4 refuses: {refusal:?}")
            });
        assert!(emission.omitted().is_empty(), "protocol S4 omissions");
        (parsed, emission.package().package_id())
    };
    let (parsed, original) = compile(&text, "1");
    let formatted = qsl_cst::format::format_with_limit(&parsed, usize::MAX).unwrap();
    let (reparsed, formatted_id) = compile(&formatted, "1-formatted");
    assert_eq!(
        qsl_cst::format::format_with_limit(&reparsed, usize::MAX).unwrap(),
        formatted,
        "private protocol second-pass bytes"
    );
    assert_eq!(
        formatted_id, original,
        "private protocol checked package identity"
    );
}

#[trace("FR-003-AC-9")]
#[test]
fn private_protocol_empty_attempt_keeps_format_identity() {
    protocol_attempt_identity(&protocol_attempt_fixture::attempt_flow("Flow", ""));
}

#[trace("FR-003-AC-9")]
#[test]
fn private_protocol_shared_binder_keeps_format_identity() {
    protocol_attempt_identity(&protocol_attempt_fixture::shared_binder_protocols());
}

#[trace("FR-003-AC-9")]
#[test]
fn private_protocol_named_contract_keeps_format_identity() {
    let source = include_str!("../../qsl-semantics/src/check/protocol_clause.rs");
    let template = function_literal(
        source,
        "an_empty_contracts_list_binds_with_no_refusal",
        "post Done ",
    );
    let declarations = template
        .replacen(
            "{}",
            &protocol_attempt_fixture::attempt_flow("Flow", "Done"),
            1,
        )
        .replace("{{", "{")
        .replace("}}", "}");
    protocol_attempt_identity(&declarations);
}

// Reuse the owning model constructors and authored clause literals. Each case
// remains a separate test so an original S4 refusal cannot hide later cases.
fn state_fixture(case: usize) -> (Vec<u8>, String) {
    use semantic_models as models;
    use serde_json::json;

    let fixture = include_str!("../../qsl-semantics/tests/it/state_clauses.rs");
    let update = || {
        models::operation(
            "attemptUpdate",
            json!([]),
            Some("ix://quire/native/Boolean"),
            json!({
                "modifies": [models::config_version_identity("versionNumber")],
                "creates": [], "deletes": []
            }),
        )
    };
    let literal = |function, prefix| function_literal(fixture, function, prefix);
    match case {
        0..=4 => {
            let document =
                models::config_version_document(update(), Vec::new(), Vec::new(), json!([]));
            let parent = source_literal(fixture, "PARENT_ORDER");
            let cycle = source_literal(fixture, "NO_CYCLE");
            let unchanged = source_literal(fixture, "VERSION_UNCHANGED");
            let body = match case {
                0 => format!("{parent}{cycle}{unchanged}"),
                1 => format!("{parent}{cycle}"),
                2 => format!("{unchanged}{cycle}{parent}"),
                3 => format!(
                    "{parent}{cycle}{unchanged}{}",
                    parent.replacen("ParentOrder", "ParentOrder2", 1)
                ),
                4 => literal(
                    "a_postcondition_reads_result_typed_as_the_operations_result",
                    "post R ",
                ),
                _ => unreachable!("bounded case range"),
            };
            (document, body)
        }
        5..=6 => (
            models::config_version_document(
                models::operation(
                    "attemptUpdate",
                    json!([]),
                    Some("ix://quire/native/Boolean"),
                    json!({
                        "modifies": [models::config_version_identity("versionNumber"),
                            models::config_version_identity("parent")],
                        "creates": [], "deletes": []
                    }),
                ),
                Vec::new(),
                Vec::new(),
                json!([]),
            ),
            literal(
                "postconditions_of_an_operation_that_modifies_parent",
                if case == 5 { "post B " } else { "post E " },
            ),
        ),
        7..=9 => (
            models::config_version_document(
                models::operation(
                    "probe",
                    json!([models::operation_parameter(
                        "probe",
                        "target",
                        "ix://example/config-version/ConfigVersion"
                    )]),
                    None,
                    models::empty_frame(),
                ),
                Vec::new(),
                Vec::new(),
                json!([]),
            ),
            literal(
                "clauses_of_an_operation_with_a_reference_typed_parameter",
                ["post A ", "pre B ", "post C "][case - 7],
            ),
        ),
        10..=11 => (
            models::config_version_document_with_operations(vec![
                models::operation(
                    "isStable",
                    json!([]),
                    Some("ix://quire/native/Boolean"),
                    models::empty_frame(),
                ),
                models::operation(
                    "versionTotal",
                    json!([]),
                    Some("ix://quire/native/Integer"),
                    models::empty_frame(),
                ),
            ]),
            literal(
                "frame_occurrence_ordinal_is_independent_of_clause_order",
                if case == 10 { "post A " } else { "post B " },
            ),
        ),
        12 => {
            let document = models::config_version_document_with_operations(vec![
                models::operation(
                    "isStable",
                    json!([]),
                    Some("ix://quire/native/Boolean"),
                    models::empty_frame(),
                ),
                models::operation(
                    "isFresh",
                    json!([]),
                    Some("ix://quire/native/Boolean"),
                    models::empty_frame(),
                ),
            ]);
            let bodies: Vec<_> = function_literals(
                fixture,
                "two_operations_with_equal_frames_each_keep_their_own_frame_record",
            )
            .into_iter()
            .filter(|body| body.starts_with("post A ") && body.contains("isFresh"))
            .collect();
            assert_eq!(bodies.len(), 1, "one owning equal-Boolean-frame source");
            (document, bodies.into_iter().next().unwrap())
        }
        13..=15 => {
            let (function, prefix) = [
                (
                    "population_coverage_is_by_conformance_and_absent_when_none_covers",
                    "invariant SubInvariant ",
                ),
                (
                    "population_domain_key_is_the_populations_own_member_type",
                    "invariant SubInvariant ",
                ),
                (
                    "operation_visibility_on_subtypes_inherits_or_refuses_ambiguous",
                    "post P ",
                ),
            ][case - 13];
            (
                models::subtype_document(update()),
                literal(function, prefix),
            )
        }
        16 => (
            models::ambiguous_operation_document(),
            literal(
                "population_coverage_is_by_conformance_and_absent_when_none_covers",
                "invariant LeftInvariant ",
            ),
        ),
        17 => (
            models::subtype_document_with_two_member_population(update()),
            literal(
                "population_with_several_members_has_one_canonical_domain_key",
                "invariant SubInvariant ",
            ),
        ),
        _ => panic!("unknown original state fixture {case}"),
    }
}

fn state_fixture_identity(case: usize) {
    let (document, body) = state_fixture(case);
    let graph = semantic_models::admit_and_assemble_with_body(&document, &body)
        .expect("the owning original state fixture assembles")
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("state fixture {case}: original S3: {refusals:?}"));
    if case == 13 {
        let (source, _) = semantic_models::config_unit_with_body(&document, &body);
        subtype_fixture_diagnostics(&graph, &source);
    }
    let original = qsl_package::emit_checked(&qsl_package::CheckedPackage::link(graph))
        .unwrap_or_else(|refusal| panic!("state fixture {case}: original public S4: {refusal:?}"));
    assert!(
        original.omitted().is_empty(),
        "state fixture {case}: S4 omitted nodes"
    );
    let (source, packages) = semantic_models::config_unit_with_body(&document, &body);
    let spine = emitted(
        SourceIdentity::new("test", "tc-459", "fixture", "fixture:1"),
        "unit.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .unwrap_or_else(|refusal| panic!("state fixture {case}: formatter spine: {refusal:?}"));
    assert_eq!(
        original.package().package_id(),
        spine.package().package_id(),
        "state fixture {case}: owning S4 and formatter spine must agree"
    );
}

fn subtype_fixture_diagnostics(graph: &qsl_semantics::check::CheckedGraph, source: &str) {
    use quire_semantic_value::location::{Location, Origin};

    eprintln!("state fixture 13: original S3 succeeded; source={source:?}");
    for node in graph.semantic_graph().nodes() {
        // The public structural terms retain the actual links, including
        // nominal links, without deriving an edge or node kind from a key.
        eprintln!(
            "state fixture 13: node={:?} tag={:?} form={} type={:?} declaration={:?} body={:?} nominal={:?}",
            node.key(),
            node.node_tag(),
            node.semantic_form(),
            node.semantic_type(),
            node.declaration(),
            node.body(),
            node.nominal()
        );
    }
    let mut unresolved_generated = 0;
    for (key, origin, location) in graph.occurrences() {
        let node = graph
            .semantic_graph()
            .node(key)
            .expect("the original subtype occurrence names a checked node");
        assert_eq!(graph.occurrence(key, &origin), Some(location));
        let region = graph.occurrence_region(key, &origin);
        eprintln!(
            "state fixture 13: occurrence={key:?}/{origin:?} location={location:?} region={region:?}"
        );
        if origin.role().as_str() == "generated" && region.is_none() {
            unresolved_generated += 1;
            eprintln!(
                "state fixture 13: unresolved Generated node={key:?} role={} ordinal={} location={location:?} tag={:?} form={} preimage={}",
                origin.role().as_str(),
                origin.ordinal(),
                node.node_tag(),
                node.semantic_form(),
                std::str::from_utf8(node.preimage()).expect("canonical preimage is UTF-8")
            );
        }
        let anchor_seed = matches!(
            &location.origin,
            Origin::Body { .. }
                | Origin::Measure { .. }
                | Origin::StateClause { .. }
                | Origin::ProtocolAttempt { .. }
        ) || (matches!(&location.origin, Origin::TypeDeclaration { .. })
            && location.depth() == 0);
        if anchor_seed {
            let root = Location::root(location.origin.clone());
            let root_region = graph.region(&root);
            eprintln!(
                "state fixture 13: anchor seed node={key:?} root={root:?} region={root_region:?}"
            );
            if let Some(region) = root_region {
                assert_eq!(region.source(), graph.source(), "original anchor source");
                assert!(
                    region.start() < region.end(),
                    "real anchor span is nonempty"
                );
                let start = usize::try_from(region.start()).unwrap();
                let end = usize::try_from(region.end()).unwrap();
                let bytes = source
                    .get(start..end)
                    .expect("the anchor span selects the original authored unit");
                eprintln!("state fixture 13: anchor root={root:?} authored bytes={bytes:?}");
                if let Origin::StateClause { clause, index } = &root.origin {
                    assert_eq!(
                        graph.state_clauses()[*index].name(),
                        clause.as_str(),
                        "the anchor names its actual checked clause"
                    );
                }
            }
        }
    }
    eprintln!(
        "state fixture 13: unresolved Generated count={unresolved_generated}; strict S4 next"
    );
    assert_eq!(graph.requirements().len(), 1, "one original subtype claim");
    let record = graph.requirements().values().next().unwrap();
    let qsl_semantics::family::ClaimExtent::Unbounded(domains) = record.requirements().extent()
    else {
        panic!("the original subtype is covered by an unbounded population");
    };
    assert_eq!(domains.len(), 1, "one original covering population");
    let (domain, kind) = domains.iter().next().unwrap();
    assert_eq!(kind, qsl_foundation::bound::DomainKind::Population);
    let qsl_foundation::bound::DomainKey::Population { member_type, .. } = domain else {
        panic!("the requirement names its population's own member type");
    };
    let member = graph.semantic_graph().resolve_wire(*member_type).unwrap();
    let node = graph.semantic_graph().node(member).unwrap();
    let Some(qsl_semantics::check::Owner::Model(owner)) = node.owner() else {
        panic!("the population member is the original model-owned type");
    };
    assert_eq!(owner.identity(), "example/config-version");
    assert_eq!(owner.node(), "ix://example/config-version/ConfigVersion");
    assert_eq!(node.semantic_form(), "object_type");
    let occurrences: Vec<_> = graph
        .occurrences()
        .filter(|(key, _, _)| *key == member)
        .collect();
    assert_eq!(
        occurrences.len(),
        1,
        "one population member type occurrence"
    );
    let (_, origin, location) = &occurrences[0];
    assert_eq!(
        origin.role().as_str(),
        "type",
        "population type is recorded"
    );
    assert_eq!(origin.ordinal(), 0, "first population type occurrence");
    let Origin::StateClause { clause, index } = &location.origin else {
        panic!("the population type is placed at its actual clause: {location:?}");
    };
    assert_eq!(clause, "SubInvariant");
    assert_eq!(*index, 0);
    assert_eq!(location.depth(), 0);
    let region = graph
        .occurrence_region(member, origin)
        .expect("the population type has its real clause body region");
    assert_eq!(region.source(), graph.source());
    let start = source.find("{ true }").expect("original subtype body") + 2;
    assert_eq!(usize::try_from(region.start()).unwrap(), start);
    assert_eq!(usize::try_from(region.end()).unwrap(), start + "true".len());
    assert_eq!(&source[start..start + "true".len()], "true");
}

macro_rules! state_identity_tests {
    ($($name:ident: $case:literal),+ $(,)?) => {
        $(
            #[trace("TC-885", "FR-003-AC-9")]
            #[test]
            fn $name() {
                state_fixture_identity($case);
            }
        )+
    };
}

state_identity_tests! {
    state_all_three_clauses_keep_checked_identity: 0,
    state_parent_and_cycle_keep_checked_identity: 1,
    state_reordered_clauses_keep_checked_identity: 2,
    state_duplicate_parent_clause_keeps_checked_identity: 3,
    state_boolean_result_keeps_checked_identity: 4,
    state_modified_parent_present_keeps_checked_identity: 5,
    state_modified_parent_equality_keeps_checked_identity: 6,
    state_reference_parameter_post_keeps_checked_identity: 7,
    state_reference_parameter_pre_keeps_checked_identity: 8,
    state_reference_parameter_comparison_keeps_checked_identity: 9,
    state_forward_frame_order_keeps_checked_identity: 10,
    state_reverse_frame_order_keeps_checked_identity: 11,
    state_equal_boolean_frames_keep_checked_identity: 12,
    state_subtype_population_keeps_checked_identity: 13,
    state_population_member_type_keeps_checked_identity: 14,
    state_inherited_operation_keeps_checked_identity: 15,
    state_uncovered_population_keeps_checked_identity: 16,
    state_two_member_population_keeps_checked_identity: 17,
}

// FR-093 generated-placement support for original state-clause roots. The
// enum/record vectors of AC-18 and formatting identity have separate controls.
#[trace("FR-093")]
#[test]
fn state_generated_occurrences_have_authored_regions() {
    use quire_semantic_value::location::Origin;

    let (document, body) = state_fixture(0);
    let (source, _) = semantic_models::config_unit_with_body(&document, &body);
    let graph = semantic_models::admit_and_assemble_with_body(&document, &body)
        .unwrap()
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("the original three clauses check before emission");
    let authored_names = ["ParentOrder", "NoCycle", "VersionUnchanged"];
    assert_eq!(graph.state_clauses().len(), authored_names.len());
    for (clause, expected) in graph.state_clauses().iter().zip(authored_names) {
        assert_eq!(clause.name(), expected, "original authored clause order");
    }
    let mut generated_zero = 0;
    for (key, origin, location) in graph.occurrences() {
        if origin.role().as_str() != "generated" {
            continue;
        }
        if origin.ordinal() == 0 {
            generated_zero += 1;
        }
        let node = graph
            .semantic_graph()
            .node(key)
            .expect("the occurrence names a node");
        let region = graph.region(location).unwrap_or_else(|| {
            panic!(
                "generated occurrence {key:?}/{origin:?} at {location:?} has no authored region; preimage={}",
                std::str::from_utf8(node.preimage()).expect("canonical preimage is UTF-8")
            )
        });
        assert_eq!(
            region.source(),
            graph.source(),
            "the region names the actual unit"
        );
        assert!(
            region.start() < region.end(),
            "generated occurrence must be placed"
        );
        let Origin::StateClause {
            clause: name,
            index,
        } = &location.origin
        else {
            panic!("generated state-fixture node must name its enclosing clause: {location:?}");
        };
        let expected = authored_names
            .get(*index)
            .expect("the clause index names an original authored declaration");
        assert_eq!(name.as_str(), *expected, "authored clause name and index");
        assert_eq!(
            graph.state_clauses()[*index].name(),
            name.as_str(),
            "the occurrence names its checked clause"
        );
        assert_eq!(
            location.depth(),
            0,
            "generated node uses the clause body root"
        );
        let declaration = source
            .find(&format!(" {name} using "))
            .expect("authored clause name");
        let start = declaration + source[declaration..].find('{').unwrap() + 1;
        let end = start + source[start..].find('}').unwrap();
        let actual_start = usize::try_from(region.start()).unwrap();
        let actual_end = usize::try_from(region.end()).unwrap();
        assert_eq!(
            &source[actual_start..actual_end],
            source[start..end].trim(),
            "generated occurrence {key:?}/{origin:?}: exact authored enclosing body"
        );
    }
    assert!(
        generated_zero > 0,
        "the original fixture exercises Generated ordinal 0"
    );
    let original = qsl_package::emit_checked(&qsl_package::CheckedPackage::link(graph))
        .expect("actual public S4 requires all original occurrences to be located");
    assert!(original.omitted().is_empty());
}

fn function_literals(test_source: &str, function: &str) -> Vec<String> {
    use syn::visit::Visit;
    struct Strings(Vec<String>);
    impl<'ast> Visit<'ast> for Strings {
        fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
            self.0.push(literal.value());
        }
        fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
            use syn::parse::Parser;
            if let Ok(arguments) =
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                    .parse2(expression.mac.tokens.clone())
            {
                for argument in &arguments {
                    self.visit_expr(argument);
                }
            }
        }
    }
    let file = syn::parse_file(test_source).unwrap();
    fn find<'a>(items: &'a [syn::Item], name: &str) -> Option<&'a syn::Block> {
        items.iter().find_map(|item| match item {
            syn::Item::Fn(item) if item.sig.ident == name => Some(item.block.as_ref()),
            syn::Item::Impl(item) => item.items.iter().find_map(|item| match item {
                syn::ImplItem::Fn(item) if item.sig.ident == name => Some(&item.block),
                _ => None,
            }),
            syn::Item::Mod(item) => item
                .content
                .as_ref()
                .and_then(|(_, items)| find(items, name)),
            _ => None,
        })
    }
    let item = find(&file.items, function)
        .unwrap_or_else(|| panic!("owning fixture function {function} exists"));
    let mut strings = Strings(Vec::new());
    strings.visit_block(item);
    strings.0
}

fn function_literal(test_source: &str, function: &str, prefix: &str) -> String {
    let matches: Vec<_> = function_literals(test_source, function)
        .into_iter()
        .filter(|literal| literal.starts_with(prefix))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "{function}: exactly one authored literal starting {prefix:?}"
    );
    matches.into_iter().next().unwrap()
}

// Instantiate the named substitutions of an owning Rust format! literal.
// Braces in that literal are Rust's escaped braces, not newly authored QSL.
fn fixture_text(mut template: String, substitutions: &[(&str, &str)]) -> String {
    for (name, value) in substitutions {
        let placeholder = format!("{{{name}}}");
        assert!(
            template.contains(&placeholder),
            "fixture uses {placeholder}"
        );
        template = template.replace(&placeholder, value);
    }
    template.replace("{{", "{").replace("}}", "}")
}

fn literal_fixture_arguments(test_source: &str, callees: &[&str]) -> Vec<String> {
    use syn::visit::Visit;
    struct Calls<'a> {
        callees: &'a [&'a str],
        sources: std::collections::BTreeSet<String>,
    }
    impl<'ast> Visit<'ast> for Calls<'_> {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref() {
                if path
                    .path
                    .get_ident()
                    .is_some_and(|name| self.callees.iter().any(|callee| name == callee))
                {
                    if let Some(syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(source),
                        ..
                    })) = call.args.first()
                    {
                        self.sources.insert(source.value());
                    }
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let mut calls = Calls {
        callees,
        sources: std::collections::BTreeSet::new(),
    };
    calls.visit_file(&syn::parse_file(test_source).unwrap());
    assert!(
        !calls.sources.is_empty(),
        "owning fixture calls contain authored source literals"
    );
    calls.sources.into_iter().collect()
}

fn plain_fixture(text: &str) {
    emitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("original owning fixture checks");
}

fn replay_fixture(text: &str) {
    emitted(
        SourceIdentity::new("agent-ix", "test:replay", "git", "r1"),
        "test:replay",
        text.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .unwrap();
}

/// S1–S4 under the evaluator's actual post-S2 profile injection.
fn ieee_fixture_identity(text: &str) {
    use qsl_semantics::check::PackageDeclarations;
    use qsl_semantics::value::{CatalogRole, DefinitionLock, DefinitionReference};
    let compile = |text: &str, revision: &str| {
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", revision),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .unwrap();
        let built = qsl_forms::build_unit(&parsed).unwrap();
        let mut declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            built,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let lock = DefinitionLock::pinned();
        let entry = lock.entry(CatalogRole::IeeeProfile);
        declarations.ieee_profile = Some(
            lock.admit_ieee_profile(
                &[DefinitionReference {
                    authority: entry.authority.to_owned(),
                    identity: entry.identity.to_owned(),
                }],
                &[],
            )
            .unwrap(),
        );
        let graph = declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect("the owning IEEE profile admits the fixture");
        let emission =
            qsl_package::emit_checked(&qsl_package::CheckedPackage::link(graph)).unwrap();
        assert!(
            emission.omitted().is_empty(),
            "IEEE {revision}: emission omitted {:?}",
            emission.omitted()
        );
        emission.package().package_id()
    };
    let original = compile(text, "1");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    let formatted = qsl_cst::format::format(&parsed).unwrap();
    let reparsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "formatted"),
        "unit.native",
        formatted.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    assert_eq!(
        qsl_cst::format::format(&reparsed).unwrap(),
        formatted,
        "IEEE second pass"
    );
    assert_eq!(
        compile(&formatted, "1-formatted"),
        original,
        "IEEE formatting changed checked package identity"
    );
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn all_evaluator_rounding_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-eval/tests/it/float_rounding.rs");
    let template = function_literal(fixture, "add", "language ");
    for float in [
        "Float64[nearest-even]",
        "Float64[nearest-away]",
        "Float64[toward-positive]",
        "Float64[toward-zero]",
        "Float64[toward-negative]",
        "Float64",
    ] {
        ieee_fixture_identity(&fixture_text(template.clone(), &[("float", float)]));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn both_evaluator_name_variants_keep_format_identity() {
    let fixture = include_str!("../../qsl-eval/tests/it/evaluation_ignores_display_strings.rs");
    let template = function_literal(fixture, "unit", "{HEADER}");
    let header = source_literal(fixture, "HEADER");
    for (alias, inner, parameter, outer) in [
        ("Digit", "inc", "x", "twice"),
        ("Nibble", "bump", "operand", "double_step"),
    ] {
        plain_fixture(&fixture_text(
            template.clone(),
            &[
                ("HEADER", &header),
                ("alias", alias),
                ("inner", inner),
                ("parameter", parameter),
                ("outer", outer),
            ],
        ));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn evaluator_enum_and_predicate_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-eval/tests/it/source_call.rs");
    let template = function_literal(fixture, "package_of", "language ");
    let declarations = literal_fixture_arguments(fixture, &["package_of"]);
    assert_eq!(
        declarations.len(),
        2,
        "both actual source-call generated units"
    );
    for declarations in declarations {
        plain_fixture(&fixture_text(
            template.clone(),
            &[("declarations", &declarations)],
        ));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn emitter_authored_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-package/src/emit/tests.rs");
    plain_fixture(&source_literal(fixture, "SPINE_TEXT"));
    let template = function_literal(fixture, "package_from_text", "language ");
    for declarations in literal_fixture_arguments(
        fixture,
        &[
            "package_from_text",
            "emit_from_text",
            "members_are_placed_at",
        ],
    ) {
        plain_fixture(&fixture_text(
            template.clone(),
            &[("declarations", &declarations)],
        ));
    }
    plain_fixture(&fixture_text(
        template,
        &[(
            "declarations",
            &source_literal(fixture, "ORDERED_COMPARISON"),
        )],
    ));
    let model = include_str!("../fixtures/spine-model.native");
    let field = source_literal(fixture, "FIELD_ACCESS");
    let equality = source_literal(fixture, "CONFORMING_EQUALITY");
    let document = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    for removed in [
        &[field.as_str(), equality.as_str()][..],
        &[field.as_str()][..],
        &[equality.as_str()][..],
    ] {
        let source = removed.iter().fold(model.to_owned(), |source, removed| {
            assert!(
                source.contains(removed),
                "owning model fixture contains removed declaration"
            );
            source.replace(removed, "")
        });
        emitted(
            SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
            "program.native",
            source.as_bytes(),
            &packages,
            &DependencyInput::default(),
        )
        .unwrap();
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn checked_claim_and_extent_fixture_units_keep_format_identity() {
    let claims = include_str!("../../qsl-semantics/src/check/claims/tests.rs");
    let header = source_literal(claims, "HEADER");
    let declarations: Vec<_> = function_literals(claims, "fixtures")
        .into_iter()
        .filter(|literal| literal.starts_with("function "))
        .collect();
    assert_eq!(
        declarations.len(),
        16,
        "every actual requirement-record fixture unit"
    );
    for declarations in declarations
        .into_iter()
        .chain(literal_fixture_arguments(
            claims,
            &["check", "family_claims"],
        ))
        .chain(
            function_literals(claims, "guard_fixtures")
                .into_iter()
                .filter(|literal| literal.starts_with("function ")),
        )
        .chain(
            function_literals(
                claims,
                "tc_160_sibling_lets_keep_their_extents_in_both_orders",
            )
            .into_iter()
            .filter(|literal| literal.starts_with("function ")),
        )
        .chain(
            function_literals(claims, "tc_160_fold_reduce_and_flat_map_binders_are_roots")
                .into_iter()
                .filter(|literal| literal.starts_with("type ")),
        )
        .chain(
            function_literals(claims, "tc_160_checking_a_unit_twice_gives_equal_records")
                .into_iter()
                .filter(|literal| literal.starts_with("function ")),
        )
    {
        plain_fixture(&format!("{header}{declarations}\n"));
    }
    let extent = include_str!("../../qsl-package/src/emit/extent_agreement.rs");
    let header = source_literal(extent, "HEADER");
    for declarations in literal_fixture_arguments(extent, &["emit_text"]) {
        plain_fixture(&format!("{header}{declarations}\n"));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn assembler_checked_owner_and_declaration_variants_keep_format_identity() {
    let fixture = include_str!("../../qsl-semantics/src/check/assemble/tests.rs");
    let profile = source_literal(fixture, "PROFILE_V");
    let header = format!("language \"ix:native\" edition \"1-draft\";\n{profile}");
    for (function, prefix) in [
        (
            "the_assembler_resolves_aliases_and_signatures",
            "type Digit ",
        ),
        (
            "a_declared_type_resolves_to_its_names_region",
            "record Point ",
        ),
        (
            "a_predicate_assembles_as_a_function_of_kind_predicate",
            "predicate Positive ",
        ),
        (
            "predicate_and_enum_nodes_lower_to_their_fr_092_forms",
            "predicate Positive ",
        ),
        (
            "predicate_and_enum_nodes_lower_to_their_fr_092_forms",
            "function Positive ",
        ),
    ] {
        plain_fixture(&format!(
            "{header}{}\n",
            function_literal(fixture, function, prefix)
        ));
    }
    let status = source_literal(fixture, "STATUS");
    plain_fixture(&format!("{header}{status}\n"));
    plain_fixture(&format!(
        "{header}{}\n",
        fixture_text(
            function_literal(
                fixture,
                "enum_types_and_members_check_and_a_case_the_enum_lacks_refuses",
                "{STATUS}function isReady "
            ),
            &[("STATUS", &status)]
        )
    ));
    let declarations = function_literal(
        fixture,
        "records_and_tuples_are_keyed_over_the_units_owner",
        "record Point ",
    );
    for owner in ["u", "w"] {
        emitted(
            SourceIdentity::new("a", owner, "git", "1"),
            "unit.native",
            format!("{header}{declarations}\n").as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .unwrap();
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn syntax_and_legacy_file_fixtures_have_observed_original_refusals() {
    for (path, bytes) in [
        (
            "composed-choreography.native",
            include_bytes!("../fixtures/composed-choreography.native").as_slice(),
        ),
        (
            "parent.native",
            include_bytes!("../fixtures/parent.native").as_slice(),
        ),
        (
            "native-package/controls.native",
            include_bytes!("../fixtures/native-package/controls.native").as_slice(),
        ),
        (
            "native-package/header.native",
            include_bytes!("../fixtures/native-package/header.native").as_slice(),
        ),
        (
            "native-package/minimal.native",
            include_bytes!("../fixtures/native-package/minimal.native").as_slice(),
        ),
        (
            "native-package/multiple.native",
            include_bytes!("../fixtures/native-package/multiple.native").as_slice(),
        ),
    ] {
        let refusal = emitted(
            SourceIdentity::new("test", path, "fixture", "1"),
            path,
            bytes,
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .expect_err("original is not an admitted complete-V1 unit");
        assert!(
            !refusal.code().is_incomplete(),
            "{path}: refusal is invalid original input, not an exhausted limit"
        );
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn lifecycle_generated_bodies_keep_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/spine/lifecycle/tests.rs");
    let header = source_literal(fixture, "HEADER");
    let identity = || SourceIdentity::new("agent-ix", "test:lifecycle", "fixture", "fixture:1");
    let compile = |source: &str, limits| {
        crate::support::front_end::emitted_with_authority(
            identity(),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            limits,
            &qsl_replay::spine::LockEvidence::default(),
        )
        .unwrap()
    };
    let body = vec!["1"; 40].join(" + ");
    compile(
        &fixture_text(
            function_literal(fixture, "long_body_chain", "{HEADER}"),
            &[("HEADER", &header), ("body", &body)],
        ),
        qsl_replay::spine::SpineLimits::default(),
    );
    let mut source = format!("{header}function d0 using v(): Integer pure {{ 1 }}\n");
    let line = function_literal(fixture, "doubling", "function d{level}");
    for level in 1..24 {
        source.push_str(&fixture_text(
            line.clone(),
            &[
                ("level", &level.to_string()),
                ("below", &(level - 1).to_string()),
            ],
        ));
        source.push('\n');
    }
    compile(&source, qsl_replay::spine::SpineLimits::default());
    let mut source = header;
    source.push_str(&function_literal(fixture, "sum", "function big "));
    source.push_str(&function_literal(fixture, "sum", " + ").repeat(2_999));
    source.push_str(&function_literal(fixture, "sum", " }"));
    let mut limits = qsl_replay::spine::SpineLimits::default();
    limits.source.source_bytes = source.len();
    limits.source.tokens = usize::MAX / 2;
    limits.source.nodes = usize::MAX / 2;
    limits.checking = quire_semantic_value::checking::CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    compile(&source, limits);
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn lifecycle_seeded_admitted_units_keep_format_identity() {
    // The owning TC-756 generator: all 600 rounds, including its refused
    // bytes. Only an actual successful original S1–S4 admission reaches the
    // helper's identity oracle; a formatted failure never becomes a skip.
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut below = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        usize::try_from(state % u64::try_from(bound).unwrap()).unwrap()
    };
    let valid = include_bytes!("../fixtures/spine-compile.native");
    let mut checked = 0;
    let mut refusals = BTreeMap::<String, usize>::new();
    for round in 0..600 {
        let mut bytes = valid.to_vec();
        match round % 3 {
            0 => {
                bytes = (0..below(200))
                    .map(|_| u8::try_from(below(256)).unwrap())
                    .collect()
            }
            1 => {
                for _ in 0..=below(4) {
                    let at = below(bytes.len());
                    bytes[at] = u8::try_from(below(256)).unwrap();
                }
            }
            _ => bytes.truncate(below(bytes.len())),
        }
        match emitted(
            SourceIdentity::new("agent-ix", "test:lifecycle", "fixture", "fixture:1"),
            "unit.native",
            &bytes,
            &BTreeMap::new(),
            &DependencyInput::default(),
        ) {
            Ok(_) => checked += 1,
            Err(refusal) => {
                *refusals
                    .entry(format!("{:?}/{:?}", refusal.stage(), refusal.code()))
                    .or_default() += 1;
            }
        }
    }
    assert!(
        checked > 0,
        "the original owning generator has admitted positive controls"
    );
    eprintln!("TC-756: {checked} original admitted units also preserve formatting identity; {} original refusals", 600 - checked);
    eprintln!("TC-756 original refusal stages/codes: {refusals:?}");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn lifecycle_large_declaration_unit_keeps_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/spine/lifecycle/tests.rs");
    let mut source = source_literal(fixture, "HEADER");
    let line = function_literal(fixture, "declarations", "function f{index}");
    for index in 0..200_000 {
        source.push_str(&fixture_text(
            line.clone(),
            &[("index", &index.to_string())],
        ));
        source.push('\n');
    }
    let mut limits = qsl_replay::spine::SpineLimits::default();
    limits.source.source_bytes = source.len();
    limits.source.tokens = usize::MAX / 2;
    limits.source.nodes = usize::MAX / 2;
    limits.checking = quire_semantic_value::checking::CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    crate::support::front_end::emitted_with_authority(
        SourceIdentity::new("agent-ix", "test:lifecycle", "fixture", "fixture:1"),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits,
        &qsl_replay::spine::LockEvidence::default(),
    )
    .unwrap();
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn all_three_deep_source_generators_keep_format_identity() {
    let fixture = include_str!("../../qsl-package/src/emit/tests/deep_bodies.rs");
    let header = source_literal(fixture, "HEADER");
    let start = fixture_text(
        function_literal(fixture, "deep_source", "{HEADER}"),
        &[("HEADER", &header)],
    );
    let end = function_literal(fixture, "deep_source", "x }");
    for label in ["sum", "else if", "let"] {
        let mut source = start.clone();
        for level in 0..100_000 {
            match label {
                "sum" if level == 0 => {}
                "sum" => source.push_str("x + "),
                "else if" => source.push_str("if a then x else "),
                _ => source.push_str(&format!("let b{level} = x in ")),
            }
        }
        source.push_str(&end);
        let bytes = source.len();
        let mut limits = qsl_replay::spine::SpineLimits::default();
        limits.source.source_bytes = limits.source.source_bytes.max(bytes);
        limits.source.tokens = limits.source.tokens.max(bytes);
        limits.source.nodes = limits.source.nodes.max(bytes.saturating_mul(16));
        limits.checking = quire_semantic_value::checking::CheckingLimits::new(u64::MAX)
            .with_input_bytes(u64::MAX)
            .with_work_budget(u64::MAX);
        crate::support::front_end::emitted_with_authority(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            limits,
            &qsl_replay::spine::LockEvidence::default(),
        )
        .unwrap_or_else(|refusal| {
            panic!("{label}: actual 100,000-level fixture must check: {refusal}")
        });
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn replay_large_source_generator_keeps_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/execute/tests/composite.rs");
    let profile = source_literal(
        include_str!("../../qsl-replay/src/execute/tests.rs"),
        "PROFILE",
    );
    let sum = vec!["l.head"; 100_000].join(" + ");
    let body = fixture_text(
        function_literal(
            fixture,
            "tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict",
            "record List ",
        ),
        &[("sum", &sum)],
    );
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{profile}{body}");
    let mut limits = qsl_replay::spine::SpineLimits {
        source: qsl_cst::Limits {
            source_bytes: 1 << 28,
            tokens: 1 << 28,
            nodes: 1 << 28,
            work_units: 1 << 20,
        },
        ..qsl_replay::spine::SpineLimits::default()
    };
    limits.checking = limits
        .checking
        .with_nodes(1 << 40)
        .with_input_bytes(1 << 40)
        .with_work_budget(1 << 40);
    crate::support::front_end::emitted_with_authority(
        SourceIdentity::new("agent-ix", "test:replay", "git", "r1"),
        "test:replay",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits,
        &qsl_replay::spine::LockEvidence::default(),
    )
    .unwrap();
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn longest_admitted_and_chain_keeps_format_identity() {
    use qsl_semantics::check::PackageDeclarations;
    let fixture = include_str!("../../qsl-package/src/emit/tests/deep_bodies.rs");
    let template = function_literal(fixture, "and_chain", "language ");
    let text = |levels: usize| {
        template
            .replacen("{}", &"a and (".repeat(levels), 1)
            .replacen("{}", &")".repeat(levels), 1)
            .replace("{{", "{")
            .replace("}}", "}")
    };
    // Match the owning S1/S2/assembly boundary search, then check its actual
    // maximum through S3/S4, rather than substituting a smaller sample.
    let admits = |levels| {
        let text = text(levels);
        let Ok(parsed) = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        ) else {
            return false;
        };
        if !parsed.is_admissible() {
            return false;
        }
        let Ok(forms) = qsl_forms::build_unit(&parsed) else {
            return false;
        };
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            forms,
            Vec::new(),
            Vec::new(),
        )
        .is_ok()
    };
    let (mut admitted, mut refused) = (1, qsl_cst::Limits::default().nodes);
    assert!(admits(admitted) && !admits(refused));
    while refused - admitted > 1 {
        let middle = admitted + (refused - admitted) / 2;
        if admits(middle) {
            admitted = middle;
        } else {
            refused = middle;
        }
    }
    plain_fixture(&text(admitted));
    eprintln!("TC-727: exact owning longest admitted source has {admitted} connectives");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn replay_execute_generators_keep_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/execute/tests.rs");
    let profile = source_literal(fixture, "PROFILE");
    for function in [
        "proved",
        "parity_unit",
        "tc_166_case_variant_functions_each_replay_their_own_body",
        "tc_913_a_u64_field_model_recompiles_to_its_package_id",
    ] {
        let source = fixture_text(
            function_literal(fixture, function, "language "),
            &[("PROFILE", &profile)],
        );
        replay_fixture(&source);
        if function == "proved" {
            replay_fixture(&source.replace("x < 5", "x < 6"));
            replay_fixture(&source.replacen('\n', "\n\n", 1));
        }
    }
    for (parameter, body) in [("Int[0, 18446744073709551615]", "x <= 18446744073709551614"),
        ("Int[0, 9223372036854775808]", "x <= 9223372036854775807"),
        ("Int[0, 18446744073709551616]", "x <= 18446744073709551615"),
        ("Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]",
         "x > -170141183460469231731687303715884105728")] {
        replay_fixture(&fixture_text(function_literal(fixture, "wide_source", "language "),
            &[("PROFILE", &profile), ("parameter", parameter), ("body", body)]));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn replay_composite_and_model_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/execute/tests/composite.rs");
    let profile = source_literal(
        include_str!("../../qsl-replay/src/execute/tests.rs"),
        "PROFILE",
    );
    let header = format!("language \"ix:native\" edition \"1-draft\";\n{profile}");
    for constant in ["NESTED", "LEAVES"] {
        replay_fixture(&format!("{header}{}", source_literal(fixture, constant)));
    }
    for function in [
        "tc_906_a_set_holding_numerically_equal_decimals_refuses",
        "tc_736_a_100000_long_list_replays_under_the_raised_input_bound",
    ] {
        replay_fixture(&format!(
            "{header}{}",
            function_literal(
                fixture,
                function,
                if function.contains("decimals") {
                    "function "
                } else {
                    "record "
                }
            )
        ));
    }
    replay_fixture(&format!(
        "{header}function t using v(b: Boolean): Boolean pure {{ b }}\n"
    ));
    let document = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let digest = qsl_semantics::model::key::hex(packages.keys().next().unwrap());
    let body = function_literal(
        fixture,
        "tc_906_a_reference_replays_by_identity_and_a_dereference_completes_no_value",
        "function ",
    );
    let source = fixture_text(
        function_literal(fixture, "model_fixture", "language "),
        &[("PROFILE", &profile), ("hex", &digest), ("body", &body)],
    );
    emitted(
        SourceIdentity::new("agent-ix", "test:replay", "git", "r1"),
        "test:replay",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .unwrap();
    let source = fixture_text(
        function_literal(
            include_str!("../../qsl-replay/src/execute/tests.rs"),
            "tc_444_a_domain_package_comes_from_the_byte_provision",
            "language ",
        ),
        &[("PROFILE", &profile), ("hex", &digest)],
    );
    emitted(
        SourceIdentity::new("agent-ix", "test:replay", "git", "r1"),
        "test:replay",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .unwrap();
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn lifecycle_text_law_fixture_keeps_format_identity() {
    use qsl_replay::spine::{LockEvidence, SpineLimits};
    use qsl_semantics::value::{CatalogRole, DefinitionLock};
    let fixture = include_str!("../../qsl-replay/src/spine/lifecycle/tests.rs");
    let header = source_literal(fixture, "HEADER");
    let source = fixture_text(
        function_literal(
            fixture,
            "check_takes_the_lock_evidence_the_text_law_comes_from",
            "{HEADER}",
        ),
        &[("HEADER", &header)],
    );
    let lock = LockEvidence::default().with_text_profile(
        DefinitionLock::pinned()
            .entry(CatalogRole::TextProfile)
            .reference(),
    );
    let compile = |lock: &LockEvidence| {
        crate::support::front_end::emitted_with_authority(
            SourceIdentity::new("agent-ix", "test:lifecycle", "fixture", "fixture:1"),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            lock,
        )
    };
    assert!(
        compile(&LockEvidence::default()).is_err(),
        "original control lacks the text law"
    );
    compile(&lock).expect("owning lock evidence admits original and formatted text unit");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn dependency_library_and_scalar_variants_keep_format_identity() {
    use qsl_replay::spine::SuppliedLibrary;
    let fixture = include_str!("../../qsl-replay/src/spine/dependency_tests.rs");
    let header = source_literal(fixture, "HEADER");
    let h = source_literal(fixture, "H");
    let compile = |body: &str, caller: &str| {
        let library = SuppliedLibrary {
            identity: "test/geometry".to_owned(),
            source: SourceIdentity::new("a", "geometry", "git", "1"),
            path: "geometry.native".to_owned(),
            bytes: format!("{header}{body}").into_bytes(),
        };
        emitted(
            library.source.clone(),
            &library.path,
            &library.bytes,
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .expect("original library checks independently");
        let source = format!("{header}import \"test/geometry\" as g;\n{caller}");
        crate::support::front_end::dependency_fixture_identity(
            SourceIdentity::new("a", "u", "git", "1"),
            "u.native",
            &source,
            vec![library],
        );
    };
    for constant in ["F", "F_CHANGED"] {
        let body = source_literal(fixture, constant);
        compile(&body, &h);
        compile(
            &body,
            "function p using v(y: Int[0, 9]): Boolean pure { g::f(y) }\n",
        );
    }
    compile(
        &source_literal(fixture, "F"),
        &fixture_text(
            function_literal(
                fixture,
                "an_imported_call_is_typed_from_the_library_and_lowered_to_a_dependency_reference",
                "{declaration}function p using v(): Boolean pure {{ g::f(3)",
            ),
            &[("declaration", "")],
        ),
    );
    for kind in [
        "Decimal[0, 100; 0, 2; nearest-even]",
        "Rational[0, 1; 1, 5]",
        "Text[1, 100; nfc]",
        "Option<Boolean>",
        "Set<Int[0, 9]>[0, 4]",
    ] {
        let function = "an_imported_call_over_each_independent_scalar_kind_checks_and_reads";
        let library = fixture_text(
            function_literal(fixture, function, "function f "),
            &[("kind", kind)],
        );
        let caller = fixture_text(
            function_literal(fixture, function, "{}function p "),
            &[("", ""), ("kind", kind)],
        );
        compile(&library, &caller);
    }
    for function in [
        "an_imported_call_evaluates_in_the_library_and_returns_to_the_caller",
        "a_halt_inside_an_imported_body_is_located_at_the_callers_call",
    ] {
        let library = function_literal(
            fixture,
            function,
            if function.starts_with("a_halt") {
                "function f "
            } else {
                "function small "
            },
        );
        let caller = fixture_text(
            function_literal(
                fixture,
                function,
                if function.starts_with("a_halt") {
                    "{}function p "
                } else {
                    "{}function local "
                },
            ),
            &[("", "")],
        );
        compile(&library, &caller);
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn dependency_chain_and_order_variants_keep_format_identity() {
    use crate::support::front_end::dependency_fixture_identity;
    use qsl_replay::spine::SuppliedLibrary;
    let fixture = include_str!("../../qsl-replay/src/spine/dependency_tests.rs");
    let header = source_literal(fixture, "HEADER");
    let h = source_literal(fixture, "H");
    let library = |identity: &str, name: &str, body: &str| SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("a", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: format!("{header}{body}").into_bytes(),
    };
    let run = |body: String, libraries| {
        dependency_fixture_identity(
            SourceIdentity::new("a", "u", "git", "1"),
            "u.native",
            &format!("{header}{body}"),
            libraries,
        )
    };
    for imports in [
        "import \"test/b\" as lb;\nimport \"test/a\" as la;\n",
        "import \"test/a\" as la;\nimport \"test/b\" as lb;\n",
    ] {
        run(
            format!("{imports}{h}"),
            vec![library("test/b", "b", &h), library("test/a", "a", &h)],
        );
    }
    let names = ["Z", "a", "a.b", "a/b"];
    let libraries: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(index, identity)| library(identity, &format!("lib-{index}"), &h))
        .collect();
    let imports: String = libraries
        .iter()
        .enumerate()
        .rev()
        .map(|(index, library)| format!("import {:?} as l{index};\n", library.identity))
        .collect();
    run(format!("{imports}{h}"), libraries);
    let geometry_import = "import \"test/geometry\" as g;\n";
    run(
        format!("{geometry_import}import \"test/a\" as la;\n{h}"),
        vec![
            library("test/geometry", "geometry", &source_literal(fixture, "F")),
            library("test/a", "a", &format!("{geometry_import}{h}")),
        ],
    );
    // Both actual chain_of sizes: limit controls use three, the admitted
    // small-stack fixture uses one thousand. Every reached library is checked.
    for count in [3, 1_000] {
        let libraries = (0..count)
            .map(|index| {
                let body = if index == 0 {
                    h.clone()
                } else {
                    format!("import \"test/c{}\" as l;\n{h}", index - 1)
                };
                library(&format!("test/c{index}"), &format!("c{index}"), &body)
            })
            .collect();
        run(
            format!("import \"test/c{}\" as l;\n{h}", count - 1),
            libraries,
        );
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn replay_importing_and_call_mapping_units_keep_format_identity() {
    use crate::support::front_end::dependency_fixture_identity;
    use qsl_replay::spine::SuppliedLibrary;
    let fixture = include_str!("../../qsl-replay/src/execute/tests.rs");
    let profile = source_literal(fixture, "PROFILE");
    let header = format!("language \"ix:native\" edition \"1-draft\";\n{profile}");
    for constant in ["BIG", "BIG_EDITED"] {
        let library = SuppliedLibrary {
            identity: "test/units".to_owned(),
            source: SourceIdentity::new("agent-ix", "test:units", "git", "r1"),
            path: "test:units".to_owned(),
            bytes: format!("{header}{}", source_literal(fixture, constant)).into_bytes(),
        };
        dependency_fixture_identity(SourceIdentity::new("agent-ix", "test:replay", "git", "r1"), "test:replay",
            &format!("{header}import \"test/units\" as u;\nfunction q using v(x: Int[0, 9]): Boolean pure {{ u::big(x) }}\n"), vec![library]);
    }
    let fixture = include_str!("../../qsl-replay/src/spine/call/tests.rs");
    let source = source_literal(fixture, "FIXTURE_F");
    emitted(
        SourceIdentity::new("agent-ix", "spine-run-fixture", "git", "1"),
        "tc-452-f.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .unwrap();
}

// Read actual authored literals without maintaining another source copy.
fn source_literal(test_source: &str, name: &str) -> String {
    fn find(items: &[syn::Item], name: &str) -> Option<String> {
        items.iter().find_map(|item| {
            if let syn::Item::Mod(item) = item {
                return item
                    .content
                    .as_ref()
                    .and_then(|(_, items)| find(items, name));
            }
            let syn::Item::Const(item) = item else {
                return None;
            };
            if item.ident != name {
                return None;
            }
            let syn::Expr::Lit(literal) = item.expr.as_ref() else {
                panic!("{name} must be an authored source literal");
            };
            let syn::Lit::Str(source) = &literal.lit else {
                panic!("{name} must be UTF-8 source");
            };
            Some(source.value())
        })
    }
    find(&syn::parse_file(test_source).unwrap().items, name)
        .unwrap_or_else(|| panic!("fixture literal {name} exists"))
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn checker_family_authored_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-semantics/src/check/family.rs");
    plain_fixture(&source_literal(fixture, "UNIT"));
    let terms = vec!["1"; 1_000].join(" + ");
    let source = fixture_text(
        function_literal(fixture, "sum_unit", "language "),
        &[("terms", &terms)],
    );
    let limits = qsl_replay::spine::SpineLimits {
        checking: quire_semantic_value::checking::CheckingLimits::new(5_000),
        ..Default::default()
    };
    crate::support::front_end::emitted_with_authority(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits,
        &qsl_replay::spine::LockEvidence::default(),
    )
    .expect("the owning 5,000-node limit admits the original sum");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn call_site_authored_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/call_site.rs");
    for constant in ["UNIT", "PREDICATE_UNIT"] {
        plain_fixture(&source_literal(fixture, constant));
    }
    plain_fixture(&function_literal(
        fixture,
        "tc_166_call_site_locates_each_case_variant_function",
        "language ",
    ));
    plain_fixture(&source_literal(
        include_str!("../../qsl-replay/tests/declared_domain_facade.rs"),
        "UNIT",
    ));
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn compile_facade_authored_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-replay/tests/compile_package_facade.rs");
    let header = source_literal(fixture, "HEADER");
    for constant in ["LIST", "TREE"] {
        plain_fixture(&format!("{header}{}", source_literal(fixture, constant)));
    }
    let document = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let hex = qsl_semantics::model::key::hex(packages.keys().next().unwrap());
    let source = fixture_text(
        function_literal(
            fixture,
            "a_domain_package_is_the_i1_input_as_in_the_spine",
            "{HEADER}model ",
        ),
        &[("HEADER", &header), ("hex", &hex)],
    );
    emitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("the original facade model unit checks");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn admission_corpus_authored_units_keep_format_identity() {
    let fixture = include_str!("../../qsl-package/src/emit/tests/admission_corpus.rs");
    let header = function_literal(
        include_str!("../../qsl-package/src/emit/tests.rs"),
        "package_from_text",
        "language ",
    );
    for constant in ["FORMS_UNIT", "CYCLIC_EQUALITY"] {
        plain_fixture(&fixture_text(
            header.clone(),
            &[("declarations", &source_literal(fixture, constant))],
        ));
    }
    let document = include_bytes!("../fixtures/systems-interface.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let digest = qsl_semantics::model::key::hex(packages.keys().next().unwrap());
    let source = fixture_text(
        function_literal(fixture, "emit", "language "),
        &[("", &digest)],
    );
    emitted(
        SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("the original systems-interface corpus unit checks");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn package_dependent_library_keeps_format_identity() {
    let fixture = include_str!("../../qsl-replay/src/spine/dependency_tests.rs");
    let document = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let digest = qsl_semantics::model::intake::PackageDocument::parse(
        document,
        qsl_foundation::IntakeLimits::default(),
    )
    .unwrap()
    .jcs_digest();
    let model_digest = qsl_semantics::model::key::hex(&digest);
    let body = fixture_text(
        function_literal(
            fixture,
            "an_imported_name_whose_signature_is_package_dependent_refuses",
            "model M =",
        ),
        &[
            ("model_digest", &model_digest),
            ("F", &source_literal(fixture, "F")),
        ],
    );
    let source = format!("{}{body}", source_literal(fixture, "HEADER"));
    emitted(
        SourceIdentity::new("a", "geometry", "git", "1"),
        "geometry.native",
        source.as_bytes(),
        &qsl_semantics::model::intake::package_input([document.as_slice()]),
        &DependencyInput::default(),
    )
    .expect("the reached library checks under its original selected model");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn evaluator_source_call_fixture_keeps_checked_identity() {
    let source = source_literal(
        include_str!("../../qsl-eval/tests/it/source_call.rs"),
        "UNIT",
    );
    emitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the source-call unit checks before and after formatting");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn replay_composite_fixture_keeps_checked_identity() {
    let profile = source_literal(
        include_str!("../../qsl-replay/src/execute/tests.rs"),
        "PROFILE",
    );
    let body = source_literal(
        include_str!("../../qsl-replay/src/execute/tests/composite_parity.rs"),
        "UNIT",
    );
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{profile}{body}");
    emitted(
        SourceIdentity::new("agent-ix", "test:replay", "git", "r1"),
        "test:replay",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the recursive-record, sequence, enum and rational fixture checks");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn wide_integer_fixture_units_keep_checked_identity() {
    // The repository-native TC-909 admitted vectors, including its comment
    // between unary minus and the i128::MIN magnitude.
    let header = source_literal(
        include_str!("../../qsl-replay/src/spine/wide_integer_tests.rs"),
        "HEADER",
    );
    for (parameter, body) in [
        ("Int[0, 18446744073709551615]", "x >= 0"),
        ("Int[0, 9223372036854775808]", "x >= 0"),
        ("Int[0, 18446744073709551616]", "x >= 0"),
        ("Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]", "x <= 170141183460469231731687303715884105727"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= -170141183460469231731687303715884105728"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= - 170141183460469231731687303715884105728"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= -// the minimum\n170141183460469231731687303715884105728"),
        ("Int[-170141183460469231731687303715884105728, 0]", "x >= -170141183460469231731687303715884105727"),
    ] {
        let source = format!("{header}function u using v(x: {parameter}): Boolean pure {{ {body} }}\n");
        emitted(
            SourceIdentity::new("a", "u", "git", "1"), "unit.native", source.as_bytes(),
            &BTreeMap::new(), &DependencyInput::default(),
        ).unwrap_or_else(|refusal| panic!("{body}: the admitted wide-integer fixture checks: {refusal}"));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn checked_file_fixtures_keep_package_identity_and_second_pass_bytes() {
    let document = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    for (name, bytes) in [
        (
            "spine-compile.native",
            include_bytes!("../fixtures/spine-compile.native").as_slice(),
        ),
        (
            "spine-run.native",
            include_bytes!("../fixtures/spine-run.native").as_slice(),
        ),
        (
            "spine-model.native",
            include_bytes!("../fixtures/spine-model.native").as_slice(),
        ),
        (
            "value-format.native",
            include_bytes!("../fixtures/value-format.native").as_slice(),
        ),
    ] {
        emitted(
            SourceIdentity::new("test", name, "fixture", "1"),
            name,
            bytes,
            &packages,
            &DependencyInput::default(),
        )
        .unwrap_or_else(|refusal| panic!("{name}: original must check: {refusal}"));
    }
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn generated_config_version_unit_keeps_checked_identity() {
    let source = crate::support::config_version::spine::unit_text();
    emitted(
        crate::support::config_version::spine::unit_identity(),
        "config-version.native",
        source.as_bytes(),
        &crate::support::config_version::spine::domain_packages(),
        &DependencyInput::default(),
    )
    .expect("ConfigVersion fixture checks before and after formatting");
}

#[trace("TC-885", "FR-003-AC-9")]
#[test]
fn changed_literal_changes_checked_identity() {
    let original = include_str!("../fixtures/spine-compile.native");
    let changed = original.replace("pure { 7 }", "pure { 8 }");
    assert_ne!(
        original, changed,
        "mutation must reach the authored literal"
    );
    let compile = |text: &str| {
        emitted(
            SourceIdentity::new("test", "identity-control", "fixture", "1"),
            "control.native",
            text.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .unwrap()
        .package()
        .package_id()
    };
    assert_ne!(
        compile(original),
        compile(&changed),
        "checked identity must detect a semantic mutation"
    );
}
