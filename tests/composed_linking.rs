// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-114: source namespace and native dependency stages through public APIs.
//! Model/profile admission and the real producer boundary remain separate work.

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::{
    admit_namespace, DeclarationDisposition, DeclarationEntry, DeclarationId, DeclarationRefusal,
    DependencyKind, DependencyRefusal, DependencySite, Dimension, ExpectedSource, SourceInventory,
    SyntaxNamespace, WorkLimits,
};
use quire_spec_language::{Code, Limits, Source, SourceIdentity};

const HEADER: &str = r#"language "ix:native" edition "1-draft";
profile S = "test:state-definition" version "1" digest "not-admitted";
"#;

fn identity(authority: &str) -> SourceIdentity {
    SourceIdentity {
        identity: authority.into(),
        revision: "test:revision".into(),
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
        "{}\n{}",
        include_str!("fixtures/composed-choreography.native"),
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
                digest: source.digest(),
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
fn multi_unit_choreography_resolves_forward_native_targets_and_bounded_repetition() {
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
    assert_eq!(namespace.declarations().len(), 6);
    for name in [
        "Orders",
        "CanCharge",
        "ChargedCorrectly",
        "RefundDue",
        "Accept",
        "Spare",
    ] {
        available(namespace, name);
    }
    let references = entry(namespace, "Orders").references();
    assert_eq!(references.len(), 3);
    for (target, kind) in [
        ("CanCharge", DependencyKind::OperationContract),
        ("ChargedCorrectly", DependencyKind::OperationContract),
        ("RefundDue", DependencyKind::TemporalRequirement),
    ] {
        let reference = references.iter().find(|r| r.name.value == target).unwrap();
        assert_eq!(reference.kind, kind);
        assert_eq!(reference.target, Some(id(namespace, target)));
        assert_eq!(
            namespace
                .unit(reference.unit)
                .unwrap()
                .source()
                .slice(reference.name.span),
            Some(target)
        );
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
fn wrong_native_target_kinds_refuse_only_the_dependent_declarations() {
    let wrong = application()
        .replace(
            "requires temporal RefundDue;",
            "requires temporal CanCharge;",
        )
        .replace(
            "contracts [CanCharge, ChargedCorrectly]",
            "contracts [RefundDue, Accept]",
        );
    let sources = [
        source("application", &wrong),
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
    refused(namespace, "Orders");
    let orders = entry(namespace, "Orders");
    let errors = causes(orders);
    assert_eq!(errors.len(), 3);
    for error in errors {
        let DependencyRefusal::WrongTargetKind { reference, target } = error else {
            panic!("wrong target kind required");
        };
        let occurrence = &orders.references()[*reference];
        assert_eq!(occurrence.target, Some(*target));
        assert_eq!(
            namespace.syntax(*target).unwrap().name.value,
            occurrence.name.value
        );
    }
    for name in ["Accept", "RefundDue", "CanCharge", "ChargedCorrectly"] {
        available(namespace, name);
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn an_invariant_is_not_an_operation_contract() {
    let application = application().replace(
        "pre CanCharge using S on M::Payment::charge { Accept(self.allowed) }",
        "invariant CanCharge using S on M::Payment at current { Accept(self.allowed) }",
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
    refused(namespace, "Orders");
    let orders = entry(namespace, "Orders");
    let errors = causes(orders);
    let [DependencyRefusal::WrongTargetKind { reference, target }] = errors.as_slice() else {
        panic!("expected invariant-kind refusal");
    };
    assert_eq!(*target, id(namespace, "CanCharge"));
    assert_eq!(
        orders.references()[*reference].kind,
        DependencyKind::OperationContract
    );
    available(namespace, "CanCharge");
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
        (first.site, second.site)
    else {
        panic!("predicate expression handles");
    };
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
fn calls_in_activation_capture_compensation_and_control_bodies_keep_their_owner() {
    let text = format!(
        "{HEADER}{}",
        r#"
profile T = "test:temporal-definition" version "1" digest "not-admitted";
profile P = "test:protocol-definition" version "1" digest "not-admitted";
model M = "test:model" version "1" digest "not-admitted";
predicate Leaf using S (): Boolean { true }
pre Allowed using S on M::Job::run { Leaf() }
temporal Future using T over (sample: M::Sample) clock "clock"
on each (started: M::Trigger) when (Leaf()) {
  capture accepted: Boolean = Leaf();
  always [0,1] holds(Leaf())
}
protocol Flow using P over (view: M::View)
on each (started: M::Trigger) when (Leaf()) {
  capture accepted: Boolean = Leaf();
  role R on M::Participant;
  channel C from R to R carries M::Message
    ordering fifo by (message: M::Message) { Leaf() } delivery [0,1];
  requires temporal Future;
  compensate Undo for Main::Applied as (forward: M::Effect)
    by R on M::Job::undo using T clock "clock" {
    capture registered: Boolean = Leaf();
    activate first (failed: M::Trigger) when { Leaf() } {
      capture activated: Boolean = Leaf();
    }
    within [0,1]; attempts 2 of M::Attempt;
    retry (older: M::Attempt, newer: M::Attempt) { Leaf() };
    commit never;
    recover (recovered: M::Recovery) { Leaf() };
  }
  run sequence Main {
    attempt Tried by R on M::Job::run contracts [Allowed]
      as (attempted: M::Attempt) { Leaf() };
    effect Applied of Tried as (applied: M::Effect) { Leaf() };
    check Checked using S { Leaf() };
    choice Pick by R visible (Leaf()) {
      case yes when { Leaf() } sequence Yes {}
      case no when { not Leaf() } sequence No {}
    }
    repeat Retry by R visible (Leaf()) max 2 while { Leaf() }
      sequence Again {} exhausted sequence Enough {}
    await Deadline after Applied using T clock "clock" within [0,1]
      match event Done by R as (done: M::Done) { Leaf() };
      then sequence Complete {} timeout sequence Timeout {}
    commit Committed by R as (committed: M::Commit) { Leaf() };
  }
  finish Finished as (finished: M::Finish) { Leaf() };
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
    for (owner, expected_calls) in [("Allowed", 1), ("Future", 3), ("Flow", 19)] {
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
    assert_eq!(entry(namespace, "Flow").references().len(), 21);
}
