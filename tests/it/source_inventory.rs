// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-767: the QSL owner report for Contract IR's source-obligation input.
//! Rows and the stop cause are asserted as typed fields only.

use crate::support::composed_types as setup;

use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use quire_contract_model as ir;
use quire_contract_model::source_obligation::{
    SourceDeclarationHandle, SourceInventoryIdentity, SourceInventoryIncompleteReason,
    SourceInventoryOwner, SourceInventoryScope, SourcePendingObligationKind,
    SourceTypeWorkDimension,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_spec_language::checking::composed::{self, TypeLimits};
use quire_spec_language::checking::source_inventory::qsl_inventory;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding::{self};
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::DeclarationId;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

/// Two claim sites: one guarded application per branch of one node.
const GUARDED: &str =
    "function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }";

fn graph() -> CheckedGraph {
    let text = format!("{HEADER}{GUARDED}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
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
        ir::SourceDocumentId::new("Inventory").expect("a valid document id"),
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
    test: impl FnOnce(&binding::Report<'_>, &composed::TypeReport<'_, '_>),
) {
    let model = setup::model("Primary");
    let sources = [setup::source("main", &model, body)];
    let formal: Vec<FormalSource> = setup::formal_sources(&sources);
    setup::with_binding(&sources, &[&model], limits, |binding| {
        let report = composed::admit_types(binding, &formal, types);
        test(binding, &report);
    });
}

const CALLS: &str =
    "predicate Positive using S (amount: M::Q): Boolean { amount >= rational(0,1) }\n\
    invariant Healthy using S on M::Node at current { Positive(self.fraction) }";

#[test]
fn a_fully_traversed_source_reports_rows_and_no_stop_cause() {
    let graph = graph();
    typed(
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |binding, types| {
            let envelope = qsl_inventory(&selected(), &graph, types);
            assert_eq!(envelope.owner(), SourceInventoryOwner::Qsl);
            assert_eq!(
                envelope.scope(),
                SourceInventoryScope::QslCheckedRequirementsAndPendingObligations
            );
            assert_eq!(envelope.source(), &selected());
            assert!(envelope.stop_reason().is_none());
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
    let graph = graph();
    typed(
        CALLS,
        BindingLimits::default(),
        TypeLimits::default(),
        |_, types| {
            let envelope = qsl_inventory(&selected(), &graph, types);
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
    let graph = graph();
    let body = "predicate Fraction using S (input: M::Q): Boolean { input = rational(2,4) }";
    typed(
        body,
        BindingLimits::default(),
        TypeLimits {
            normalization: 0,
            ..TypeLimits::default()
        },
        |binding, types| {
            let native = types.exhaustion().expect("the budget is exhausted");
            let envelope = qsl_inventory(&selected(), &graph, types);
            let Some(SourceInventoryIncompleteReason::TypeWorkExhausted(reported)) =
                envelope.stop_reason()
            else {
                panic!("exhaustion is not reported as {:?}", envelope.stop_reason());
            };
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
    let graph = graph();
    typed(
        "predicate Pending using S (): Boolean { true }",
        BindingLimits {
            bindings: 0,
            ..BindingLimits::default()
        },
        TypeLimits::default(),
        |binding, types| {
            assert!(types.exhaustion().is_none());
            let envelope = qsl_inventory(&selected(), &graph, types);
            assert_eq!(
                envelope.stop_reason(),
                Some(&SourceInventoryIncompleteReason::TypeAdmissionUnfinished {
                    declaration: SourceDeclarationHandle(id(binding, "Pending").index()),
                })
            );
        },
    );
}
