//! FR-219 and FR-220 at S3: memory defaults, weak joins, fences.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, AccessOrdering, MemoryModelName};
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

use crate::check::{
    AccessClass, CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, ProtocolAnchorCause,
};
use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, ValueTypeRef,
};
use crate::model::intake::SelectedModel;
use crate::model::key::DeclarationKey;
use crate::model::normalize::{normalize, NormalizeOutcome};

#[allow(dead_code)]
mod attempt_fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_attempt.rs");
}
use attempt_fixture::{m_actor_model, HEADER};

fn attempt(name: &str) -> String {
    format!(
        "attempt {name} by Seller on M::Actor::op contracts [] as (b{name}: Boolean) {{ true }};"
    )
}

/// A `parallel` named `name`, with the `memory` clause and join given, whose
/// first branch holds `inner` after an attempt.
fn parallel(name: &str, memory: &str, join: &str, inner: &str) -> String {
    format!(
        "parallel {name} {memory} {{ \
         branch left sequence L{name} {{ {} {inner} }} \
         branch right sequence R{name} {{ }} }} join {join};",
        attempt(&format!("A{name}"))
    )
}

fn protocol(run: &str) -> String {
    format!(
        "protocol Flow using v over (input: Boolean) on origin {{\n\
         role Seller on M::Actor;\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

fn assemble(declarations: &str) -> (String, PackageDeclarations) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![m_actor_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{declarations}: the assembler refuses: {refusal:?}"));
    (text, assembled)
}

fn checks(declarations: &str) -> CheckedGraph {
    let (_, assembled) = assemble(declarations);
    assembled
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// The one refusal `declarations` checks to: its source text, cause, code
/// and tag.
fn only_refusal(declarations: &str) -> (String, ProtocolAnchorCause, &'static str, String) {
    let (text, assembled) = assemble(declarations);
    let found: Vec<CheckRefusal> = match assembled.check(CheckingLimits::default()) {
        Err(refusals) => refusals,
        Ok(_) => panic!("{declarations}: checking refuses"),
    };
    let [refusal] = &found[..] else {
        panic!("{declarations}: one refusal, got {found:?}");
    };
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a ProtocolAnchor cause, got {:?}", refusal.cause);
    };
    (
        text,
        cause.as_ref().clone(),
        refusal.cause.code().as_str(),
        refusal.cause.cause().unwrap_or_default().to_owned(),
    )
}

fn located<'t>(text: &'t str, cause: &ProtocolAnchorCause) -> &'t str {
    let span = cause.span();
    &text[span.start..span.end]
}

fn models(graph: &CheckedGraph) -> Vec<MemoryModelName> {
    graph
        .protocol("Flow")
        .expect("the checked protocol")
        .memory
        .parallels
        .iter()
        .map(|parallel| parallel.memory)
        .collect()
}

/// A `parallel` with no `memory` clause records `Sc` and one with `memory
/// tso` records `Tso`; a nested `parallel` with no clause or with the
/// outermost default checks, and another default refuses
/// `invalid_model_binding`/`conflicting-binding` at the nested clause,
/// naming both controls.
#[trace("TC-664", "FR-219-AC-1")]
#[test]
fn a_memory_default_is_recorded_and_a_nested_one_must_match_the_outermost() {
    let plain = protocol(&parallel("Both", "", "all [left, right]", ""));
    assert_eq!(models(&checks(&plain)), [MemoryModelName::Sc]);
    let weak = protocol(&parallel("Both", "memory tso", "all [left, right]", ""));
    assert_eq!(models(&checks(&weak)), [MemoryModelName::Tso]);

    let nested = |outer: &str, inner: &str| {
        protocol(&parallel(
            "Both",
            outer,
            "all [left, right]",
            &parallel("Inner", inner, "all [left, right]", ""),
        ))
    };
    let silent = nested("memory tso", "");
    assert_eq!(
        models(&checks(&silent)),
        [MemoryModelName::Tso, MemoryModelName::Sc]
    );
    let same = nested("memory tso", "memory tso");
    assert_eq!(
        models(&checks(&same)),
        [MemoryModelName::Tso, MemoryModelName::Tso]
    );

    for (outer, inner, outer_model, inner_model) in [
        (
            "memory tso",
            "memory ra",
            MemoryModelName::Tso,
            MemoryModelName::Ra,
        ),
        (
            "memory tso",
            "memory sc",
            MemoryModelName::Tso,
            MemoryModelName::Sc,
        ),
        ("", "memory tso", MemoryModelName::Sc, MemoryModelName::Tso),
    ] {
        let (text, cause, code, tag) = only_refusal(&nested(outer, inner));
        assert_eq!(
            (code, tag.as_str()),
            ("invalid_model_binding", "conflicting-binding")
        );
        let ProtocolAnchorCause::MemoryConflict {
            outermost,
            outermost_model: found_outer,
            nested,
            nested_model: found_inner,
            ..
        } = &cause
        else {
            panic!("a MemoryConflict cause, got {cause:?}");
        };
        assert_eq!(
            (
                outermost.as_str(),
                *found_outer,
                nested.as_str(),
                *found_inner
            ),
            ("Both", outer_model, "Inner", inner_model)
        );
        assert_eq!(located(&text, &cause), inner);
    }
}

