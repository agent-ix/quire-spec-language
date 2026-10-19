//! FR-219 "Source" and FR-220 "Admitted orderings": the S3 checks of a
//! `parallel`'s `memory` clause and of a `fence` control.

use ix_trace_rs::trace;
use qsl_forms::{FenceOrdering, MemoryModelName};

use super::super::protocol_controls::tests::{checks, located, only_refusal, pipeline};
use super::super::ProtocolAnchorCause;

const SC: MemoryModelName = MemoryModelName::Sc;
const TSO: MemoryModelName = MemoryModelName::Tso;
const RA: MemoryModelName = MemoryModelName::Ra;

/// `parallel P` with an optional `memory` clause and the given join.
fn parallel(model: Option<&str>, join: &str) -> String {
    let clause = model.map_or_else(String::new, |model| format!("memory {model} "));
    format!(
        "parallel P {clause}{{ branch east sequence E {{ }} branch west sequence W {{ }} }} \
         join {join};"
    )
}

/// `parallel P` holding `parallel Q` in its `east` branch, each with an
/// optional `memory` clause.
fn nested(outer: Option<&str>, inner: Option<&str>) -> String {
    let clause = |model: Option<&str>| model.map_or_else(String::new, |m| format!("memory {m} "));
    format!(
        "parallel P {}{{ branch east parallel Q {}{{ branch a sequence A {{ }} \
         branch b sequence B {{ }} }} join all [a, b]; branch west sequence W {{ }} }} \
         join all [east, west];",
        clause(outer),
        clause(inner),
    )
}

/// The default each checked `parallel` of `Pipeline` records, by control
/// name, in source order.
fn recorded(run: &str) -> Vec<(String, MemoryModelName)> {
    let graph = checks(&pipeline("", run));
    let protocol = graph.protocol("Pipeline").expect("the checked protocol");
    protocol
        .parallels
        .iter()
        .map(|parallel| {
            let control = protocol
                .controls
                .control(parallel.node)
                .expect("the parallel is in the control tree");
            (control.name.clone(), parallel.memory)
        })
        .collect()
}

fn named(entries: &[(&str, MemoryModelName)]) -> Vec<(String, MemoryModelName)> {
    entries
        .iter()
        .map(|(name, model)| ((*name).to_owned(), *model))
        .collect()
}

/// A `parallel` records its declared default, `sc` when it declares none; a
/// nested `parallel` declaring its outermost `parallel`'s default, or none,
/// checks.
#[trace("TC-664", "FR-219-AC-1")]
#[test]
fn a_parallel_records_its_declared_memory_default_and_sc_when_it_declares_none() {
    assert_eq!(
        recorded(&parallel(None, "all [east, west]")),
        named(&[("P", SC)])
    );
    assert_eq!(
        recorded(&parallel(Some("tso"), "all [east, west]")),
        named(&[("P", TSO)])
    );
    assert_eq!(
        recorded(&parallel(Some("ra"), "all [east, west]")),
        named(&[("P", RA)])
    );
    assert_eq!(
        recorded(&nested(Some("tso"), Some("tso"))),
        named(&[("P", TSO), ("Q", TSO)])
    );
    assert_eq!(
        recorded(&nested(Some("tso"), None)),
        named(&[("P", TSO), ("Q", SC)])
    );
    assert_eq!(
        recorded(&nested(None, Some("sc"))),
        named(&[("P", SC), ("Q", SC)])
    );
}

/// A nested `parallel` declaring another default than its outermost
/// `parallel`'s refuses `invalid_model_binding`/`conflicting-binding` at its
/// clause, naming both controls and both defaults.
#[trace("TC-664", "FR-219-AC-1")]
#[test]
fn a_nested_parallel_declaring_another_default_refuses_naming_both_controls() {
    for (outer, inner, outer_model, inner_model) in [
        (None, "tso", SC, TSO),
        (Some("tso"), "ra", TSO, RA),
        (Some("tso"), "sc", TSO, SC),
    ] {
        let (text, cause, code, tag) = only_refusal(&pipeline("", &nested(outer, Some(inner))));
        assert_eq!(
            (code, tag.as_str()),
            ("invalid_model_binding", "conflicting-binding")
        );
        let ProtocolAnchorCause::ConflictingMemory {
            parallel,
            model,
            outermost,
            outermost_model,
            ..
        } = &cause
        else {
            panic!("a ConflictingMemory cause, got {cause:?}");
        };
        assert_eq!(
            (
                parallel.as_str(),
                *model,
                outermost.as_str(),
                *outermost_model
            ),
            ("Q", inner_model, "P", outer_model)
        );
        assert_eq!(located(&text, &cause), format!("memory {inner}"));
    }
}

/// A `parallel` declaring `tso` or `ra` whose join is not `all`, or that
/// has an `outstanding` clause, refuses `unsupported_construct`/
/// `declaration-form` at the join, naming the policy and the weak default;
/// `join all` with no `outstanding` clause, and `join any` under `sc`,
/// check.
#[trace("TC-664", "FR-219-AC-1")]
#[test]
fn a_weak_parallel_whose_join_is_not_all_refuses_naming_the_policy_and_the_default() {
    checks(&pipeline("", &parallel(Some("ra"), "all [east, west]")));
    checks(&pipeline("", &parallel(None, "any [east]")));
    checks(&pipeline(
        "",
        &parallel(Some("sc"), "all [east, west] outstanding continue"),
    ));

    for (model, join, policy, written) in [
        (RA, "any [east]", "any", "any"),
        (TSO, "quorum(1) [east, west]", "quorum(1)", "quorum(1)"),
        (
            TSO,
            "all [east, west] outstanding continue",
            "all outstanding continue",
            "all",
        ),
        (
            RA,
            "any [east] outstanding cancel",
            "any outstanding cancel",
            "any",
        ),
    ] {
        let source = pipeline("", &parallel(Some(model.spelling()), join));
        let (text, cause, code, tag) = only_refusal(&source);
        assert_eq!(
            (code, tag.as_str()),
            ("unsupported_construct", "declaration-form")
        );
        let ProtocolAnchorCause::WeakJoin {
            parallel,
            model: declared,
            policy: found,
            ..
        } = &cause
        else {
            panic!("a WeakJoin cause, got {cause:?}");
        };
        assert_eq!(
            (parallel.as_str(), *declared, found.as_str()),
            ("P", model, policy)
        );
        assert_eq!(located(&text, &cause), written);
    }
}

/// A `fence` with each ordering the grammar admits checks, and is recorded
/// with its ordering; a protocol holding one is no longer refused as
/// unimplemented.
#[trace("TC-665")]
#[test]
fn a_fence_checks_and_records_its_ordering() {
    let run = "sequence Main { \
         fence F1 ordering acquire; fence F2 ordering release; \
         fence F3 ordering acq_rel; fence F4 ordering seq_cst; }";
    let graph = checks(&pipeline("", run));
    let protocol = graph.protocol("Pipeline").expect("the checked protocol");
    let recorded: Vec<(String, FenceOrdering)> = protocol
        .fences
        .iter()
        .map(|fence| {
            let control = protocol
                .controls
                .control(fence.node)
                .expect("the fence is in the control tree");
            (control.name.clone(), fence.ordering)
        })
        .collect();
    assert_eq!(
        recorded,
        [
            ("F1".to_owned(), FenceOrdering::Acquire),
            ("F2".to_owned(), FenceOrdering::Release),
            ("F3".to_owned(), FenceOrdering::AcqRel),
            ("F4".to_owned(), FenceOrdering::SeqCst),
        ]
    );
    assert!(protocol.parallels.is_empty());
}
