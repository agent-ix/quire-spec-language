---
id: SR-17225
title: "code-review (with rust-review lane) of QSL-1071 hyperproperty node emission (quire-spec-language PR 1560)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@843fe72ebabf115a94cd969e1e3db9f06e0ea7d5; git diff origin/main...843fe72eb (PR 1560, branch qsl-1071-hyper-code): qsl-semantics/src/check/{mod,assemble}.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-semantics/src/check/hyper_clause/{emission,lowering}.rs, qsl-semantics/src/check/lowering/{hyper,temporal,model,state}.rs, qsl-package/src/checked_v2/{hyper_root,clause_root}.rs, qsl-package/src/emit/tests/hyper_clause.rs, qsl-forms/src/spans.rs; spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1266
    type: references
---

# Code review of QSL-1071 hyperproperty node emission (quire-spec-language PR 1560)

## Summary

Ticket: QSL-1071. Reviewed head 843fe72ebabf115a94cd969e1e3db9f06e0ea7d5 (PR 1560 was merged by the owner at e1e5edddb, head 35de233f3, while this review ran; 843fe72eb..35de233f3 differs only by a merge of main, no change to the files reviewed). One pass: code-review with the rust-review lane over the whole diff (20 files, +1851/-59), read against merged FR-488 (AC-1 to AC-23), TC-1266, FR-487, FR-337, FR-402 as FR-488 quotes it. Production behaviour was probed with throwaway tests on the ConfigVersion fixture (not committed).

The production code is faithful to FR-488. Traced and probed:

- Node set and seven bindings in order; `quantifiers` is a Tuple of Groups (quantifier Group, then a fairness Group per constraint, after its own variable); fairness `granularity` resolved, so unmarked and `weak whole` emit one NodeId, `each` another, a constraint on `b` differs from the same on `a`, `exists`, reversed order, a changed match and a trace rename each change the NodeId, and reformatting does not (probe).
- Slot names and levels. `esc` is byte-wise and injective, the atom slot level is its index in `atom_slots()`, a conjunct's slot level is its index in `slots()`. Probe: `b.step.op = stutter and a.step.op = b.step.op` mints `op_sb` at level 0 and at level 1 as two nodes and `op_sa` at level 0, so no level/key collision.
- `conjuncts()` orders both parts by span start (probe: `U0, E0, U1, E1` for an interleaved clause with a constant conjunct) and the same order drives `match` and the sites.
- Wrapper: `quire.op.temporal.clause` with the lock's `temporal_profile` law (probe: lock row equals the law), `over` slot 0, `model-steps`, `origin` activation, empty captures and fairness, `formula` the root. `clause_roots` and `resolve_clause_root` skip it; `hyper_roots`/`resolve_hyper_root` return `hyper-root-unresolved` for an unknown id, a non-hyper node, a non-claim role and a missing ordinal.
- Refusals: precedence 1 to 5 is one function with one early return per rule (QSL-1094 can lift 1, 4, 5 and 2 independently). Probes: a timed clause is refused at the clause; `(let r = (c.parent @ a) in ... deref(value(r)).versionNumber ...) @ b` is refused at the read `deref(value(r)).versionNumber`; the same `let` inside `@ a` emits and is admitted; infinite-trace `until` and `eventually [2, *]` and event-position `always [0, 3]` emit and are admitted.
- `occurrence()` is assigned from the assembled `claim` occurrence of the `hyper <c>` site and round-trips to the I2 reader's triple.

Remaining findings are one gate regression the PR introduces, test-quality points (AC-21, two wrong trace tags) and low code points. The coder's admitted gaps are judged in the gap analysis (SR-17226); this review only adds the production-code side.

Judgement of the production-code residuals the coder named:

