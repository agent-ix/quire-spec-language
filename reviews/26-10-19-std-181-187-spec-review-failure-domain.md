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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new ceiling rule contradicts itself on when it runs. S3 refuses "When S3 reduces a weight to lowest terms and the reduced numerator or denominator would exceed the ... ceiling ... before computing the value". The size of the reduced value is known only after the gcd reduction it is meant to guard, and that reduction runs on an unreduced value of any size: `decimal(1, 4294967295)` has denominator 10^4294967295. A check made before the computation can only bound the value as written (its integers' bit lengths, and ten to the scale). Two implementations will also disagree on a value that is large as written but small reduced, such as `rational(n, n)` with `n` of 5,000 bits, which reduces to 1. Fix: say which value the ceiling measures. Bounding the as-written integers and the scale's expansion before reduction, as QSpec value-accounting charges `decimal.scale-expansion` before expanding, is the form that can run first. Add a vector whose reduced value fits and whose unreduced value does not. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:79-85,142; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:110-114; quire-specification proposals/quire-v1/definitions/value-accounting.md:19-22,75 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 81277341b: "If a claim states a parameter more than once, S3 SHALL refuse it [`partial-confidence`], naming the repeated parameter." AC-4 and TC-621 step 4 add `alpha 0.01, alpha 0.05, beta 0.01, indifference 0.0005`, refused naming `alpha`. |
| FND-002 | fixed | 81277341b states S2's integer domain ("S2 holds each weight as written (FR-393) with unbounded integers") and gives S3 a stated ceiling with a refusal and a vector (`decimal(1, 4294967295)` in FR-185-AC-2 and FR-186-AC-4). FR-393 now points at FR-185's ceiling instead of the undefined "its own limits". The ceiling's code, cause and limit type are new findings: SR-12910 FND-006, and FND-004 here. |
| FND-003 | fixed | 81277341b: "A workload name is a package-wide identifier, so `W` resolves to the one workload of that name." AC-3's vector is now "`NoFault` (on `Health`) with `under Throttle`, where `Throttle` is declared `on Service`, is refused `foreign-workload` naming `Throttle` and `Service`; `NoFault` with `under Steady`, `Steady` declared `on Health`, checks." |
| FND-004 | fixed | 2694bcb7a measures the value as written before reducing: "Before S3 reduces a weight to lowest terms, it SHALL measure the weight as written: the bit length of each written integer, and for `decimal(m, s)` ... the bit length of the denominator `10^s` ... computed from `s` without expanding it." It refuses "before reducing or expanding the value". The vectors cover written-over/reduced-fits (`rational(n, n)` with `n` of 70,000 bits, refused although it reduces to 1), written-fits (`decimal(25, 3)`, admitted as `1/40`) and an over-limit scale (`decimal(1, 4294967295)`). |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The measure gives the bit length of `10^s` as "`⌈s · log₂ 10⌉` bits computed from `s`". The bit length of `10^s` is `⌊s · log₂ 10⌋ + 1`: the two agree for `s >= 1`, but at `s = 0` (`decimal(m, 0)`, `10^0 = 1`) the formula gives 0 instead of 1. The text also does not say the value must be computed exactly. With a large `s` near the bound, a floating-point `s · log₂ 10` can be off by one, so two implementations can disagree at the boundary. Fix: write `⌊s · log₂ 10⌋ + 1`, require exact integer computation (for example by comparing `10^s` against `2^k` through `s` and `k` without expanding), and add a boundary vector exactly at 65,536 and at 65,537 bits. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:81-84,150 |
