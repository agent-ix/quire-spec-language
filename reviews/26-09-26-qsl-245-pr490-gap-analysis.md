---
id: SR-739
title: "PR 490 kernel refusal records gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@985e0d016fa50ef1da5dadff950941ccc7823e86; FR-096-AC-8; FR-096-AC-13; FR-096-AC-14; FR-100-AC-9; FR-100-AC-10; TC-428; TC-500; TC-452; quire-exact/src/outcome.rs; qsl-foundation/tests/kernel_refusal_record.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/tests/it/collection_queries.rs; qsl-replay/src/spine/call.rs; qsl-replay/src/spine/call/tests.rs; src/command/output.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#490 at 985e0d01.
This maps each AC to its tests and measures how strong each oracle is. The mutation checks ran in a detached scratch worktree with its own `CARGO_TARGET_DIR`, which was deleted afterwards.

AC to test:

- **FR-096-AC-8 (TC-428 step 4):**
  - `quire-exact/src/outcome.rs:366` `tc_428_every_record_building_refusal_names_code_and_cause` checks all 15 code/cause cases, and that `CheckedInvariant` gets `None`.
  - `qsl-foundation/tests/kernel_refusal_record.rs:63` and `:153` check each field's exact spelling against AC-8's own examples.
  - `:191` checks that `CheckedInvariant` builds no record.
  - The replay test `tc_452_step_4_outcome_mapping_covers_every_category` checks that each of the ten conversions becomes a `CallRefusal::Record` with a location.
  - These are strong oracles: full field vectors and exact strings.
- **FR-096-AC-13 (TC-428 step 5):** `kernel_refusal_record.rs:202` covers all three causes, with `expected` `Int[0, 9]`.
- **FR-096-AC-14 (TC-500):** `collection_queries.rs:1343`, `:1369` and `:1385` cover the addition case at the `sum` node, with the last charge `integer-arithmetic.arithmetic` and no `result-retain`. They also cover completion with 3, and the seed case at the summand node with no arithmetic charge. `q13` now expects `Undefined::SumOutOfDomain`.

Mutation results:

- Killed: removing the seed check (TC-500 step 3 fails) and removing the integer-target `InexactDecimal` rewrite (`quantities.rs:2394` fails).
- Survived: disabling the empty-sum check, swapping the NaN `target`/`source`, and hard-wiring `IeeeNotExact` Binary32 (the last two are in SR-738 FND-001).

The empty-sum path can be reached. A scratch test ran `sum<Int[1, 3]>(x in q: x)` over an empty `Sequence<Int[1, 3]>[0, 2]`. It checks under `CheckMode::Kernel` and returns `SumOutOfDomain` at the `sum` node. So that path is not just defensive code.

## Verdict

**Changes requested** (two medium findings). Every FR-096 AC in scope is backed by strong oracles. The two paths this PR added and left untested are the empty sum and the replay spelling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The empty-`sum` `SumOutOfDomain` path has no test, and the suite still passes when it is disabled. The PR describes it as defensive, but it can be reached. Under `CheckMode::Kernel`, `sum<Int[1, 3]>` over an empty `Sequence<Int[1, 3]>[0, 2]` checks and reaches it, as a scratch test confirmed. FR-100-AC-10 on main specifies exactly this case (empty `q`, `Pos = Int[1, 9]`, located at the `sum` node, rendered `sum-out-of-domain`, exit 20), and no test backs it. The code is this PR's, so add a qsl-eval test for the empty case next to the TC-500 tests. | qsl-eval/src/value/expression/evaluate.rs:1619-1631 |
| FND-002 | medium | Nothing tests the new replay spelling `Undefined::SumOutOfDomain => "sum-out-of-domain"`. The kernel-reason loop this PR edits still says "each of the four reasons" and lists four. The CLI test `undefined_kernel_reasons_render_and_exit_20` also lists four, while FR-100-AC-9 on main says five. A typo in the spelling would pass. Fix: add `(Undefined::SumOutOfDomain, "sum-out-of-domain")` to the replay loop (test code only; FR-100 and TC-452 are not edited). | qsl-replay/src/spine/call.rs:459; qsl-replay/src/spine/call/tests.rs:554-563; src/command/output.rs:682-690 |
| FND-003 | low | AC-8 asks for a record "built from an S6a `Evaluation`". For the ten value refusals, every record test starts from a hand-built `Refusal`: the foundation tests, the replay `convert` of a constructed outcome, and the `evaluation(refusal)` helper in `model_reference_queries.rs`. The PR removed the value-refusal `None` assertions from `a_kernel_refusal_builds_a_record_only_where_the_catalog_has_a_code` and added no positive case. Together with SR-738 FND-001, this means no test goes from a real kernel raise to a rendered `expected` field. One end-to-end case closes it: a checked `Int[0, 3]` Coerce or arithmetic refused in S6a, then `refusal_record(package.graph())` with `expected` `Int[0, 3]`. | qsl-eval/tests/it/model_reference_queries.rs:3586-3590; qsl-foundation/tests/kernel_refusal_record.rs:34-57 |

## Dispositions

Round 1, re-checked at 94e49221.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 54633014: `tc_500_an_empty_sum_whose_domain_excludes_zero_is_undefined_at_the_sum_node` checks `sum<Positive = Int[1, 3]>` over an empty `Sequence<Int[1, 3]>[0, 2]` under `CheckMode::Kernel`: `SumOutOfDomain` at the `sum` node, with no arithmetic and no retain charge. Disabling the empty-sum check now makes it fail. |
| FND-002 | fixed | 54633014: the replay loop and the CLI test each list five reasons, including `sum-out-of-domain`. The CLI test feeds the string in directly, so it checks rendering only; the replay test checks the mapping. |
| FND-003 | fixed | 54633014: `coerce_and_count_refusals_carry_the_declared_domain` builds the record from a real S6a `Evaluation` through `refusal_record(package.graph())`: `integer_out_of_domain`/`outside-domain`, `expected` `Int[0, 3]`. |
