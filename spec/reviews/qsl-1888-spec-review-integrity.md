---
id: SR-599003
title: "Integrity review of QSL-1888 join outstanding rule and repeat proof rulings"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md, spec/test-cases/TC-663-s3-checks-protocol-control-constructs.md (PR #2682 diff, head b2f26c872)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: reviews
---
# SR-599003: Integrity review of QSL-1888 join outstanding rule and repeat proof rulings

## Summary

Ticket: QSL-1888. Cross-requirement consistency check of PR #2682 (head
b2f26c872, based on current main 4f692347b). The PR adds an S3 refusal for a
non-`all` join with no `outstanding` rule (FR-218, FR-218-AC-40) and states
that QSL V1 has no prover for a `repeat`'s invariant and variant (FR-218,
FR-390, FR-390-AC-74).

What holds:

- **G3b matches the stepper.** `qsl-eval/src/simulation/protocol/program.rs:608`
  refuses `UnruledOutstanding` exactly when
  `parallel.outstanding.is_none() && parallel.policy != CheckedJoinPolicy::All`.
  The new FR-218 rule refuses the same set: `any`, `quorum(n)` and `predicate`
  with no rule, and exempts `all`. The predicate case is refused earlier by
  `Unsupported::JoinPredicate` (line 605) in the stepper, which does not change
  the set.
- **QSpec agrees.** QSpec FR-171 says each join declares whether unsettled
  branches continue or are cancelled. QSpec FR-466 keeps `outstanding`
  optional on the wire, which still holds for `join all`.
- **FR-206 and FR-216 do not own or contradict the rule.** FR-206 states only
  the `cancel` and `continue` behaviour and uses an explicit rule in every
  non-`all` AC (AC-3, AC-6, AC-9, AC-11). FR-216 uses only explicit rules.
  FR-205-AC-3/AC-7 and FR-383's "a `parallel` with no `outstanding`" (`join
  all`) stay valid.
- **G5 is consistent.** No other QSL statement says S3 discharges a `repeat`'s
  invariant or variant. ADR-014 §absent-bounds already says "A declared
  variant that is not proved settles `inconclusive`".
- **No AC number collisions.** FR-218-AC-40 and FR-390-AC-74 are unused on
  main, and no other open PR touches FR-218, FR-219, FR-390 or TC-663.

The blocking defect is FR-219. Its AC-1 and AC-4 use a non-`all` join with no
stated rule, and the test that backs AC-1 checks exactly the joins the new rule
refuses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-218's new rule contradicts FR-219, which the PR leaves unchanged. FR-219-AC-1 has `memory ra` on a `parallel` joined `any` refuse `unsupported_construct`/`declaration-form` at the join. Under the new rule that join, with no `outstanding` rule, also refuses `invalid_package`/`invalid-value` at the same locus. Neither FR says which refusal wins or whether both are reported. The TC-664 test that backs AC-1 (`a_weak_default_requires_join_all_without_outstanding`) uses `any [left]` and `quorum(1) [left, right]` with no rule, asserts `only_refusal` is the weak-join refusal, and asserts that `memory sc` + `any [left]` and no clause + `any [left]` check. That last assertion is exactly what FR-218-AC-40 now refuses. FR-219-AC-4 ("joins `any`" with no memory clause, checks under `sc`) states no rule either, so as written it is a protocol S3 now refuses. Fix: amend FR-219-AC-1 and AC-4 to write `outstanding cancel` or `continue` on their non-`all` joins, and say in FR-219 or FR-218 how the weak-join refusal and the missing-rule refusal combine at one join. | spec/functional/FR-219-declare-and-select-the-memory-model-of-a-parallel.md:118,121 · spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:210-214 · qsl-semantics/src/check/protocol_memory/tests.rs:205-230 |
| FND-002 | medium | No ticket owns the S3 implementation of the new SHALL, or the fixture fallout. The PR body names only QSL-1885 (predicate-join stepping) as remaining work. S3 does not refuse the case today: `CheckedParallel.outstanding` is documented as "None when the source writes none, which carries no default", and no check refuses it. When the rule lands, these emit tests that reach S3 with a non-`all` join and no rule will refuse: qsl-package/src/emit/tests/protocol_view.rs:458,709; protocol_recursion.rs:257; protocol_graph.rs:1747,1796,1903,1942, plus the TC-664 test in FND-001. FR-218-AC-40 is also not marked as waiting on that work, unlike the "Passes now" / "Waits on" marking FR-218-AC-39 and FR-390 ACs use. Fix: file or name the implementing ticket, and note the fixture updates it carries. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:638 · qsl-semantics/src/check/protocol_clause/types.rs:450-452 · qsl-package/src/emit/tests/protocol_view.rs:458 |
| FND-003 | medium | TC-663 adds step 10 but no expected result for it. Expected Results stops at Step 9, so the refusal code, locus and the "checks" outcomes of FR-218-AC-40 and the inconclusive settlement of FR-390-AC-74 have no stated oracle in the TC. Step 10 is also placed after the "Tag the tests `#[trace(\"TC-663\", \"FR-218-AC-n\")]`" line, which breaks the numbered procedure, and that tag line does not cover the newly scoped FR-390-AC-74. Fix: move step 10 above the tag line, add a "Step 10" expected result, and widen the tag line to FR-390-AC-74. | spec/test-cases/TC-663-s3-checks-protocol-control-constructs.md:42-45 |

