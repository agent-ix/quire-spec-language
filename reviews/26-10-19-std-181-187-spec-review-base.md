---
id: SR-12909
title: "Base review of PR #1001: FR-185/FR-186 probabilistic refusal causes, fixtures and TC-620/TC-621"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@739cd6354; FR-185 Behavior and AC-1 to AC-4; FR-186 Behavior and AC-1 to AC-4; FR-311 ExactValue note; FR-392 EventForm and form notes; FR-393 ExactValue; ADR-024 §7.3 Monthly; TC-620; TC-621"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-620
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-621
    type: reviews
---

## Summary

Tickets: STD-181, STD-187. PR quire-spec-language#1001, diff
`origin/main...739cd6354` only. This is the base checklist: ID formats, no
new or colliding ids, AC-to-TC coverage, and whether TC-620 and TC-621 agree
with the edited ACs. QSpec (`~/dev/quire-specification@a2230e1`) is
normative for grammar and causes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-186 adds the SHALL "Any other producer of a scheduler that pairs a workload with a fairness set SHALL be refused [`fairness-under-workload`], naming PM-7", but no AC and no TC-621 step exercises it. AC-3's only vector for this case is `under Steady fair weak Node::tick`, which FR-186 itself says S1 refuses `invalid_syntax`. The text also names no such producer, so no test can be written. STD-181 asked for a vector per refusal. Fix: either name the producer (for example, a hand-built `SchedulerForm` or an IR import) and add a vector, or drop the SHALL and record the cause as unreachable from QSL source until STD-188 settles the grammar. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:101-107,160; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md:23,32 |
| FND-002 | medium | FR-185-AC-2 drops the old negative-weight S3 vector and replaces it with "A weight `-1` is not an `exact-value`, so S1 refuses it". That holds for the bare token `-1`. But `exact-number = 'rational', '(', signed-int, ',', signed-int, ')' \| 'decimal', '(', signed-int, ',', uint, ')'` with `signed-int = [ '-' ], uint`, so `rational(-1, 1)` and `decimal(-1, 0)` are negative weights that parse and reach S3. No AC now covers a negative weight being refused `invalid-weight` at S3, which is the case the cause exists for (QSpec FR-405: "a weight that is not a positive exact rational"). Fix: add `rational(-1, 1)` (`invalid-weight`) to AC-2, keep the bare `-1` vector as the S1 case, and add the matching TC-620 step 2 result. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:119; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md:22,31; quire-specification proposals/quire-v1/shared-grammar.md:149,367-368 |
| FND-003 | low | TC-620 step 4 reads "`reward cost on Server::request = 2` on `request`". The trailing "on `request`" repeats the `on Server::request` the fixture now carries. Drop it. | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md:24 |

## Verdict

Clean where the brief asked:

- **No id changes or collisions.** The PR adds no FR, TC, AC or ADR id. It
  edits FR-185-AC-2 to AC-4, FR-186-AC-1 to AC-4, TC-620 and TC-621 in
  place, and `quire validate --scope .` on the eight changed files reports
  no errors (module-registration warnings only).
- **"Four claims".** ADR-024 §7 declares exactly `NoFault` (§7.1), `P95`
  (§7.2), `LongRun` and `Monthly` (§7.3). FR-186-AC-1's "four claims" and
  TC-621 step 1's "the four claims" are correct.
- **TC-620 and TC-621 agree with the ACs.** TC-620 steps 2 to 4 and their
  expected results carry AC-2 to AC-4's causes, including the S1
  `invalid_syntax` for `-1` and `duplicate-reward`. TC-621 steps 2 to 4
  list each AC-2 to AC-4 vector, and their expected results give the same
  codes, in order: `ill_typed`/`unit-mismatch`, `qualitative-threshold`,
  `quantile-over-fraction`; `foreign-workload`, fairness under every
  scheduler checks, S1 `invalid_syntax`; `invalid-confidence`,
  `partial-confidence`, `confidence: None`.
- **Fixtures parse.** `reward duration on Server::attempt = …`,
  `reward ok on Server::attempt = (outcome = Ok)`, `reward cost on
  Server::request = 2` match `reward-decl`. `quantile 0.95 of (fraction
  holds(v.up) over [0, 10]) <= rational(1, 2)` matches the `quantile`
  form, with `fraction` a `measure` and `rational(1, 2)` an `expr`.
  `Monthly`'s `fraction … >= rational(999, 1000)` matches `event = measure,
  bound-op, expr`. `under every scheduler fair weak Node::tick` matches
  `fairness-item`. `under Steady fair weak Node::tick` fails, since
  `scheduler-clause = 'under', ident` is followed by `activation = 'on', …`.

Not mergeable as is: FND-001 and FND-002 (medium) should be fixed in this
PR. No high.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 81277341b removes the untestable SHALL. FR-186 now reads "QSL has no producer of a workload scheduler with a fairness set, so S3 has no `fairness-under-workload` check; QSpec FR-407 defines that cause, and STD-188 asks QSpec to settle whether the grammar or the cause changes." AC-3 keeps the S1 `invalid_syntax` vector. |
| FND-002 | fixed | 81277341b adds "a weight `rational(-1, 1)` (`invalid-weight`)" to FR-185-AC-2 and the matching TC-620 step 2 result, and keeps the bare `-1` as the S1 case. |
| FND-003 | fixed | 81277341b: TC-620 step 4 now reads "Check each reward variant of FR-185-AC-4, and `reward cost on Server::request = 2`." |
| FND-004 | fixed | fdf76e209 moves the bound vectors out of AC-2's "refused ... carrying the cause" list into their own sentence ("Exact-value bounds, each as the weight of `Server::request`: ... are admitted; ... are refused with the limit outcome"). TC-620 step 2 now lists them with separators. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-185-AC-2 opens with "Each of these random-parameter variants of `Service` is refused at S3 with a diagnostic naming the parameter and carrying the cause", but its list now includes `decimal(25, 3)` "(admitted, `1/40`)" and two rational-bits limit outcomes, which carry no `invalid_probabilistic_model` cause. TC-620 step 2 also lost a separator: "`decimal(1, 4294967295)` `rational(n, n)` with `n` of 70,000 bits". Fix: move the admitted case and the limit outcomes into their own sentence of AC-2, and add "and" to TC-620 step 2. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:150; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md:22 |
