---
id: SR-2273001
title: "QSL-1633 code review (Rust lane) of PR #2273"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2273 head f1e3e4c40; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/src/value/expression/evaluate/node_source.rs; qsl-eval/src/value/expression/evaluate/invariants.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: references
---

## Summary

Ticket: QSL-1633; PR 2273 (step 1 of QSL-1625). `Machine`, `Task`, `Iteration`, `SeqIteration` and `DispatchGuard` become generic over a crate-private `NodeSource<'a>` trait with `CheckedNode` as the only implementor. The five callee lookups (`DispatchGuard` body, `Call`, `ImportedCall`, dispatch precondition/selected-name, dispatch body) move from `CheckedGraph::function_state` to `NodeSource::callee`. Rust lane (rust-review) folded into this file.

## Verdict

**CONDITIONAL**: behaviour-preserving and perf-neutral, but the trait's `kind` signature cannot be served by the canonical reader `NodeRead` the follow-up step must walk (FND-001).

Behaviour: `CheckedNode::callee` is `graph.function_state(function).map(...)` copying `name`, `body` and `slots`. Those are exactly the three fields of `FunctionState`. Each call site passes the same graph as before: `self.graph` for Call and dispatch, and `library` for ImportedCall. The `ok_or_else(invariant(..))` and `unavailable_imported_call` fallbacks are unchanged. `self.graph` is still swapped by `PackageContext` at the same point, so callee resolution after a package switch is identical. No other `function_state` caller remains in the machine.

Perf: there is no `dyn`. `Machine` is `pub(crate)` and every instantiation is in qsl-eval, which compiles at opt-level 3 in dev. So it is a single monomorphization over `CheckedNode`. The trait methods are trivial forwards to the same inherent qsl-semantics calls the old code made. `children()` already returned a `Vec` before. `Callee` is a by-value struct of three `Copy`-sized fields, the same as `FunctionState`. No new clone or allocation on the hot path.

Focused tests on the head: qsl-eval lib 180 pass / 1 fail. The failure, `tc_454_published_index_sequence_is_the_samplers`, panics on a missing `QSPEC_DIR` in simulation/sample.rs, outside the diff and environmental. The PR reports the same failure on main. qsl-eval `it`, filtered to dispatch|import|call|precondition: 113 pass / 0 fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `NodeSource::kind(self) -> &'a NodeKind<Self::Child>` fixes the literal parameter to the default `CheckedLiteral` and requires a borrow that lives for `'a`. The canonical reader descriptor is `quire_checked::NodeRead<'a, Child>`, which holds `operation: NodeKind<Child, &'a Value>` **by value** with `&Value` literals. A reader-backed source therefore cannot implement `kind` from a `NodeRead` without materialising its own `NodeKind<Child, CheckedLiteral>` arena. That is the checked-term reconstruction FR-390-AC-16 excludes. The machine also hard-codes `NodeKind::Literal(CheckedLiteral(value))` (evaluate.rs:1811). As written, step 2 must change `kind`'s signature and the literal match, so this trait is not yet the shape the one-machine reader walk needs. Fix: add `type Literal: Borrow<Value>` (or yield `&'a Value` literals) and match literals through it. Then either keep `&'a NodeKind<Child, Literal>` with a documented requirement that the reader owns stored operations for `'a`, or return the operation by value via `NodeRead`/`map_ref`. Measure the second option, because `map_ref` allocates for `Vec<Child>` operands on every visit. Record the choice in the trait doc. | qsl-eval/src/value/expression/evaluate/node_source.rs:12-17; qsl-eval/src/value/expression/evaluate.rs:1811 |
| FND-002 | low | `callee(self, graph: &'a CheckedGraph, function)` keys the callee on a `CheckedGraph` and a `CheckedGraph` function index, and the `CheckedNode` impl ignores `self`. A reader source's `NodeKind::Call` function index names its own projected function table, not `self.graph`'s, so the `graph` argument is meaningless to it. `ImportedCall` still resolves the library via `self.graph.scope().imported_graph(..)` before `callee` is reached (evaluate.rs:2526). The trait's doc ("Function `function` of `graph`") makes the CheckedGraph the authority, which a reader impl would have to contradict. Fix: document `graph` as the checked fallback, or drop it and have the source carry its own function table, so the reader impl does not special-case it. | qsl-eval/src/value/expression/evaluate/node_source.rs:35-38,78-84; qsl-eval/src/value/expression/evaluate.rs:2526-2536 |
