//! FR-219 and FR-220 at S3: memory defaults, weak joins, fences.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, AccessOrdering, MemoryModelName};
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, ProtocolAnchorCause};

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
        (
            "",
            "memory tso",
            MemoryModelName::Sc,
            MemoryModelName::Tso,
        ),
    ] {
        let (text, cause, code, tag) = only_refusal(&nested(outer, inner));
        assert_eq!((code, tag.as_str()), ("invalid_model_binding", "conflicting-binding"));
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
            (outermost.as_str(), *found_outer, nested.as_str(), *found_inner),
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
    checks(&protocol(&parallel("Both", "memory ra", "all [left, right]", "")));
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
        let (text, cause, code, tag) =
            only_refusal(&protocol(&parallel("Both", memory, join, "")));
        assert_eq!((code, tag.as_str()), ("unsupported_construct", "declaration-form"));
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
