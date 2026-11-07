// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-114: source namespace and native dependency stages through public APIs.
//! Model/profile admission and the real producer boundary remain separate work.

use ix_trace_rs::trace;
use qsl_foundation::{Code, Source, SourceIdentity};
use quire_spec_language::linking::composed::{
    admit_namespace, DeclarationDisposition, DeclarationEntry, DeclarationId, DeclarationRefusal,
    DependencyKind, DependencyRefusal, DependencySite, Dimension, ExpectedSource, SourceInventory,
    SyntaxNamespace, WorkLimits,
};
use quire_spec_language::Limits;

const HEADER: &str = r#"language "ix:native" edition "1-draft";
profile S = "test:state-definition";
"#;

fn identity(authority: &str) -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: authority.into(),
    }
}

fn source(authority: &str, text: &str) -> Source {
    Source::read(
        identity(authority),
        format!("{authority}.native"),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap()
}

fn predicates(declarations: &str) -> String {
    format!("{HEADER}{declarations}")
}

fn application() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"quire.state.graph/v1\";\n\
         profile T = \"quire.temporal.timestamped-event.finite-window/v1\";\n\
         model M = \"test:orders-and-refunds\";\n{}",
        r#"
pre CanCharge using S on M::Payment::charge { Accept(self.allowed) }
post ChargedCorrectly using S on M::Payment::charge { Accept(result.ok) }
temporal RefundDue using T over (sample: M::RefundSample)
clock "payment-clock" on origin { eventually [0,5] holds(Accept(sample.ready)) }
"#,
    )
}

fn inventory(sources: &[Source]) -> SourceInventory {
    SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
            })
            .collect(),
    }
}

fn id(namespace: &SyntaxNamespace, name: &str) -> DeclarationId {
    let [id] = namespace.lookup(name) else {
        panic!("expected one declaration named {name}");
    };
    *id
}

fn entry<'a>(namespace: &'a SyntaxNamespace, name: &str) -> &'a DeclarationEntry {
    namespace.declaration(id(namespace, name)).unwrap()
}

fn available(namespace: &SyntaxNamespace, name: &str) {
    assert_eq!(
        namespace.disposition(id(namespace, name)),
        Some(DeclarationDisposition::Available),
        "{name} must finish native name/dependency resolution"
    );
}

fn refused(namespace: &SyntaxNamespace, name: &str) {
    assert_eq!(
        namespace.disposition(id(namespace, name)),
        Some(DeclarationDisposition::Refused)
    );
}

