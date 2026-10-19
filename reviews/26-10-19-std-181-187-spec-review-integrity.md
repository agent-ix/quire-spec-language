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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The fix adds a new ceiling in FR-185's "Exact values" subsection, also referenced from FR-186 and FR-393: "the check's `max_exact_value_bits` ceiling (default 4096), S3 SHALL refuse the declaration `resource_exhausted` with cause `exact-value-bits-exceeded`, naming the ceiling and the declaration". This breaks QSL's own limit convention and QSpec's cause sets. (1) QSL's S3 ceilings are `CheckingLimits` fields with an `s3.*` setting. Reaching one returns `LimitExceeded`/`StageFailure::Limit`, naming the kind, the configured bound, the counter and the field or setting that raises it (FR-277, FR-096, FR-255, FR-316). FR-277 keeps `resource_exhausted` for the format output budget. `max_exact_value_bits` is in no limits type and has no setting name. (2) QSpec FR-272-AC-16 fixes `resource_exhausted` at exactly five causes (`insufficient-next-charge`, `allocation-failed`, `reservation-size-overflow`, `intake-limit-exceeded`, `input-bytes-exceeded`). `exact-value-bits-exceeded` is not one of them, and unlike `duplicate-reward` and the other new causes it is not marked QSL-proposed pending STD-188. (3) QSpec already has a limit family for this: `quire.value.accounting/v1`'s `integer_bits` and `scale_expansion` counters, charged at `decimal.scale-expansion` and refused `resource_exhausted`/`insufficient-next-charge` with `limit_kind`. QSL also already has the exact-rational budget `max_rational_bits` (FR-203, default 65,536). Fix: make the ceiling a `CheckingLimits` field with an `s3.*` setting whose reach returns `stage_limit_exceeded`/`LimitExceeded` per FR-277 and FR-255, or reuse QSpec's `integer_bits`/`scale_expansion` accounting. If a new cause string is still needed, mark it QSL-proposed and add it to STD-188. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:77-85,142; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:99-101,185; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:111-114; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md; spec/functional/FR-316-check-union-declarations.md:72-77; quire-specification spec/objects/foundation/FR-272-typed-diagnostic-cause.md:87,193; quire-specification proposals/quire-v1/definitions/value-accounting.md:8-14,75 |
| FND-007 | low | Two preambles now disagree with what follows them. FR-185's opens with "Every refusal below is an `invalid_probabilistic_model` diagnostic", but its new "Exact values" subsection refuses `resource_exhausted`. FR-186's lists `fairness-under-workload` among the causes of "Each refusal below that carries a bracketed string", but after the fix no refusal below carries it. Fix: scope FR-185's sentence to the bracketed refusals, and move the `fairness-under-workload` mention out of FR-186's cause list into the PM-7 bullet, which already explains it. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:65-67,79-85; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:84-88,117-124 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 81277341b: the FR-185 preamble now says "Every cause except `duplicate-reward` is in QSpec FR-272's closed set ... `duplicate-reward` is a QSL-proposed cause that QSpec does not yet define; it is requested of QSpec in STD-188", and the reward bullet adds "(QSL-proposed, STD-188)". |
| FND-002 | fixed | 81277341b: the FR-186 preamble names `invalid-confidence` and `quantile-over-fraction` as QSL-proposed causes requested in STD-188. Both bullets add "(QSL-proposed, STD-188)", and AC-2 adds "(QSL-proposed)". |
| FND-003 | fixed | 81277341b: "A weight is an `exact-value`, and `exact-number` admits a signed integer, so `rational(-1, 1)` parses and S3 refuses it `invalid-weight`. A bare `-1` is not an `exact-value`, so S1 refuses it `invalid_syntax`." |
| FND-004 | fixed | 81277341b rewrites ADR-024 §7.1 to §7.3 in grammar form: `random Node::tick(fault) ~ {…};`, `random Server::attempt(d) ~ {…};`, `reward duration on Server::attempt = d;` and `long_run_fraction holds(v.up) >= 0.999`. They match QSpec TC-351's declarations. |
| FND-005 | fixed | 81277341b lists the `exact-value` positions exactly ("a probability, mean-of-fraction or long-run threshold, a quantile level, a confidence parameter and the magnitude of a `quantity-literal`") and adds "a quantity threshold such as `5 ms` or `0.005 s` is a `quantity-literal` and keeps its decimal magnitude." |
