---
id: SR-12911
title: "Failure-domain review of PR #1001: confidence repeats, ExactValue limits and workload name resolution"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@739cd6354; FR-186 confidence and scheduler Behavior, AC-3, AC-4; FR-392 confidence field; FR-393 ExactValue; FR-311 ExactValue note; FR-185 weight Behavior; QSpec FR-407 confidence parameters"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
---

## Summary

Tickets: STD-181, STD-187. PR quire-spec-language#1001. This review looks
for inputs that the edited refusal rules and the new "held as written"
ExactValue leave with no stated outcome.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-186 now says: "When it states one or two, S3 SHALL refuse it [`partial-confidence`]." QSpec's cause is wider, "a set that omits or repeats one refuses with `invalid_probabilistic_model`/`partial-confidence`", and the shared grammar's `confidence` clause allows repeats syntactically. FR-392 keeps every item, in declared order, in `confidence: Vec<…>`, so `with alpha 0.01, alpha 0.05, beta 0.01, indifference 0.0005` reaches S3 stating all three, with `alpha` twice. FR-186 gives it no outcome, so one implementation can admit it with either `alpha` while another refuses it. Fix: add "or repeats one" to the rule, and add a repeated-`alpha` vector to AC-4 and TC-621 step 4. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:144-145,161; spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:60; quire-specification proposals/quire-v1/shared-grammar.md:806-807,878-881 |
| FND-002 | medium | FR-393 now says "S2 neither reduces it nor raises ten to a scale, so a scale of any size costs nothing at S2 … S3 reduces it to a numerator and denominator in lowest terms under its own limits (FR-185, FR-186)". Neither FR-185 nor FR-186 states a limit, or a refusal when one is exceeded. FR-393 also does not say how wide the "two integers" are, which matters because `uint` and `signed-int` are unbounded in the grammar. So `decimal(1, 4294967295)`, or a `uint` with more digits than u64 holds, has no stated outcome at either stage: S3 may compute ten to that scale, overflow or refuse, each with a different diagnostic. Fix: state the integer domain S2 holds, together with S2's outcome past it, and give S3's reduction limit and its refusal code and cause in FR-185 and FR-186. If no limit is intended, drop "under its own limits". | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:105-114; spec/functional/FR-311-build-random-reward-and-workload-forms-at-s2.md:61-63; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:73-82; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:129-148 |
| FND-003 | low | FR-186-AC-3's new vector "A claim `under Steady` naming a workload of another model is refused `foreign-workload`" names neither the claim nor whose `Steady` it means. ADR-024 §7 declares a workload `Steady` on each of `Health`, `Service` and `Avail`, and FR-186 does not say how `under ident` resolves when two models in scope declare a workload of the same name. Read one way, `NoFault under Steady` resolves to `Health`'s own `Steady` and checks. Read the other way, it is ambiguous. Fix: give the vector an unambiguous name, for example "`NoFault` (on `Health`) `under Burst`, with `Burst` declared on `Service`", and state how a workload name resolves (to the subject model's workload of that name, or one package-wide name). | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:99-100,108-109,160; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:223,266,307 |

## Verdict

The rest of the edited refusals cover their failure domain:

- Workload refusals cover omission, an unknown operation, a duplicate and
  a non-positive weight, as QSpec FR-405 does.
- Reward refusals cover a state read, a non-integer, non-quantity type and
  a duplicate name; non-negativity is left to FR-187, as before.
- The fairness-under-workload case is unreachable from source by grammar,
  and FR-186 now says so. Its testability is SR-12909 FND-001.
- The unbounded-event and trace-subject refusals still have no cause.
  QSpec has none to cite, and STD-188 tracks it, so it is not a finding
  against this PR.

Not mergeable as is: FND-001 and FND-002 (medium) should be fixed in this
PR. No high.