/// A weak default on a `parallel` joined `any`, `quorum`, or `all` with an
/// `outstanding` clause refuses `unsupported_construct`/`declaration-form`
/// at the join, naming the policy and the weak default; `sc` and a weak
/// `join all` with no `outstanding` clause check.
#[trace("TC-664", "FR-219-AC-1")]
#[test]
fn a_weak_default_requires_join_all_without_outstanding() {
    checks(&protocol(&parallel(
        "Both",
        "memory ra",
        "all [left, right]",
        "",
    )));
    checks(&protocol(&parallel("Both", "memory sc", "any [left]", "")));
    checks(&protocol(&parallel("Both", "", "any [left]", "")));

    for (memory, join, model, policy) in [
        ("memory ra", "any [left]", MemoryModelName::Ra, "any"),
        (
            "memory tso",
            "quorum(1) [left, right]",
            MemoryModelName::Tso,
            "quorum",
        ),
        (
            "memory tso",
            "all [left, right] outstanding continue",
            MemoryModelName::Tso,
            "all outstanding",
        ),
    ] {
        let (text, cause, code, tag) = only_refusal(&protocol(&parallel("Both", memory, join, "")));
        assert_eq!(
            (code, tag.as_str()),
            ("unsupported_construct", "declaration-form")
        );
        let ProtocolAnchorCause::WeakJoin {
            parallel: name,
            model: found_model,
            policy: found_policy,
            ..
        } = &cause
        else {
            panic!("a WeakJoin cause, got {cause:?}");
        };
        assert_eq!(
            (name.as_str(), *found_model, found_policy.as_str()),
            ("Both", model, policy)
        );
        assert!(
            join.starts_with(located(&text, &cause)),
            "{join} locates {:?}",
            located(&text, &cause)
        );
    }
}

/// A `fence` records its ordering, whichever of the four it is written
/// with.
#[trace("TC-665", "FR-220-AC-2")]
#[test]
fn each_fence_ordering_is_recorded() {
    let orderings = [
        ("acquire", AccessOrdering::Acquire),
        ("release", AccessOrdering::Release),
        ("acq_rel", AccessOrdering::AcqRel),
        ("seq_cst", AccessOrdering::SeqCst),
    ];
    for (written, expected) in orderings {
        let graph = checks(&protocol(&format!(
            "sequence Main {{ fence Barrier ordering {written}; }}"
        )));
        let memory = &graph.protocol("Flow").expect("checked").memory;
        let [fence] = &memory.fences[..] else {
            panic!("one fence: {memory:?}");
        };
        assert_eq!(fence.ordering, expected, "{written}");
    }
}

// ---------------------------------------------------------------------------
// FR-220: access classification, orderings and shape over a store-buffering
// model (ADR-025 section 10)
// ---------------------------------------------------------------------------

const PACKAGE: &str = "example/protocol-fixture";

fn key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: format!("ix://{PACKAGE}/{name}"),
    }
}

fn boolean_field(owner: &str, name: &str) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type: ValueTypeRef::Native(NativeValueType::Boolean),
        multiplicity: Multiplicity {
            lower: 1,
            upper: Some(1),
            ordered: false,
            unique: true,
        },
        presence: quire_exact::Presence::Required,
        subsets: Vec::new(),
        redefines: None,
    })
}

