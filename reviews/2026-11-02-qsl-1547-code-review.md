---
id: SR-2026110215471
title: "code-review of QSL-1547 protocol read surface (quire-spec-language PR 2158), rust lane included"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR 2158 diff at 6523d4a2c124d2ccfef09c774eb23e4e5068d447 (qsl-package/src/protocol_view.rs, qsl-package/src/emit/tests/protocol_view.rs, shared test-visibility edits)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: reviews
---

# Code review of QSL-1547 (PR 2158)

## Summary

Ticket: QSL-1547. Head 6523d4a2c, base main 1d2a256d4. The causal relation is derived correctly from QSpec FR-464 "Derived causal relation": every rule matches the check-time derivation in qsl-semantics graph.rs (sequence, branch, join, repeat progress plus back edge, await success/timeout/anchor, message, effect). No allow attribute, no unwrap/expect/panic on a production path, no change to any Cargo manifest or lock, no quire-protocol dependency, no pin. Shared-file edits are visibility-only. Workspace check, clippy -D warnings and the no-std check of qsl-package pass; the 8 new tests and 5 doctests pass. Mutation checks: dropping the repeat back edge, moving the message edge to the sender's enter port, dropping sort/dedup, removing the layout reversal and swapping the delivery bounds each turn a test red; making every parallel branch join leaves all 8 tests green (FND-001).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The `joined` filter of a `parallel` is unprotected: replacing the join condition with a constant true leaves every test green, because the fixture's only `parallel` (Shipments) joins both branches. A derivation that joins every branch would pass TC-1314 row 1. | qsl-package/src/protocol_view.rs:701; qsl-package/src/emit/tests/protocol_view.rs:23-37 | correct-requirement-no-evidence |
| FND-002 | medium | The `await ... after <compensation>` branch of `awaiting` (edge from the enter port of the template's `forward` effect) has no test at all, so the one FR-464 anchor rule the derivation reads through a second node is unverified; the missing end-to-end path (check refuses such a source) does not prevent a test over a package built by editing the fixture's await node, which the other tests already do. | qsl-package/src/protocol_view.rs:732-736 | correct-requirement-no-evidence |
| FND-003 | medium | FR-519-AC-2 is neither satisfied as written nor evidenced: `git grep` finds the same relation derived in production at qsl-semantics/src/check/protocol_clause/graph.rs (`compile`) and in src/protocol_artifact/native/controls.rs, and no test or inspection checks AC-2, yet the equality test carries the tag FR-519-AC-2. The equality test does pin protocol_view's derivation to graph.rs's, which bounds the drift risk. | qsl-package/src/emit/tests/protocol_view.rs:215 | correct-requirement-no-evidence |
| FND-004 | low | The AC-6 test is a source-text grep (five reader names absent from protocol_view.rs, one substring absent from qsl-package/Cargo.toml). It does not read the resolved dependency graph (Cargo.lock) and does not run arch-lint direction; the grep is not evidence of "no byte read" (an aliased import passes). | qsl-package/src/emit/tests/protocol_view.rs:874-889 | correct-requirement-no-evidence |
| FND-005 | low | Several AC-3 and AC-5 comparisons are weaker than their AC text: compensations are compared to hard-coded names, not the in-process clause; controls are compared to the in-process tree as a set (order rests on a hard-coded list); activation rows compare only the `Each` variant, not the guard or trigger; `source` and `definition` return the element `find` matched against the argument (a membership test that cannot return anything but its input); the `dependency` success path uses a synthetic selection. | qsl-package/src/emit/tests/protocol_view.rs:300-330; qsl-package/src/protocol_view.rs:950-992 | correct-requirement-no-evidence |
| FND-006 | low | The `compile_fail,E0432` doctests name types that never existed in the module, so they fail to resolve before and after any deletion; they cannot detect a re-export of an Ended record under that name. | qsl-package/src/protocol_view.rs:12-32 | correct-requirement-no-evidence |
| FND-007 | low | `protocol_view` walks the `run` tree with no visited set or bound; a package where two controls hold the same child would list it twice and a deep shared DAG would expand exponentially. Safe only while IR admission guarantees a tree; low confidence since admission likely guarantees it. | qsl-package/src/protocol_view.rs:555-559 | correct-requirement-no-evidence |

## Verdict

Mergeable after FND-001 and FND-002 are closed with small tests (a partial-join `parallel` in the fixture; an await anchored at a compensation built by editing the fixture node). The derivation itself is correct; no high finding. FND-003 is a spec defect to route to the spec lane, not a code change.

## Dispositions

Round 1, against head 45ec3ccd5fd6b5a43db3a47abf71ef676b567b1b. Mutations rerun by the reviewer on that head: join-any (constant true join condition) turns `a_parallel_joins_only_the_branches_its_join_names` red; AwaitAnchor moved to the forward effect's exit port turns `an_await_after_a_compensation_is_anchored_at_the_forward_effect` red; removing the visited-set refusal turns `a_control_held_twice_is_refused_not_listed_twice` red; asserting a known transitive crate (serde_json) absent from the Cargo.lock closure turns the AC-6 test red, so the closure walk reaches transitive packages (a literal quire-protocol lock edit cannot be run under `--locked`, cargo rewrites it). Whole-workspace check, clippy -D warnings, fmt, no-std, full qsl-package suite (544 passed, 2 ignored, plus 3 and 27), arch-lint qualified-core, api-surface --qsl-only and direction over fresh shallow clones of IR, runtime and codegen all pass.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | New partial-join test `a_parallel_joins_only_the_branches_its_join_names` (`join any [a]`, asserts one Join edge and equality with the checked relation); join-any mutation is red. |
| FND-002 | fixed | New test `an_await_after_a_compensation_is_anchored_at_the_forward_effect` over an edited fixture await node, asserting the forward effect's enter port; anchor-exit mutation is red. |
| FND-003 | deferred | Spec-lane item: FR-519-AC-2's premise is false (the relation is also derived in qsl-semantics graph.rs); the AC-2 tag is removed from the test and the PR body states AC-2 is unchecked. |
| FND-004 | fixed | The AC-6 test now parses Cargo.lock and walks the resolved closure of qsl-package for quire-protocol; arch-lint direction passes over fresh clones. |
| FND-005 | fixed | Compensations compare to the in-process obligations, controls compare to an in-process layout in order, activation compares the whole row, `source` takes an identity, `definition` is replaced by `selection` over profile selections compared to the wire, and `dependency` is read from a real two-package closure. |
| FND-006 | accepted-no-change | A compiling control doctest was added, which rules out a path typo as the cause of the E0432 failures; the Ended-name doctests still cannot detect a re-export, which the spec's own "compile-fail use of each" wording asks for. |
| FND-007 | fixed | `control_tree` refuses a control held twice with MalformedMember on `run`; test `a_control_held_twice_is_refused_not_listed_twice`; removing the refusal turns it red. |