- `occurrence()` returning the zero key for an unassembled clause: acceptable. The FR-488 signature returns `&OccurrenceKey` and the PR adds `assembled_occurrence() -> Option` beside it; every clause reachable from `CheckedGraph::hyper_clauses()` is assembled. Not a must-fix (FND-005, low).
- Untested argument-read, interleaved, `Linked`, systems_interface, two-refusal and `until` paths: the production code for each is correct in the probes above; the missing evidence is a test gap, judged in SR-17226.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-488 | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md | examined |
| TC-1266 | spec/test-cases/TC-1266-a-hyper-clause-emits-a-hyperproperty-claim-node.md | examined |
| FR-487 | spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md | context_only |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The PR adds a new `string-edge` gate failure: `hyper_root.rs` compares a binding name with `&**name == "body"` in branch-gating position, unmarked and unlisted. `make ci` runs `string-edge` and it fails with 12 occurrences on the PR head against 11 on its base (7b3f9235b); the 11 others are in `qsl-replay` and are not this PR's. Mark it with `#[ix_trace_rs::string_edge]` per the xtask convention or compare through a typed name. | qsl-package/src/checked_v2/hyper_root.rs:80 | implementation-bug-despite-evidence |
| FND-002 | medium | FR-488-AC-21's test `the_emitted_nodes_count_against_the_node_ceiling` uses `with_nodes(8)` and asserts `is_err()` only. The real boundary is 10 (probe: 10 compiles, 9 is refused `ResourceExhausted` Typing/Nodes at `hyper 0 match 0`), so the test passes two nodes below the boundary, any error satisfies it, and "at the ceiling it compiles" is not run. The pattern to copy is `nesting_is_bounded_only_by_the_node_ceiling` (needed - 1 refuses with kind and limit, needed compiles). Whether the nodes S4 mints (wrapper, slots, formula nodes) are charged to anything but the work meter is not shown by any test. | qsl-package/src/emit/tests/hyper_clause.rs:493-503 | correct-requirement-no-evidence |
| FND-003 | medium | `a_fairness_set_under_a_bounded_profile_is_not_emitted` carries `FR-488-AC-18` and `FR-488-AC-19` but asserts FR-488-AC-20 (a bounded-profile clause with a fairness set is refused). It asserts only that some refusal is `NotEmitted`; not the cause location (the constraint), and it never runs a timed clause (AC-18) or an emitted interval (AC-19). The tags bind the test to ACs it does not check and leave AC-20 untagged. | qsl-package/src/emit/tests/hyper_clause.rs:472-491 | correct-requirement-no-evidence |
| FND-004 | medium | `a_hyper_clause_emits_the_hyperproperty_node_with_its_seven_bindings` carries `FR-488-AC-6`, but it compiles one-alias `NonInterference` over ConfigVersion. No test has two aliases, so `subjects` with `S` and `N` in order of first naming, parameters `d` and `e`, and slots `slot_sd_ss` and `slot_se_sn` are never run. | qsl-package/src/emit/tests/hyper_clause.rs:172 | correct-requirement-no-evidence |
| FND-005 | low | `CheckedHyperClause::occurrence()` returns the all-zero key (`[0;32]`, claim ordinal 0) for a clause no package assembled. The required signature leaves no `Option`, and the PR adds `assembled_occurrence()`, but a consumer that calls `occurrence()` on a clause from `HyperTraceFamily::check` gets a plausible-looking triple. Accepted; the doc comment should say the FR-315 consumer reads clauses from `CheckedGraph::hyper_clauses()` only. | qsl-semantics/src/check/hyper_clause.rs:85-92 | wrong-requirement |
| FND-006 | low | `hyper_skipped` is built with `filter_map` (an operation that does not resolve is dropped) and later consumed by a positional `skipped.next()` across clauses, so a dropped entry would shift every later clause's anchors; the `align.len() != clause.align_skip().len()` guard then `continue`s silently and surfaces only as the generic `unlowered()` fault. S3 resolves every `align skip` operation, so this is unreachable today. Keep one vector per clause, or raise a stage fault at the drop. | qsl-semantics/src/check/mod.rs:3380-3410 | implementation-bug-despite-evidence |
| FND-007 | low | `unencodable_read` replaces a `Query` slot and a `Fold` binder by their source subtree when it computes a read's roots, and treats `AllInstances` as a rootless read. FR-488 "Clauses S4 does not emit" names only the `let`-bound replacement, so a read through a quantifier or fold binder over a slot-rooted source is encodable here and would be refused by the letter of the spec. Probably the intended generalisation; record it in FR-488 or restrict the code. | qsl-semantics/src/check/hyper_clause/emission.rs:395-408 | wrong-requirement |
| FND-008 | low | A constant conjunct (`true`) emits as a `reference` term to a boolean `literal` expression node, not a `literal` term. FR-488 "Match conjuncts" and AC-8 say "a `literal` term"; the code returns what the FR-093 lowering returns, which is what the first sentence of that bullet asks for. Amend AC-8/Q-4 to the actual term, or test the term and change the lowering. | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:358-361 | wrong-requirement |
| FND-009 | low | `wrappers()` hides any `temporal_clause` node that is a hyper `body` target from `clause_roots` and `resolve_clause_root` by node content, so a standalone temporal clause that is structurally equal to a wrapper (the case FR-488 Q-5 names) disappears from FR-338/FR-315 lookups. Practically unreachable (it needs a user parameter named `slot_s...` at level 0 with the same formula); accepted per Q-5. | qsl-package/src/checked_v2/clause_root.rs:108-116 | wrong-requirement |

## Verdict

FAIL on the aggregate gate, PASS on the production code. The code is faithful to FR-488 and no wrong-output path was found. The `string-edge` regression (FND-001) is introduced by this PR and keeps `make ci` red beyond the known baseline; FND-002 to FND-004 are test-evidence defects. Because PR 1560 is already merged, land FND-001 to FND-004 in a follow-up PR against main (e1e5edddb), with the gap-analysis tests (SR-17226 FND-001 to FND-005).