/// An operation of `owner` writing the `receiver_modifies` fields of its
/// receiver, over the reference parameters `parameters` (name, type).
fn operation(
    owner: &str,
    name: &str,
    parameters: &[(&str, &str)],
    receiver_modifies: &[&str],
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        parameters: parameters
            .iter()
            .map(|(parameter, type_name)| OperationParameterRecord {
                key: key(&format!("{owner}/{name}/{parameter}")),
                value_type: ValueTypeRef::Package(key(type_name)),
                multiplicity: Multiplicity {
                    lower: 1,
                    upper: Some(1),
                    ordered: false,
                    unique: true,
                },
            })
            .collect(),
        result: None,
        effect: OperationEffect {
            receiver_modifies: receiver_modifies
                .iter()
                .map(|field| key(&format!("{owner}/{field}")))
                .collect(),
            ..OperationEffect::default()
        },
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: false,
        redefines: None,
    })
}

/// `Cell { v, w }` and `Core { r }`: `Cell::set` stores `v`, `Cell::flip`
/// reads and writes it, `Cell::both` reads and writes `v` and `w`,
/// `Cell::touch` writes nothing, and `Core::read(src: Cell)` reads `src.v`
/// into the receiver's register `r`.
fn store_buffering_model() -> SelectedModel {
    let object = |name: &str| {
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key(name),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        })
    };
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: PACKAGE.to_owned(),
            version: "1".to_owned(),
            digest: [0_u8; 32],
        },
        vec![
            object("Actor"),
            object("Cell"),
            object("Core"),
            boolean_field("Cell", "v"),
            boolean_field("Cell", "w"),
            boolean_field("Core", "r"),
            operation("Actor", "op", &[], &[]),
            operation("Cell", "set", &[], &["v"]),
            operation("Cell", "flip", &[], &["v"]),
            operation("Cell", "both", &[], &["v", "w"]),
            operation("Cell", "touch", &[], &[]),
            operation("Core", "read", &[("src", "Cell")], &["r"]),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the store-buffering model normalizes");
    };
    SelectedModel {
        alias: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    }
}

const SB_CLAUSES: &str = "pre ReadSrc using v on M::Core::read { src.v }\n\
    post Flip using v on M::Cell::flip { self.v = not pre(self.v) }\n\
    post Both using v on M::Cell::both { self.v = pre(self.w) and self.w = pre(self.v) }\n";