fn causes(entry: &DeclarationEntry) -> Vec<&DependencyRefusal> {
    entry
        .refusals()
        .iter()
        .filter_map(|refusal| {
            if let DeclarationRefusal::Dependency(cause) = refusal {
                Some(cause)
            } else {
                None
            }
        })
        .collect()
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-2")]
fn multi_unit_application_resolves_forward_predicate_calls_across_units() {
    let sources = [
        source("application", &application()),
        source(
            "library",
            &predicates(
                "predicate Accept using S (allowed: Boolean): Boolean { allowed }\n\
             predicate Spare using S (): Boolean { true }",
            ),
        ),
    ];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty());
    assert!(report.exhaustion().is_none());
    let namespace = report.namespace().unwrap();
    assert!(namespace.dependencies_complete());
    assert_eq!(namespace.units().len(), 2);
    assert_eq!(namespace.declarations().len(), 5);
    for name in [
        "CanCharge",
        "ChargedCorrectly",
        "RefundDue",
        "Accept",
        "Spare",
    ] {
        available(namespace, name);
    }
    for name in ["CanCharge", "ChargedCorrectly", "RefundDue"] {
        let [reference] = entry(namespace, name).references() else {
            panic!("expected one call");
        };
        assert_eq!(reference.kind, DependencyKind::PredicateCall);
        assert_eq!(reference.target, Some(id(namespace, "Accept")));
        assert_ne!(reference.unit, entry(namespace, "Accept").unit());
    }
    // Identical alias spelling still refers to each unit's own unadmitted selection.
    assert_ne!(
        namespace.units()[0].profiles()[0].package.value,
        namespace.units()[1].profiles()[0].package.value
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn a_call_to_a_non_predicate_refuses_only_the_caller() {
    let application = application().replace(
        "post ChargedCorrectly using S on M::Payment::charge { Accept(result.ok) }",
        "post ChargedCorrectly using S on M::Payment::charge { CanCharge(result.ok) }",
    );
    let sources = [
        source("application", &application),
        source(
            "library",
            &predicates("predicate Accept using S (allowed: Boolean): Boolean { allowed }"),
        ),
    ];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = report.namespace().unwrap();
    assert!(namespace.dependencies_complete());
    refused(namespace, "ChargedCorrectly");
    let caller = entry(namespace, "ChargedCorrectly");
    let errors = causes(caller);
    let [DependencyRefusal::WrongTargetKind { reference, target }] = errors.as_slice() else {
        panic!("wrong target kind required");
    };
    assert_eq!(*target, id(namespace, "CanCharge"));
    assert_eq!(
        caller.references()[*reference].kind,
        DependencyKind::PredicateCall
    );
    for name in ["Accept", "RefundDue", "CanCharge"] {
        available(namespace, name);
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn a_missing_target_retains_the_original_cause_through_dependents() {
    let sources = [source(
        "chain",
        &predicates(
            "predicate Top using S (): Boolean { Middle() }\n\
         predicate Middle using S (): Boolean { Missing() }\n\
         predicate Spare using S (): Boolean { true }",
        ),
    )];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = report.namespace().unwrap();
    refused(namespace, "Top");
    refused(namespace, "Middle");
    available(namespace, "Spare");
    let middle = entry(namespace, "Middle");
    assert_eq!(
        causes(middle),
        vec![&DependencyRefusal::MissingTarget { reference: 0 }]
    );
    assert_eq!(middle.references()[0].target, None);
    let occurrence = &middle.references()[0];
    assert_eq!(
        namespace
            .unit(occurrence.unit)
            .unwrap()
            .source()
            .slice(occurrence.name.span),
        Some("Missing")
    );
    assert_eq!(
        causes(entry(namespace, "Top")),
        vec![&DependencyRefusal::RefusedTarget {
            reference: 0,
            target: id(namespace, "Middle"),
        }]
    );
}

#[test]
#[trace("TC-114", "TC-126", "FR-036-AC-2", "FR-046-AC-2")]
fn ambiguous_native_names_never_pick_the_first_candidate() {
    let sources = [
        source("first", &predicates("predicate Same using S (): Boolean { true }")),
        source("second", &predicates("predicate Same using S (): Boolean { false }")),
        source("caller", &predicates("predicate Caller using S (): Boolean { Same() }\npredicate Spare using S (): Boolean { true }")),
    ];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = report.namespace().unwrap();
    assert_eq!(namespace.lookup("Same").len(), 2);
    refused(namespace, "Caller");
    assert_eq!(
        causes(entry(namespace, "Caller")),
        vec![&DependencyRefusal::AmbiguousTarget { reference: 0 }]
    );
    assert_eq!(entry(namespace, "Caller").references()[0].target, None);
    available(namespace, "Spare");
}

const CYCLE: &str = "predicate SelfCall using S (): Boolean { SelfCall() }\n\
    predicate Left using S (): Boolean { Right() }\n\
    predicate Right using S (): Boolean { Left() }\n\
    predicate Follower using S (): Boolean { Left() }\n\
    predicate Spare using S (): Boolean { true }";

#[test]
#[trace("TC-114", "TC-126", "FR-036-AC-2", "FR-036-AC-7", "FR-046-AC-2")]
fn self_and_mutual_cycles_refuse_their_dependents_with_exact_edge_loci() {
    let sources = [source("cycles", &predicates(CYCLE))];
    let inventory = inventory(&sources);
    let limits = WorkLimits {
        references: 9,
        dependency_edges: 20,
        ..WorkLimits::default()
    };
    let report = admit_namespace(&inventory, &sources, limits, Limits::default());
    assert!(report.exhaustion().is_none());
    let namespace = report.namespace().unwrap();
    assert!(namespace.dependencies_complete());
    // Five value nodes + four reference attempts; four edges x four graph passes
    // plus four incoming-edge visits propagating the cycle refusals.
    assert_eq!(report.usage().references, 9);
    assert_eq!(report.usage().dependency_edges, 20);
    for (from, to) in [
        ("SelfCall", "SelfCall"),
        ("Left", "Right"),
        ("Right", "Left"),
    ] {
        refused(namespace, from);
        let owner = entry(namespace, from);
        assert!(causes(owner).contains(&&DependencyRefusal::Cycle {
            reference: 0,
            target: id(namespace, to)
        }));
        let reference = &owner.references()[0];
        assert_eq!(
            namespace
                .unit(reference.unit)
                .unwrap()
                .source()
                .slice(reference.name.span),
            Some(to)
        );
    }
    refused(namespace, "Follower");
    assert_eq!(
        causes(entry(namespace, "Follower")),
        vec![&DependencyRefusal::RefusedTarget {
            reference: 0,
            target: id(namespace, "Left"),
        }]
    );
    available(namespace, "Spare");
}

const DIAMOND: &str = "predicate Leaf using S (): Boolean { true }\n\
    predicate Left using S (): Boolean { Leaf() }\n\
    predicate Right using S (): Boolean { Leaf() }\n\
    predicate Top using S (): Boolean { Left() and Right() }";

#[test]
#[trace("TC-114", "FR-036-AC-7")]
fn shared_diamond_uses_the_declared_exact_work_budget_and_fresh_retry() {
    let sources = [source("diamond", &predicates(DIAMOND))];
    let inventory = inventory(&sources);
    let original_inventory = inventory.clone();
    let original_source = sources[0].text().to_owned();
    // Six value nodes + four name-resolution attempts; four edges each inserted
    // and visited by forward DFS, reverse SCC and cycle classification.
    let exact = WorkLimits {
        references: 10,
        dependency_edges: 16,
        ..WorkLimits::default()
    };
    for (dimension, references, edges) in [
        (Dimension::References, 0, 16),
        (Dimension::References, 9, 16),
        (Dimension::DependencyEdges, 10, 0),
        (Dimension::DependencyEdges, 10, 15),
    ] {
        let limits = WorkLimits {
            references,
            dependency_edges: edges,
            ..exact
        };
        let stopped = admit_namespace(&inventory, &sources, limits, Limits::default());
        let exhaustion = *stopped.exhaustion().unwrap();
        assert_eq!(exhaustion.dimension, dimension);
        assert_eq!(exhaustion.code(), Code::ResourceExhausted);
        assert!(stopped.is_incomplete());
        let unfinished = stopped.namespace().unwrap();
        assert!(!unfinished.dependencies_complete());
        assert_eq!(
            unfinished.disposition(id(unfinished, "Top")),
            Some(DeclarationDisposition::Unfinished)
        );
        let successful = admit_namespace(&inventory, &sources, exact, Limits::default());
        assert!(successful.exhaustion().is_none());
        assert_eq!(successful.usage().references, 10);
        assert_eq!(successful.usage().dependency_edges, 16);
        let namespace = successful.namespace().unwrap();
        for name in ["Top", "Left", "Right", "Leaf"] {
            available(namespace, name);
        }
        assert_eq!(stopped.exhaustion(), Some(&exhaustion));
        assert!(!unfinished.dependencies_complete());
    }
    assert_eq!(inventory, original_inventory);
    assert_eq!(sources[0].text(), original_source);
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-7")]
fn repeated_authored_calls_keep_distinct_occurrences_and_charges() {
    let sources = [source(
        "repeated",
        &predicates(
            "predicate Leaf using S (): Boolean { true }\n\
         predicate Top using S (): Boolean { Leaf() and Leaf() }",
        ),
    )];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits {
            references: 6,
            dependency_edges: 8,
            ..WorkLimits::default()
        },
        Limits::default(),
    );
    assert!(report.exhaustion().is_none());
    let namespace = report.namespace().unwrap();
    available(namespace, "Top");
    let [first, second] = entry(namespace, "Top").references() else {
        panic!("two authored calls");
    };
    assert_eq!(first.target, Some(id(namespace, "Leaf")));
    assert_eq!(first.target, second.target);
    assert_ne!(first.name.span, second.name.span);
    assert_ne!(first.site, second.site);
    assert_eq!(report.usage().references, 6);
    assert_eq!(report.usage().dependency_edges, 8);
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn equal_expression_handles_in_different_units_keep_their_original_owners() {
    let sources = [
        source(
            "first",
            &predicates("predicate First using S (): Boolean { Leaf() }"),
        ),
        source(
            "second",
            &predicates("predicate Second using S (): Boolean { Leaf() }"),
        ),
        source(
            "library",
            &predicates("predicate Leaf using S (): Boolean { true }"),
        ),
    ];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = report.namespace().unwrap();
    let first = &entry(namespace, "First").references()[0];
    let second = &entry(namespace, "Second").references()[0];
    let (DependencySite::Expression(first_id), DependencySite::Expression(second_id)) =
        (first.site, second.site);
    assert_eq!(first_id, second_id);
    assert_ne!(first.unit, second.unit);
    assert_eq!(
        namespace.unit(first.unit).unwrap().source().identity(),
        sources[0].identity()
    );
    assert_eq!(
        namespace.unit(second.unit).unwrap().source().identity(),
        sources[1].identity()
    );
    assert_eq!(first.target, second.target);
    available(namespace, "First");
    available(namespace, "Second");
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn unreachable_source_calls_are_still_semantic_dependencies() {
    let sources = [source(
        "unreachable",
        &predicates(
            "predicate Recursive using S (): Boolean { if false then Recursive() else true }",
        ),
    )];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = report.namespace().unwrap();
    refused(namespace, "Recursive");
    assert!(
        causes(entry(namespace, "Recursive")).contains(&&DependencyRefusal::Cycle {
            reference: 0,
            target: id(namespace, "Recursive"),
        })
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-2")]
fn calls_in_activation_and_capture_bodies_keep_their_owner() {
    let text = format!(
        "{HEADER}{}",
        r#"
profile T = "test:temporal-definition";
model M = "test:model";
predicate Leaf using S (): Boolean { true }
pre Allowed using S on M::Job::run { Leaf() }
temporal Future using T over (sample: M::Sample) clock "clock"
on each (started: M::Trigger) when (Leaf()) {
  capture accepted: Boolean = Leaf();
  always [0,1] holds(Leaf())
}
"#
    );
    let sources = [source("scopes", &text)];
    let inventory = inventory(&sources);
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    let namespace = report.namespace().unwrap();
    for (owner, expected_calls) in [("Allowed", 1), ("Future", 3)] {
        available(namespace, owner);
        let calls: Vec<_> = entry(namespace, owner)
            .references()
            .iter()
            .filter(|reference| reference.kind == DependencyKind::PredicateCall)
            .collect();
        assert_eq!(calls.len(), expected_calls, "all calls in {owner}");
        let declaration = namespace.syntax(id(namespace, owner)).unwrap();
        for call in calls {
            assert_eq!(call.target, Some(id(namespace, "Leaf")));
            assert!(call.name.span.start >= declaration.span.start);
            assert!(call.name.span.end <= declaration.span.end);
            assert_eq!(
                namespace
                    .unit(call.unit)
                    .unwrap()
                    .source()
                    .slice(call.name.span),
                Some("Leaf")
            );
        }
    }
}

/// FR-001: two sources that differ only in their authority label are two
/// distinct sources, each matched to its own inventory entry.
#[test]
#[trace("TC-424", "FR-001-AC-5")]
fn sources_differing_only_in_authority_are_distinct() {
    let read = |authority: &str, text: &str| {
        Source::read(
            SourceIdentity {
                authority: authority.into(),
                ..identity("shared")
            },
            format!("{authority}.native"),
            text.as_bytes(),
            Limits::default().source_bytes,
        )
        .unwrap()
    };
    let sources = [
        read(
            "first",
            &predicates("predicate One using S (): Boolean { true }"),
        ),
        read(
            "second",
            &predicates("predicate Two using S (): Boolean { true }"),
        ),
    ];
    let inventory = SourceInventory {
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().authority.clone(),
                identity: source.identity().clone(),
            })
            .collect(),
        ..inventory(&sources)
    };
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    assert_eq!(report.namespace().unwrap().units().len(), 2);
}
