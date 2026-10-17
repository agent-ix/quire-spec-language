---
id: SR-12025
title: "Spec review of quire-spec-language PR #917: FR-318 clause record of a case"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@580f089a7dad0610431da5bc98a7aa090cecffea; spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-318-check-case-expressions-and-exhaustiveness.md, spec/test-cases/TC-824-a-case-checks-under-each-clause-kind-and-adds-no-requirement-record.md"
review_set: subset
---
# Spec review of quire-spec-language PR #917

## Summary

Ticket: QSL-502. Spec review (base, with the integrity consistency check)
over the PR diff: the FR-057 claim-form row for a clause containing `case`,
the FR-318 "Requirements and clause kinds" sentence, FR-318-AC-5 and TC-824
step 2. Checked against FR-057 "Kind applicability", FR-062 "Requirement
records of a value function", FR-104 (state clause records), FR-057-AC-10,
FR-320, FR-323, FR-324 and ADR-012.

The state-clause half is correct and agrees with FR-104: one
`operation-contract` record per clause, keyed by its `claim` occurrence, and
a nested `case` adds nothing. The predicate half adds records that no other
requirement defines and that FR-062 excludes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-318-AC-5 and TC-824 step 2 require a `decreases` measure containing the `case` to yield one `value-validity` record. FR-062 says an occurrence in a function's `decreases` measure is not in the body and is no claim site, and `check` discharges the measure at admission (FR-146 termination, `MeasureObligation`). No requirement defines a record, key or claim for a measure, so no conforming implementation can pass this AC. Either define the measure record in FR-062 (key, claim, extent), or drop `decreases` from the FR-057 row, FR-318 and AC-5 and expect no record for the measure. | FR-318:123; TC-824:19-29; FR-057:177; FR-062:215-219 |
| FND-002 | high | AC-5 says a pure Boolean predicate clause containing the `case` yields one `value-validity` record, but FR-062 records a predicate (a `Value` function, FR-091-AC-30 kind `Predicate`) once per scalar operation occurrence, not once per clause. AC-1's `case` holds three such occurrences (`r * r`, `* 3`, `w * h`), so a predicate over it yields three. The same AC says a function containing it yields no record for the `case`, so it gives two record counts for one family. No requirement defines a clause-level record for a "pure Boolean predicate clause", and TC-824 never checks this branch. | FR-318:123; FR-318:99-103; FR-057:177; FR-062:213-219 |
| FND-003 | medium | ADR-012 still says unconditionally that a clause containing `case` requests `value-validity`: "A clause that contains a `case` expression requests `value-validity`" (line 1441), and the S3 requirements row says "records the clause's own kind, `value-validity`" (line 1579). Both contradict the new state-clause rule (`operation-contract`, FR-104). | ADR-012:1441-1443; ADR-012:1579 |

## Verdict

FIX. The state-clause rule matches FR-104 and FR-057 "Kind applicability".
The FR-057 row, the FR-318 sentence, AC-5 and TC-824 agree on it, and nothing
else in FR-104, FR-057, FR-323 or FR-324 says a `case` under a state clause
yields `value-validity`. FR-057-AC-10 and FR-320 do not conflict. FND-001 and
FND-002 add predicate-family records that conflict with FR-062's
per-occurrence record model. FND-003 is a stale ADR statement of the old
rule.