/// A protocol of one `parallel` over `left` and `right`, each a sequence of
/// the given `(name, role, operation, ordering)` attempts.
fn litmus(
    memory: &str,
    left: &[(&str, &str, &str, &str)],
    right: &[(&str, &str, &str, &str)],
) -> String {
    let branch = |attempts: &[(&str, &str, &str, &str)]| {
        attempts
            .iter()
            .map(|(name, role, operation, ordering)| {
                format!(
                    "attempt {name} by {role} on {operation} {ordering} contracts [] \
                     as (b{name}: Boolean) {{ true }};"
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!(
        "{SB_CLAUSES}protocol Flow using v over (input: Boolean) on origin {{\n\
         role X on M::Cell;\n\
         role C1 on M::Core;\n\
         role C2 on M::Core;\n\
         run parallel Both {memory} {{ \
         branch left sequence L {{ {} }} \
         branch right sequence R {{ {} }} }} join all [left, right];\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}",
        branch(left),
        branch(right)
    )
}

/// The checked classes of the attempts of `source`, by attempt name.
fn classes(source: &str) -> Vec<(String, AccessClass, Option<AccessOrdering>)> {
    let (_, assembled) = assemble_model(source, store_buffering_model());
    let graph = assembled
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{source}: {refusals:?}"));
    let checked = graph.protocol("Flow").expect("the checked protocol");
    checked
        .memory
        .accesses
        .iter()
        .map(|access| {
            let control = checked.controls.control(access.node).expect("in the tree");
            (control.name.clone(), access.class, access.ordering)
        })
        .collect()
}

fn assemble_model(declarations: &str, model: SelectedModel) -> (String, PackageDeclarations) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![model],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{declarations}: the assembler refuses: {refusal:?}"));
    (text, assembled)
}

fn refused(source: &str) -> (String, Vec<ProtocolAnchorCause>) {
    let (text, assembled) = assemble_model(source, store_buffering_model());
    let Err(found) = assembled.check(CheckingLimits::default()) else {
        panic!("{source}: checking refuses");
    };
    let causes = found
        .iter()
        .map(|refusal| {
            assert_eq!(
                (refusal.cause.code().as_str(), refusal.cause.cause()),
                match &refusal.cause {
                    CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                        ProtocolAnchorCause::AccessOrdering { .. } =>
                            ("ill_typed", Some("operator-ineligible")),
                        ProtocolAnchorCause::AccessShape { .. } =>
                            ("unsupported_construct", Some("declaration-form")),
                        other => panic!("an access cause, got {other:?}"),
                    },
                    other => panic!("a ProtocolAnchor cause, got {other:?}"),
                }
            );
            match &refusal.cause {
                CheckCause::ProtocolAnchor(cause) => cause.as_ref().clone(),
                _ => unreachable!(),
            }
        })
        .collect();
    (text, causes)
}

const STORE: &str = "M::Cell::set";
const LOAD: &str = "M::Core::read";

/// ADR-025 section 10's `SB`: each `set` is a store, each `read` a load of
/// the other thread's cell, whose own register is branch-local; without
/// orderings the accesses are the same kinds, non-atomic; `flip` is a
/// read-modify-write.
#[trace("TC-665", "TC-664")]
#[test]
fn store_buffering_classifies_stores_loads_and_read_modify_writes() {
    let sb_in = |memory: &str, store_order: &str, load_order: &str| {
        litmus(
            memory,
            &[
                ("Sx", "X", STORE, store_order),
                ("Ly", "C1", LOAD, load_order),
            ],
            &[
                ("Sy", "X", STORE, store_order),
                ("Lx", "C2", LOAD, load_order),
            ],
        )
    };
    let sb = |store_order: &str, load_order: &str| sb_in("", store_order, load_order);
    let sb_with = |memory: &str| sb_in(memory, "ordering release", "ordering acquire");
    let atomic = classes(&sb("ordering release", "ordering acquire"));
    assert_eq!(
        atomic,
        [
            (
                "Sx".to_owned(),
                AccessClass::Store,
                Some(AccessOrdering::Release)
            ),
            (
                "Ly".to_owned(),
                AccessClass::Load,
                Some(AccessOrdering::Acquire)
            ),
            (
                "Sy".to_owned(),
                AccessClass::Store,
                Some(AccessOrdering::Release)
            ),
            (
                "Lx".to_owned(),
                AccessClass::Load,
                Some(AccessOrdering::Acquire)
            ),
        ]
    );
    let plain = classes(&sb("", ""));
    assert!(plain.iter().all(|(_, _, ordering)| ordering.is_none()));
    assert_eq!(
        plain.iter().map(|(_, class, _)| *class).collect::<Vec<_>>(),
        [
            AccessClass::Store,
            AccessClass::Load,
            AccessClass::Store,
            AccessClass::Load
        ]
    );

    // The same protocol checks under each weak default: every access is one
    // location (FR-219-AC-1).
    for (memory, model) in [
        ("memory tso", MemoryModelName::Tso),
        ("memory ra", MemoryModelName::Ra),
    ] {
        let weak = classes(&sb_with(memory));
        assert_eq!(weak.len(), 4, "{memory}");
        let (_, assembled) = assemble_model(&sb_with(memory), store_buffering_model());
        let graph = assembled.check(CheckingLimits::default()).expect("checks");
        assert_eq!(
            graph.protocol("Flow").expect("checked").memory.parallels[0].memory,
            model
        );
    }

    let rmw = classes(&litmus(
        "",
        &[("Fa", "X", "M::Cell::flip", "ordering acq_rel")],
        &[("Fb", "X", "M::Cell::flip", "ordering acq_rel")],
    ));
    assert!(rmw
        .iter()
        .all(|(_, class, ordering)| *class == AccessClass::Rmw
            && *ordering == Some(AccessOrdering::AcqRel)));
}

/// A load with `release`, a store with `acquire`, a read-modify-write with
/// no ordering and a local attempt with `relaxed` each refuse
/// `ill_typed`/`operator-ineligible`, naming the attempt; the orderings each
/// kind takes check.
#[trace("TC-665", "FR-220-AC-2")]
#[test]
fn an_ordering_its_access_kind_does_not_take_refuses() {
    let pair = |left: (&str, &str, &str, &str), right: (&str, &str, &str, &str)| {
        litmus("", &[left], &[right])
    };
    // `Sx` stores a cell `Ly` loads, so both are shared.
    let cases = [
        (
            pair(
                ("Sx", "X", STORE, ""),
                ("Ly", "C1", LOAD, "ordering release"),
            ),
            ("Ly", "release"),
            AccessClass::Load,
            Some(AccessOrdering::Release),
        ),
        (
            pair(
                ("Sx", "X", STORE, "ordering acquire"),
                ("Ly", "C1", LOAD, ""),
            ),
            ("Sx", "acquire"),
            AccessClass::Store,
            Some(AccessOrdering::Acquire),
        ),
        (
            pair(
                ("Fa", "X", "M::Cell::flip", ""),
                ("Fb", "X", "M::Cell::flip", "ordering seq_cst"),
            ),
            ("Fa", "Fa"),
            AccessClass::Rmw,
            None,
        ),
        (
            pair(
                ("Sx", "X", STORE, ""),
                ("Ta", "X", "M::Cell::touch", "ordering relaxed"),
            ),
            ("Ta", "relaxed"),
            AccessClass::Local,
            Some(AccessOrdering::Relaxed),
        ),
    ];
    for (source, (name, at), class, ordering) in cases {
        let (text, causes) = refused(&source);
        let [ProtocolAnchorCause::AccessOrdering {
            attempt,
            class: found_class,
            ordering: found_ordering,
            span,
        }] = &causes[..]
        else {
            panic!("one AccessOrdering cause: {causes:?}");
        };
        assert_eq!(
            (attempt.as_str(), *found_class, *found_ordering),
            (name, class, ordering)
        );
        assert_eq!(&text[span.start..span.end], at);
    }

    for (store, load) in [
        ("ordering relaxed", "ordering relaxed"),
        ("ordering release", "ordering acquire"),
        ("ordering seq_cst", "ordering seq_cst"),
        ("", ""),
    ] {
        classes(&litmus(
            "",
            &[("Sx", "X", STORE, store)],
            &[("Ly", "C1", LOAD, load)],
        ));
    }
}

/// An attempt that accesses two shared fields of its receiver refuses
/// `unsupported_construct`/`declaration-form` naming both locations under a
/// weak default, and checks under `sc`, classified as a read-modify-write.
#[trace("TC-665")]
#[test]
fn a_two_field_access_refuses_under_a_weak_default_and_checks_under_sc() {
    let both = |memory: &str| {
        litmus(
            memory,
            &[("Ba", "X", "M::Cell::both", "ordering seq_cst")],
            &[("Bb", "X", "M::Cell::both", "ordering seq_cst")],
        )
    };
    let under_sc = classes(&both(""));
    assert!(under_sc
        .iter()
        .all(|(_, class, _)| *class == AccessClass::Rmw));
    classes(&both("memory sc"));

    let (_, causes) = refused(&both("memory tso"));
    assert_eq!(causes.len(), 2, "{causes:?}");
    for (cause, name) in causes.iter().zip(["Ba", "Bb"]) {
        let ProtocolAnchorCause::AccessShape {
            attempt,
            model,
            locations,
            ..
        } = cause
        else {
            panic!("an AccessShape cause, got {cause:?}");
        };
        assert_eq!((attempt.as_str(), *model), (name, MemoryModelName::Tso));
        assert_eq!(
            locations,
            &[
                format!("ix://{PACKAGE}/Cell/v"),
                format!("ix://{PACKAGE}/Cell/w")
            ]
        );
    }
}

/// MA-1 shares nothing outside a `parallel`, so an attempt there is local
/// and an `ordering` on it refuses `ill_typed`/`operator-ineligible` at the
/// ordering; the same attempt without one checks.
#[trace("TC-665")]
#[test]
fn an_ordering_on_an_attempt_outside_every_parallel_refuses() {
    let run = |ordering: &str| {
        protocol(&format!(
            "sequence Main {{ attempt Lone by Seller on M::Actor::op {ordering} contracts [] \
             as (lone: Boolean) {{ true }}; }}"
        ))
    };
    checks(&run(""));
    let (text, cause, code, tag) = only_refusal(&run("ordering seq_cst"));
    assert_eq!((code, tag.as_str()), ("ill_typed", "operator-ineligible"));
    let ProtocolAnchorCause::AccessOrdering { class, .. } = &cause else {
        panic!("an AccessOrdering cause, got {cause:?}");
    };
    assert_eq!(*class, AccessClass::Local);
    assert_eq!(located(&text, &cause), "seq_cst");
}
