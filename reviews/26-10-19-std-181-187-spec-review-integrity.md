---
id: SR-12910
title: "Integrity review of PR #1001: FR-185/FR-186 causes against QSpec FR-405, FR-407, FR-272 and the shared grammar"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@739cd6354; FR-185 Behavior and AC-1 to AC-4; FR-186 Behavior and AC-1 to AC-4; FR-311 ExactValue note; FR-392 EventForm and form notes; FR-393 ExactValue; FR-325 TemporalFormulaForm; ADR-024 §7; QSpec FR-405, FR-407, FR-272, native-diagnostics, shared-grammar probabilistic forms"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-392
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
---

## Summary

Tickets: STD-181, STD-187. PR quire-spec-language#1001. This review checks
that every cause the PR cites exists in QSpec (`a2230e1`), that the
fixtures parse under the shared grammar, and that the ExactValue and
EventForm decisions agree across FR-311, FR-392, FR-393 and FR-325.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-185's new Behavior preamble says "Every refusal below is an `invalid_probabilistic_model` diagnostic (QSpec FR-405, FR-407) whose cause is the string in brackets", and the reward bullet then gives [`duplicate-reward`]. QSpec has no such cause. FR-272's `invalid_probabilistic_model` row and native-diagnostics list a closed set of 16 causes, and FR-272-AC-15 requires each FR-405 and FR-407 refusal to carry a cause from that set. As written, the text attributes `duplicate-reward` to QSpec, and an implementation that emits it breaks QSpec's closed set. Nothing in the diff says it is a QSL-local name pending STD-188. Fix: mark it as QSL's name until QSpec adds it, for example "[`duplicate-reward`, QSL name pending QSpec (Remaining work: STD-188)]", and take it out of the preamble's QSpec attribution. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:65-67,109-110,121; quire-specification spec/objects/foundation/FR-272-typed-diagnostic-cause.md:95,192 |
| FND-002 | medium | FR-186 also attributes two causes to QSpec that QSpec does not define. Its preamble attributes every bracketed cause to QSpec FR-407, but [`quantile-over-fraction`] and [`invalid-confidence`] are not in FR-272's closed `invalid_probabilistic_model` set, and QSpec FR-407 gives neither refusal a cause. AC-2, AC-4 and TC-621 then fix both names as expected results. Fix: mark both as QSL names pending STD-188 in the Behavior text and the ACs. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:84-86,134-136,146-148,159,161; quire-specification spec/objects/foundation/FR-272-typed-diagnostic-cause.md:95 |
| FND-003 | medium | FR-185's preamble says "a weight is an `exact-value`, so a signed weight is `invalid_syntax`". The shared grammar contradicts this: `exact-value = uint \| decimal-literal \| exact-number`, and `exact-number` takes `signed-int` (`signed-int = [ '-' ], uint`). So `rational(-1, 1)` and `decimal(-1, 0)` are signed weights that S1 admits, and S3 has to refuse them `invalid-weight`. Read literally, the sentence lets an implementer skip the negative check at S3. Fix: narrow it to "a bare `-` before a weight is `invalid_syntax`; a negative `rational` or `decimal` weight reaches S3 and is refused [`invalid-weight`]". | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:68-69; quire-specification proposals/quire-v1/shared-grammar.md:149,367-368,781 |
| FND-004 | low | The PR rewrites `Monthly` in ADR-024 §7.3 so that it parses, but leaves §7's other spellings ungrammatical, even though FR-185-AC-1 ("ADR-024 §7.2's `Service` model") and FR-186-AC-1 ("ADR-024 §7's four claims") name them as fixtures. The remaining spellings are the inline `operation Server::attempt(random d: Quantity<Time> ~ {…})` and `operation Node::tick(random fault: …)` in place of `random-decl`, `reward duration = d;` inside the operation with no `on` (the shape the PR fixes in FR-185-AC-4), and `long-run fraction holds(v.up)` in place of `long_run_fraction`. §7 says its spellings are illustrative, and QSpec TC-351 and TC-353 hold the grammar-conformant spellings. The fix is either to point the two ACs at QSpec TC-351's spelling of these fixtures, or to make ADR-024 §7 conformant throughout. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:220,256,261,311; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:118; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:158 |
| FND-005 | low | FR-186's decimal sentence lists "a threshold" among the `exact-value` positions, then says "a threshold in an `expr` position … is written `rational(n, d)` or `decimal(n, s)`". The quantile threshold is in an `expr` position, yet AC-2 writes it `0.005 s`. That is grammatical, because a `quantity-literal`'s magnitude is an `exact-value`, but the sentence reads as if it forbids it. Fix: name the `exact-value` positions exactly (a probability, mean-of or long-run threshold, a quantile level, a confidence parameter, a quantity literal's magnitude), and say that a bare dimensionless threshold in an `expr` position is written `rational` or `decimal`. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:86-90,159; quire-specification proposals/quire-v1/shared-grammar.md:781-782,870-873 |

## Verdict

Consistent where the brief asked:

- **QSpec causes exist.** `invalid-weight`, `repeated-support-value`,
  `duplicate-distribution`, `value-outside-type`,
  `random-read-in-precondition`, `incomplete-workload`, `unknown-operation`,
  `duplicate-weight`, `state-read-in-reward` and `reward-type` are FR-405
  causes. `qualitative-threshold`, `partial-confidence`, `foreign-workload`
  and `fairness-under-workload` are FR-407 causes (AC-1, AC-4, AC-5, AC-6).
  All 14 are in FR-272's `invalid_probabilistic_model` row and in
  native-diagnostics. `unit-mismatch` is an `ill_typed` cause in FR-272 and
  native-diagnostics, and FR-186 cites it as `ill_typed`/`unit-mismatch`, in
  parentheses outside the bracket convention, which is correct. The brief
  points to "FR-350" for it, but QSpec FR-350 is Verus contract emission; the
  cause is FR-272's (also used by FR-414). Each QSL refusal maps to the
  QSpec refusal of the same rule.
- **New causes.** `invalid-confidence`, `duplicate-reward` and
  `quantile-over-fraction` are absent from QSpec, and STD-188 (Backlog)
  asks QSpec to add all three. They are not yet presented as QSL names
  pending STD-188 (FND-001, FND-002).
- **ExactValue as written.** FR-393 now holds an `exact-value` as a
  spelling kind and two integers, and S3 reduces it. FR-311 points at
  FR-393's type and says S3 reduces each weight. FR-392 says "`ExactValue`
  is FR-393's: an `exact-value` held as written". The three agree, and the
  FR-393 behavior bullet (`0.0005` held as digits `5` and 4 fraction
  digits) matches its type text.
- **EventForm::Temporal.** It is now `Temporal(TemporalFormulaForm)`, FR-325's
  S2 tree type for a temporal formula (FR-325 Outputs). FR-392's note says
  the same, and it matches QSpec's `event = temporal-expr | …`.
- **"Four claims"** is correct per ADR-024 §7.

Not mergeable as is: FND-001, FND-002 and FND-003 (medium) should be fixed
in this PR. No high.
