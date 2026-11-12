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
