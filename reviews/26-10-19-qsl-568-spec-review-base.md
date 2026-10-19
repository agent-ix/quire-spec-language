---
id: SR-12896
title: "base review of QSL-568 S2 probabilistic claim form (PR #990)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a18d54884; spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md, spec/test-cases/TC-1077-s2-builds-the-probabilistic-claim-form.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-392
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1077
    type: reviews
---
# SR-12896: base review of QSL-568 (PR #990)

## Summary

Ticket: QSL-568. Base checklist over the diff `origin/main...a18d54884`:
new FR-392 (AC-1 to AC-3), new TC-1077, the spec index row and `contains`
edge, and the FR-186 back-reference. The ids are well formed and do not
collide. FR-392 and TC-1077 are on no `origin/main` path (FR-391 and TC-1076
are the current maximums), and no open PR's file list adds them. #991 takes
FR-393 and TC-1078. `quire validate` passes on both new files. Every field
of `ProbabilisticForm` matches QSpec shared-grammar.md lines 799-825
(quire-specification 4f65ed2): name, `using` profile, `on` subject,
optional `over (parameter)`, `scheduler-clause` (`under ident` or
`under every scheduler {fairness-item}`), the required `activation`, the
optional `confidence` list, and the five `probabilistic-form` alternatives
with their `exact-value` and `expr` thresholds. The defects are in the ACs
and the TC oracle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-392-AC-3 says `P95` with threshold `0.005 s` builds at S2 and "is refused only at S3 (FR-186-AC-2 to FR-186-AC-4)". FR-186-AC-2 says the opposite: "`P95` with threshold `0.005 s` checks and records `5 ms`". If a test follows AC-3 and asserts an S3 refusal, it fails against a correct S3. The case belongs with the cases that build and check, or should become `5 m` (length), the case FR-186-AC-2 does refuse. TC-1077 step 3 copies the case. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:99; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:142; spec/test-cases/TC-1077-s2-builds-the-probabilistic-claim-form.md:23 |
| FND-002 | medium | Two of the five body shapes have no AC. AC-1 covers `Probability` (with a `Temporal` and a `Compare` event), `Quantile` and `LongRunFraction`. Nothing covers `MeanOfFraction` (`mean of (fraction ...)`) or `ExpectedUntil` (`expected accumulate R / elapsed until holds(B)`). Also untested: the `steps` and `elapsed` measures, `weighted by`, `over (p: T)`, `on each` activation, and the `strong` and `each` fairness parts. An S2 that never builds `mean of` or `expected`, or builds them with the wrong parts, passes TC-1077. Add one AC fixture for each missing body shape, plus `over` and `weighted by`. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:62-70,97; spec/test-cases/TC-1077-s2-builds-the-probabilistic-claim-form.md:19-21 |
| FND-003 | medium | TC-1077's oracles for AC-2 and AC-3 are only "it builds". Step 3 expects "all four build", and step 2 expects "`Every` with one fairness item". Neither reads back what was built. An S2 that drops a lone `alpha`, fills in defaults for missing items, or builds the fairness item with the wrong kind or operation passes. The step also does not assert that S2 emits no diagnostic. AC-3's "refused only at S3" is not run at all. Assert `confidence == [(Alpha, 1/100)]`, `FairnessConstraintForm{kind: Some(Weak), granularity: None, operation: Node::tick}`, and that S2 emits no diagnostic. Either run S3 for the refusal or drop that clause from AC-3. | spec/test-cases/TC-1077-s2-builds-the-probabilistic-claim-form.md:22-23,30-31; spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:98-99 |
| FND-004 | low | Several names in Outputs and Behavior do not match qsl-forms. `ClauseForm` does not exist: S2's form enum is `DeclarationForm` (qsl-forms/src/syntax.rs:3136), which FR-311 and FR-102 extend. `FairnessForm` is FR-325's `FairnessConstraintForm`. `ExpectedQuantity` and the horizon form are used but never defined. Behavior does not say that `LeadingTokenKind` gains a `Probabilistic` variant, as FR-311 says for `Random`, `Reward` and `Workload`. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:45,59,68,73-78,82-83 |

## Verdict

Not mergeable as is: fix FND-001, FND-002 and FND-003. The production
mapping itself is exact. The scheduler alternatives, the declared-order
`Vec` of confidence items and the `Option` for `over` all match the grammar.
The `Vec` also keeps repeated confidence items for S3 to see. The partial
confidence case does build at S2: `confidence = 'with', confidence-item,
{ ',', confidence-item }` accepts any non-empty list. The rule that all three
items must be stated is semantic. QSpec places it in FR-407
(`invalid_probabilistic_model`/`partial-confidence`), so leaving it to S3 is
correct. The index row and the FR-186 back-reference are correct. AC-1's
four claims are all of ADR-024 §7. FR-186-AC-1's "five claims" miscounts
that section, which is outside this diff.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | Two of the new AC-3 fixtures are not complete claims. `quantile 0.5 of (steps from holds(A) until holds(B) within 10)` and `… (elapsed … within 10 ms)` stop at the `)`. The production `'quantile', exact-value, 'of', '(', measure, ')', bound-op, expr` needs a `bound-op` and a threshold after it, so S1 refuses the text as written. Each test author has to invent a bound. Add `<= 5` and `<= 5 ms`, or similar. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:125 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 8044d39f2: AC-5 now lists `P95` with threshold `5 m`, the case FR-186-AC-2 refuses "naming the dimension", where it used to list `0.005 s`, which FR-186-AC-2 checks and records as `5 ms`. The other cases in that list agree with FR-186: threshold `1` (FR-186-AC-2, qualitative form; QSpec FR-407 `qualitative-threshold`) and the trace subject (FR-186-AC-3). The lone `alpha` case (FR-186-AC-4; QSpec FR-407-AC-5 `partial-confidence`) is asserted as a build. TC-1077 step 5 follows. |
| FND-002 | fixed | 8044d39f2: AC-3 covers `MeanOfFraction` with `weighted by`, `ExpectedUntil` with `Accumulate(duration)` and `Elapsed`, and the `steps` and `elapsed` measures with `Steps` and `Quantity` horizons. AC-4 covers `over (n: Node)`, `on each`, and the `fair weak` and `fair strong each` items. TC-1077 steps 3 and 4 read each one back. The fixture gap is FND-005. |
| FND-003 | fixed | 8044d39f2: TC-1077 step 5 asserts one confidence item with no default, both `alpha` items in order, no S2 diagnostic for the trace-subject, threshold-`1` and `5 m` claims, and then the FR-186 refusals at S3. Step 4 reads back the kind and granularity of each fairness item. |
| FND-004 | fixed | 8044d39f2 makes four corrections. It writes `DeclarationForm` in place of `ClauseForm`, and `FairnessConstraintForm` in place of `FairnessForm`. It defines `ExpectedQuantity` (`Accumulate(Spanned<Identifier>)` or `Elapsed`) and `HorizonForm::{Steps, Quantity}`. Behavior now says `LeadingTokenKind` gains `Probabilistic`. |