## Verdict

REVISE. FND-001 is high and blocks merge: the PR makes FR-219-AC-1/AC-4 and
their existing test contradict FR-218. FND-002 and FND-003 are medium and
should be fixed in this PR. The G3b condition itself matches the stepper and
QSpec, and G5 is consistent with the rest of the QSL spec.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | Other existing QSL ACs and test descriptions still say non-`all` joins with no `outstanding` rule check, which FR-218's new rule refuses. FR-218-AC-23 and TC-1065 row 7: "`join quorum(2) [east, west]`, `join any [east]` ... check". FR-218-AC-31 and TC-1065 row 11: "`join predicate(f) [a, c]` and `join predicate(f) [c, a, c]` each check". FR-381-AC-11 and TC-1058 rows 19b and 19d: a commit in a branch of `join any [a, b]` or `quorum(1) [a, b]` "check"; rows 19c and the AC's `join any [a]` / `quorum(2) [a, b]` cases expect only the `commit-precedes-effect` refusal, but would now also refuse for the missing rule. The tests that back them (qsl-semantics/src/check/protocol_controls.rs, protocol_obligation/tests/recorded.rs, protocol_requirement/tests.rs) are not in QSL-1891's fixture list. Fix: write `outstanding cancel` or `continue` on these joins in FR-218-AC-23, FR-218-AC-31, FR-381-AC-11, TC-1065 rows 7 and 11 and TC-1058 rows 19b to 19d, and add those test files to QSL-1891. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:622,630 · spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:289 · spec/test-cases/TC-1065-a-channel-s-loss-selector-and-the-admission-rules-emission-needs.md:40,44 · spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md:58-59 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | Joins with no `outstanding` rule are still written in four places that the round-1 sweep did not reach. Each place expects the protocol to check, emit or step, but FR-218 now refuses it at S3. (1) FR-383-AC-22 and TC-1061 row 9: "**W** with `Shipments` joined `predicate(both) [west, east]`" emits a `parallel` node. With no rule, S3 refuses it and nothing is emitted. (2) TC-1061 row 6: "**W** with each join policy (`quorum(2)`, `any`, `predicate`), a `parallel` memory model and `outstanding`" lists `outstanding` as a separate variant, so the policy variants read as having no rule. (3) FR-206-AC-3 (TC-651): `join quorum(2)` is not enabled after one completion, and `join predicate(f)` with `f` true exactly when `a` is completed. Neither has a rule or an ellipsis, so the stepper is given a protocol S3 refuses. (4) TC-1297 **Y** and TC-1062 `Flow` are predicate-join protocols whose fixture text states no rule (low on its own). Fix: write `outstanding cancel` on the joins in FR-383-AC-22, TC-1061 rows 6 and 9 and FR-206-AC-3, and say in the TC-1297 and TC-1062 fixture prose that the join carries one. | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:631 · spec/test-cases/TC-1061-s4-emits-each-protocol-construct-as-its-v2-protocol-node.md:52,55 · spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:119 · spec/test-cases/TC-1297-protocol-results-stay-independent-when-one-protocol-is-refused.md:24 · spec/test-cases/TC-1062 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-383-AC-14 and TC-1062 row 9 list "a join policy changed" among the edits to **W** that change a node key. **W**'s `Shipments` is `join all` with no `outstanding` rule. Changing it to `any`, `quorum` or `predicate` without adding a rule makes S3 refuse the protocol, so the variant never reaches the key comparison. A test author finds this at once, so no wrong behaviour ships, but the variant is underspecified. Fix: write "a join policy changed (to `join any [east, west] outstanding cancel`)", or defer it to QSL-1891's fixture work. | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:623 · spec/test-cases/TC-1062-protocol-node-keys-order-and-determinism.md:50 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | de65fd145: FR-219-AC-1 and AC-4 now write `join any … outstanding cancel`; FR-218 states the missing-rule and weak-join refusals are independent and both reported at one join; TC-664 steps 1 and 4 updated. |
| FND-002 | fixed | de65fd145: FR-218-AC-40 is marked "Pending QSL-1891"; QSL-1891 owns the S3 refusal and lists the TC-664 and emit-test fixture updates. |
| FND-003 | fixed | de65fd145: step 10 sits above the tag line, the tag line adds FR-390-AC-74, and Expected Results has a Step 10 oracle. |
| FND-004 | fixed | 7b848350b: `outstanding cancel` added to the non-`all` joins in FR-218-AC-23, FR-218-AC-31, FR-381-AC-11, TC-1065 rows 6, 7, 11 and 12, and TC-1058 rows 19b, 19c, 19d and 19f. |
| FND-005 | fixed | 531f171b4: `outstanding cancel` added in FR-383-AC-22, FR-206-AC-3, TC-1061 rows 6 (and a `continue` variant) and 9, and the TC-1062 `Flow` and TC-1297 **Y** fixtures; FR-383-AC-6, FR-383-AC-28 and FR-308 are aligned. |
