---
id: SR-12230
title: "Gap analysis of quire-spec-language PR #1107: FR-391 population names in state clauses and holds atoms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@50d047195253cf4ddcbca445656d15364d7986d8; FR-391-AC-1..AC-8, TC-1076 rows 1-10, FR-139-AC-2 as narrowed by FR-391, FR-106 required population (FR-391 clause), FR-155-AC-2 against qsl-semantics/tests/it/named_population.rs, qsl-eval/tests/it/named_population.rs, qsl-semantics/tests/it/refinement_populations.rs, qsl-eval/tests/it/footprints.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1107

## Summary

Ticket: QSL-541 (FR-391 follow-up). This is a planless gap analysis, so plan
completion was not assessed. It is scoped to the PR diff. The spec text was
read from `spec/` on `origin/main`, merged via PR #1077. Each `#[trace]` was
read against its AC.

Covered and sound:

- **AC-1.** Precondition kind, `Population(None)` operand, unbounded
  `Set<Reference<Counter>>` result, and every read at `pre`. The invariant
  reads at `current`.
- **AC-3.** All four value uses refuse `ill_typed`/`operator-ineligible`.
  `M::Counter` refuses `missing-name`.
- **AC-4.** `Population(Some(3))` operand and `[0,3]` result. The bounded
  `counters` leaves the unbounded domains, with bound `B` linked. With
  `archive`, the clause is ambiguous at `on` whichever population it names.
  `widgets` and `counters` are both domains for `Grow`.
- **AC-5.** Decides over exactly a, b and c (the `Size` and `Room`
  high/low pair). A `counters` that is not complete is `Incomplete` with
  `incomplete_population`/`incomplete-scope`. `Other` gives `incomplete-closure`.
- **AC-6.** Two keys and two identities. `AnyMembership{counters}` only for
  `A` and `{archive}` only for `B`, beside the receiver `Membership`.
  Emission refuses `unsupported_construct`/`not-yet-implemented`.
- **FR-139-AC-2.** The `seq` half is retagged and the `count` in an
  invariant is now admitted.
- **FR-155-AC-2.** The quantification clause over `counters` reads
  `AnyField{v}` and `AnyMembership{counters}`. Together with the existing
  `footprints.rs` test, the AC's halves are covered.

Every test here can fail: each asserts the specific code and cause, location,
type, domain set or boolean, against a high/low snapshot pair.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-391-AC-7 tag over-claims. AC-7 requires each of `P`, `Q`, `Cap` and the temporal `C` to return `Incomplete` over a `counters` that is not complete. The test checks this only for `GrowQ`, a precondition. The temporal atom over an incomplete `counters` is not tested, and it actually decides `true` (SR-12229 FND-002). Add a not-complete case per clause, the `holds` atom included. | qsl-eval/tests/it/named_population.rs:375-458 |
| FND-002 | medium | The FR-391-AC-8 tag covers only the function-body and `seq` halves. Its "type-level invariant of an object type refuses `missing-name` at `counters`" half has no test (a declared gap). Add the case, or drop the tag until it exists. | qsl-semantics/tests/it/named_population.rs:389-410 |
| FND-003 | medium | The AC-2, AC-3 and AC-8 tests assert the refusal span as the whole name (`M::nothing`, `M::Counter`, `M::counters`). Each AC says the second segment (`nothing`, `Counter`, `counters`), so the tests encode a deviation rather than the criterion (SR-12229 FND-005). | qsl-semantics/tests/it/named_population.rs:179, qsl-semantics/tests/it/named_population.rs:215, qsl-semantics/tests/it/named_population.rs:400 |
| FND-004 | low | TC-1076 row 7's "one `population.visit` charge per member" is not asserted anywhere (a declared gap). AC-5's own text does not name the charge, so its tag stands, but the TC row is not fully executed. A meter-delta assertion over a, b and c would close it. | qsl-eval/tests/it/named_population.rs:250-284 |
| FND-005 | low | `lookup<T>(M::P, r)` has no test at S3 or S6a. FR-391 Inputs, Typing and Evaluation name it beside `allInstances`, and the untested code has a coverage gap (SR-12229 FND-003). No AC names `lookup` explicitly, so this is a coverage gap, not a tag over-claim. | qsl-eval/src/value/expression/evaluate.rs:2428-2488 |
| FND-006 | low | Emission refusal is tested only for a state-clause unit. A unit whose only population reference is in a `holds` atom was confirmed by a scratch probe to refuse, but no committed test pins it. The same goes for the empty-but-complete population that SR-12229 FND-001 found faulting. | qsl-eval/tests/it/named_population.rs:357-367 |

## Verdict

The gap analysis has no majors of its own. FND-001's untested half is
SR-12229 FND-002, which blocks in the code review. Tags on AC-7 and AC-8
over-claim (mediums), and the AC-2, AC-3 and AC-8 location assertions encode
a spec deviation (medium). FND-004 to FND-006 are lows. AC-1, AC-3, AC-4,
AC-5, AC-6, the FR-139-AC-2 retag and the FR-155-AC-2 tag are backed by
tests that can fail.
