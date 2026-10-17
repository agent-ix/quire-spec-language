// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-767: the QSL owner report for Contract IR's source-obligation input.
//! Rows and the stop cause are asserted as typed fields only.

use crate::support::composed_types as setup;

use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use quire_contract_model as ir;
use quire_contract_model::source_obligation::{
    SourceDeclarationHandle, SourceInventoryCompleteness, SourceInventoryIdentity,
    SourceInventoryIncompleteReason, SourceInventoryOwner, SourceInventoryScope,
    SourcePendingObligationKind, SourceTypeWorkDimension,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_spec_language::checking::composed::{self, TypeLimits};
use quire_spec_language::checking::source_inventory::{
    qsl_inventory, CheckedUnit, UnitLabel, UnitLabelMismatch,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding::{self};
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::DeclarationId;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

/// Two claim sites: one guarded application per branch of one node.
const GUARDED: &str =
    "function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }";

/// The labels `setup::source("main", ..)` gives its unit.
fn matching_graph() -> CheckedGraph {
    graph(SourceIdentity::new("test", "unit:main", "test", "authored"))
}

fn graph(identity: SourceIdentity) -> CheckedGraph {
    let text = format!("{HEADER}{GUARDED}\n");
    let parsed = qsl_cst::parse(
        identity,
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .expect("the fixture checks")
}

fn selected() -> ir::SourceIdentity {
    ir::SourceIdentity::new(
        ir::SourceDocumentId::new("NativeTypes0").expect("a valid document id"),
        ir::SourceRevision::new(1).expect("a valid revision"),
    )
}

fn id(binding: &binding::Report<'_>, name: &str) -> DeclarationId {
    let [id] = binding.namespace().lookup(name) else {
        panic!("one authored declaration named {name}")
    };
    *id
}

fn typed(
    body: &str,
    limits: BindingLimits,
    types: TypeLimits,
    test: impl FnOnce(&binding::Report<'_>, &FormalSource, &composed::TypeReport<'_, '_>),
) {
    typed_named("main", body, limits, types, test);
}

fn typed_named(
    name: &str,
    body: &str,
    limits: BindingLimits,
    types: TypeLimits,
    test: impl FnOnce(&binding::Report<'_>, &FormalSource, &composed::TypeReport<'_, '_>),
) {
    let model = setup::model("Primary");
    let sources = [setup::source(name, &model, body)];
    let formal: Vec<FormalSource> = setup::formal_sources(&sources);
    setup::with_binding(&sources, &[&model], limits, |binding| {
        let report = composed::admit_types(binding, &formal, types);
        test(binding, &formal[0], &report);
    });
}

const CALLS: &str =
    "predicate Positive using S (amount: M::Q): Boolean { amount >= rational(0,1) }\n\
    invariant Healthy using S on M::Node at current { Positive(self.fraction) }";

#[test]
fn a_fully_traversed_source_reports_rows_and_no_stop_cause() {
    let graph = matching_graph();
    typed(
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |binding, formal, types| {
            let unit = CheckedUnit::new(formal, &graph, types).expect("the labels match");
            let envelope = qsl_inventory(&unit).expect("a consistent enumeration");
            assert_eq!(envelope.owner(), SourceInventoryOwner::Qsl);
            assert_eq!(
                envelope.scope(),
                SourceInventoryScope::QslCheckedRequirementsAndPendingObligations
            );
            assert_eq!(envelope.source(), &selected());
            assert_eq!(
                envelope.completeness(),
                &SourceInventoryCompleteness::Complete
            );
            assert_eq!(envelope.observed_count(), envelope.items().len());
            let positive = id(binding, "Positive");
            let totals = envelope
                .items()
                .iter()
                .filter(|row| {
                    matches!(
                        row.identity(),
                        SourceInventoryIdentity::PendingObligation(pending)
                            if pending.kind == SourcePendingObligationKind::PredicateTotal {
                                target: SourceDeclarationHandle(positive.index()),
                            }
                    )
                })
                .count();
            assert_eq!(totals, 1);
        },
    );
}

#[test]
fn requirement_and_pending_sets_stay_separate_identity_kinds() {
    let graph = matching_graph();
    typed(
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |_, formal, types| {
            let unit = CheckedUnit::new(formal, &graph, types).expect("the labels match");
            let envelope = qsl_inventory(&unit).expect("a consistent enumeration");
            let requirements: Vec<_> = envelope
                .items()
                .iter()
                .filter_map(|row| match row.identity() {
                    SourceInventoryIdentity::RequirementRecord(record) => Some(record),
                    _ => None,
                })
                .collect();
            let pending = envelope
                .items()
                .iter()
                .filter(|row| {
                    matches!(
                        row.identity(),
                        SourceInventoryIdentity::PendingObligation(_)
                    )
                })
                .count();
            let expected: Vec<_> = graph
                .requirements()
                .keys()
                .map(|key| {
                    (
                        *key.node().as_bytes(),
                        key.origin().role().as_str().to_owned(),
                        key.origin().ordinal(),
                    )
                })
                .collect();
            assert!(expected.len() >= 2, "both guarded occurrences are records");
            let reported: Vec<_> = requirements
                .iter()
                .map(|record| (record.node, record.role.to_string(), record.ordinal))
                .collect();
            assert_eq!(reported, expected);
            let obligations: usize = types
                .declarations()
                .iter()
                .map(|declaration| declaration.obligations().len())
                .sum();
            assert!(obligations > 0);
            assert_eq!(pending, obligations);
            assert_eq!(envelope.items().len(), requirements.len() + pending);
        },
    );
}

#[test]
fn exhausted_type_work_keeps_its_native_fields() {
    let graph = matching_graph();
    let body = "predicate Fraction using S (input: M::Q): Boolean { input = rational(2,4) }";
    typed(
        body,
        BindingLimits::default(),
        TypeLimits {
            normalization: 0,
            ..TypeLimits::default()
        },
        |binding, formal, types| {
            let native = types.exhaustion().expect("the budget is exhausted");
            let unit = CheckedUnit::new(formal, &graph, types).expect("the labels match");
            let envelope = qsl_inventory(&unit).expect("a consistent enumeration");
            let SourceInventoryCompleteness::Incomplete(
                SourceInventoryIncompleteReason::TypeWorkExhausted(reported),
            ) = envelope.completeness()
            else {
                panic!(
                    "exhaustion is not reported as {:?}",
                    envelope.completeness()
                );
            };
            assert_eq!(envelope.items().len(), envelope.observed_count());
            assert!(envelope.items().len() >= graph.requirements().len());
            assert_eq!(reported.dimension, SourceTypeWorkDimension::Normalization);
            assert_eq!(
                (reported.limit, reported.prior, reported.requested),
                (native.limit, native.prior, native.requested)
            );
            assert_eq!(
                reported.site.declaration,
                SourceDeclarationHandle(id(binding, "Fraction").index())
            );
            assert_eq!(
                (reported.site.start, reported.site.end),
                (native.site.span.start, native.site.span.end)
            );
        },
    );
}

#[test]
fn an_unfinished_declaration_is_distinct_from_exhaustion() {
    let graph = matching_graph();
    typed(
        "predicate Pending using S (): Boolean { true }",
        BindingLimits {
            bindings: 0,
            ..BindingLimits::default()
        },
        TypeLimits::default(),
        |binding, formal, types| {
            assert!(types.exhaustion().is_none());
            let unit = CheckedUnit::new(formal, &graph, types).expect("the labels match");
            let envelope = qsl_inventory(&unit).expect("a consistent enumeration");
            assert_eq!(
                envelope.completeness(),
                &SourceInventoryCompleteness::Incomplete(
                    SourceInventoryIncompleteReason::TypeAdmissionUnfinished {
                        declaration: SourceDeclarationHandle(id(binding, "Pending").index()),
                    }
                )
            );
        },
    );
}

fn label(authority: &str, identity: &str) -> UnitLabel {
    UnitLabel {
        authority: authority.into(),
        identity: identity.into(),
    }
}

#[test]
fn a_graph_of_another_source_is_refused_with_both_labels() {
    let graph = graph(SourceIdentity::new("a", "u", "git", "1"));
    typed(
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |_, formal, types| {
            let refused = CheckedUnit::new(formal, &graph, types).expect_err("another source");
            assert_eq!(
                refused,
                UnitLabelMismatch::Graph {
                    selected: label("test", "unit:main"),
                    graph: label("a", "u"),
                }
            );
        },
    );
}

#[test]
fn a_type_report_of_another_source_is_refused_with_both_labels() {
    let graph = matching_graph();
    typed_named(
        "other",
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |_, formal, types| {
            let refused = CheckedUnit::new(formal, &graph, types).expect_err("another source");
            assert_eq!(
                refused,
                UnitLabelMismatch::Graph {
                    selected: label("test", "unit:other"),
                    graph: label("test", "unit:main"),
                }
            );
        },
    );
}

#[test]
fn a_formal_source_that_is_not_the_reports_unit_is_refused() {
    let graph = matching_graph();
    let model = setup::model("Primary");
    let foreign = setup::formal_sources(&[setup::source("foreign", &model, CALLS)]);
    typed(
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |_, _, types| {
            let refused = CheckedUnit::new(&foreign[0], &graph, types).expect_err("foreign");
            assert_eq!(
                refused,
                UnitLabelMismatch::TypeReport {
                    selected: label("test", "unit:foreign"),
                    unit: label("test", "unit:main"),
                }
            );
        },
    );
}

#[test]
fn a_report_over_two_units_is_not_one_checked_unit() {
    let graph = matching_graph();
    let model = setup::model("Primary");
    let sources = [
        setup::source("main", &model, CALLS),
        setup::source(
            "second",
            &model,
            "predicate Other using S (): Boolean { true }",
        ),
    ];
    let formal = setup::formal_sources(&sources);
    setup::with_binding(&sources, &[&model], BindingLimits::default(), |binding| {
        let types = composed::admit_types(binding, &formal, TypeLimits::default());
        let refused = CheckedUnit::new(&formal[0], &graph, &types).expect_err("two units");
        assert_eq!(refused, UnitLabelMismatch::NotOneUnit { count: 2 });
    });
}
