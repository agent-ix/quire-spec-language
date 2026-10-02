---
id: SR-1212
title: "QSL-613 gap analysis of PR #596 (FR-356, TC-898, TC-899, TC-903)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4aacd77e4364a077ace57655773e0de8025b70d3; PR #596 diff against origin/main; spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md; spec/test-cases/TC-898, TC-899, TC-903"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-899
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-903
    type: reviews
---
## Summary

Ticket: QSL-613. PR: quire-spec-language#596.

Trace:
- FR-356-AC-1:
  - `quire_walk_has_no_features_and_no_dependency` (tests/it) checks both
    the declared and the resolved graph.
  - `tc_arch_lint_metadata_009` and `tc_arch_lint_direction_004` cover the
    shared leaf and the qsl-* refusal.
  - The `make quire-walk-no-std` build passes for `thumbv7em-none-eabi`.
- FR-356-AC-2: the three `tc_898_*` walk tests in quire-walk/tests/it/deep.rs
  run on 512 KiB threads at depth 100,000. They cover the chain in pre-order
  and post-order, the alternating mutually recursive walk with frames of each
  kind, and a stop at 50,000 with no callbacks after it.
- FR-356-AC-3: `tc_898_a_100k_node_arena_computes_subtree_sizes_in_one_forward_loop`.
- FR-356-AC-4: ten Kani harnesses in quire-walk/src/proofs.rs.
- FR-356-AC-6 (context): `maybe_grow_carries_a_100k_deep_recursion_on_a_512_kib_stack`,
  plus the `cfg(kani)` plain-call proof.
- FR-356-AC-7: the `deep_input` fuzz target plus `qsl_bench::deep_input`
  tests. 10,000 runs is the coder's evidence; I did not re-run them.

Kani bound (the coordinator's question):
- The harnesses enumerate every parent array of 1 to 4 nodes in which each
  parent precedes its child: 1 + 1 + 2 + 6 = 10 harnesses. Every ordered
  tree of up to 4 nodes has such a labelling (preorder), so every tree
  shape within the bound is covered. Frame payloads are symbolic.
- FR-356-AC-4 and TC-899 say "for every tree within the harness bound" and do
  not require a symbolic shape. The enumerated bound meets them as written.
- The bound itself, 4 nodes enumerated, is stated only in code (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-356-AC-7 says the fuzz target drives each input "through the S1 parser and the S3 checker". `DeepInput::limits` deliberately keeps the S2 forms and S3 checker depth caps at their defaults. That is correct today, because raising them reaches S3 walks that still recurse natively. But an input deeper than the forms cap is refused at S2 and never reaches S3, and an input deeper than the checker cap stops there, so the target does not yet exercise deep walks past either cap. TC-903's Status says "Implemented" with no caveat. State in TC-903's Status, and in FR-356's Status, that inputs past the default S2/S3 depth caps end in those caps' limit outcomes until ADR-030 slice 1 deletes the caps. The second sentence of AC-7 (a result or a stated limit outcome) is met. | spec/test-cases/TC-903-deep-input-fuzz-target-drives-the-parser-and-checker.md:33; qsl-bench/src/deep_input.rs:205-223 |
| FND-002 | low | FR-356-AC-1 says the direction check "refuses one [edge] to any `qsl-*` crate". FB-05 admits CG's normal edge to `qsl-replay`, and this PR's arch-lint admits exactly that edge, so the AC's literal wording contradicts both FB-05 and the code. Add "other than CG's normal edge to `qsl-replay` (FB-05)". | spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md:96 |
| FND-003 | low | AC-4 and TC-899 are met by enumeration, but neither states what the harness bound is. A reader cannot tell from the spec that the proof covers every tree of 1 to 4 nodes, one harness per parent array, rather than a symbolic shape. State the bound in TC-899's Expected Results (or FR-356 item 4): every tree of at most 4 nodes, enumerated by parent array, with symbolic frame payloads. | spec/test-cases/TC-899-kani-verifies-the-walker-toolkit.md:22-28; quire-walk/src/proofs.rs:1-30 |

## Verdict

Changes requested: FND-001 is medium, FND-002 and FND-003 are low. AC-1 to
AC-4 and AC-6 are traced and tested.
