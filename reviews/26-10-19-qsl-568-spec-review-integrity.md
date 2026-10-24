---
id: SR-12897
title: "Integrity review of PR #990: FR-392 against QSpec shared grammar, FR-186, FR-195 and the S2 siblings"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@a18d54884; FR-392 Description, Outputs, Behavior, AC-1 to AC-3; FR-186 Outputs and AC-1 to AC-4; FR-195 Outputs and S3 behavior; FR-311, FR-310, FR-308, FR-325, FR-102-AC-4; FR-253; ADR-024 §7; agent-ix/quire-specification@4f65ed2 proposals/quire-v1/shared-grammar.md lines 153-155 and 799-894, FR-407 lines 108 and 128-157"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-392
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
---
# SR-12897: integrity review of QSL-568 (PR #990)

## Summary

Ticket: QSL-568. This review checks FR-392 against four things. The first is
merged QSpec, read only from quire-specification origin/main 4f65ed2. The
second is the S3 requirements that consume the form: FR-186, and FR-195,
which extends FR-186's `ProbForm`. The third is the S2 sibling conventions
in FR-311, FR-310, FR-308, FR-325 and FR-102. The fourth is the repo's
convention for refusal codes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-392 hands every S3 check to FR-186: "so S3 can check it (FR-186)", "unchecked for S3 (FR-186)", and the only S3 `traces_to` edge. FR-186 does not check several parts of the form. Its `ProbForm` has no `expected` variant, and its `BoundedMeasure` has no `elapsed`. FR-195 is the S3 requirement that admits `expected accumulate/elapsed until` as `ProbForm::ExpectedReward` and the fairness set under `every scheduler`. FR-204 gives `expected elapsed` its time reward. So for `ExpectedUntil`, `Every{fairness}` and `elapsed`, a reader following FR-392's references finds no check, and FR-186 does not refuse them either. Add `traces_to` FR-195 (and FR-204 for `elapsed`), and name which S3 requirement checks each part. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:12-13,26-27,89-91; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:56-79; spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md |
| FND-002 | low | FR-392 never says that S2 resolves no name. The profile `ev`, the subject alias, the workload `Steady`, the `accumulate` reward, the `weighted by` ident and the fairness operation all stay as written for S3. FR-310 states this explicitly ("S2 SHALL build no form that resolves a name"). FR-392 says it only for the subject's kind. AC-2 also does not cite FR-102-AC-4's `dispatch_entry_is_a_single_thin_call` check for the new arm. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:82-91,98 |

## Verdict

The grammar mapping is exact. The field order, the optionality, both
scheduler alternatives, `fairness-item` (kind required, `each` optional),
the `confidence-item` keys and all five `probabilistic-form` shapes match
lines 799-825, and `activation` matches lines 153-155. FR-392 follows
FR-311's pattern: the same Description template, `Spanned` parts, a dispatch
AC, and a "builds at S2, refused at S3" AC. ADR-024 §7 supplies the AC-1
fixtures, spelled in the grammar's form (`long_run_fraction` for §7.3's
`long-run fraction`). FR-312 is the clause run request reader, not an S2
form, so it is not a sibling here.

Refusal codes: the author recommends no stable refusal codes for FR-185 and
FR-186. The repo convention does not support that. About 70 QSL FRs name
QSpec `code`/`subcode` pairs, and the sibling FR-253 names
`invalid_probabilistic_model`/`duplicate-distribution` and
`delay-on-untimed-model`. QSpec defines that code's subcodes for exactly
the FR-185 and FR-186 refusals: `partial-confidence`,
`qualitative-threshold`, `fairness-under-workload`, `foreign-workload`,
`incomplete-workload`, `invalid-weight`, `negative-reward` and others
(FR-407-AC-5). FR-185 and FR-186 should cite them. That change is outside
this PR's diff and does not block it. It belongs in a follow-up on the
FR-185/FR-186 owner ticket.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new allocation sentence says "FR-204 checks `expected elapsed` and the `elapsed` measure". FR-204 does not check the `elapsed` measure. It is EN-5 engine admission: it admits `Reach`, deadline events and `expected elapsed until`, and any other shape settles `Unsupported(TimedFormShape)`. A quantile over `elapsed`, one of AC-3's own fixtures, is refused at S3 by FR-186 ("A quantile SHALL be over an `accumulate` or `steps` measure"). An `elapsed` measure inside a `probability` `Compare` event has no S3 rule in FR-186 at all, because FR-186's `BoundedMeasure` has no `Elapsed`. Name FR-186's quantile rule for a quantile over `elapsed`, and record the event case as an FR-186 gap. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:111-117; spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:70-73; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:119 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 8044d39f2 adds `traces_to` edges to FR-195 and FR-204, and Description and Dependencies now name all three. Behavior names an S3 requirement for each body: FR-186 for `probability`, `quantile`, `mean of` and `long_run_fraction`; FR-195 for `expected accumulate` and fairness under `every scheduler`; FR-204 for `expected elapsed`. Part of the `elapsed` wording is wrong: FND-003. |
| FND-002 | fixed | 8044d39f2: "S2 SHALL resolve no name (FR-310): the profile, subject, workload, rewards, predicates and fairness operations stay as written". AC-2 asserts the names are unresolved and that dispatch makes one call (FR-102-AC-4). |
