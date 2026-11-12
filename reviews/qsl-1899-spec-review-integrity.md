---
id: SR-2690003
title: "integrity review of FR-576 operation-body carrier (PR 2690)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Checked FR-576 for duplication and consistency with
FR-102, FR-103, FR-302, FR-094, FR-494 and the sibling FR-496. FR-576 is not a
duplicate. FR-102 retains the form, FR-103 and FR-302 join it and set
`has_body`, FR-094 builds the node, and FR-494 carries body identity only per
dispatched call candidate. None of them emits a per-operation body record or
its typed absence. There are three consistency defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The key "operation occurrence" is never defined: an operation is a declaration, not an occurrence. The alias rule, under which one operation reached through two model aliases gets two records, diverges from FR-496, the existing per-operation carrier, which keys `operation_effects` rows by `(document, declaring, operation)` and gives one row per document. FR-576 does not reference FR-496, so the two per-operation records would key the same operation differently. | FR-576 Behavior bullet 3; FR-576 Outputs; FR-496 Behavior |
| FND-002 | medium | For a dispatched operation, its body identity is emitted twice, once in the FR-494 dispatch candidate and once in the FR-576 record, and nothing requires the two to be equal. A consumer can read two disagreeing body handles for one operation. | FR-576 Behavior bullet 1; FR-494 Behavior; QSpec FR-322-AC-290 |
| FND-003 | low | Restated obligations. The last Behavior bullet and AC-7 restate FR-302's body type check and FR-302-AC-8, which already asserts no checked package. The second half of AC-6, where renaming the lookup label keeps identities, restates FR-094-AC-5 ("Rebuilding with a different synthesized label gives the same ids"), and that label is synthesized by `check`, not authored. | FR-576-AC-7; FR-576-AC-6; FR-302-AC-8; FR-094-AC-5 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | FR-576 now extends FR-094's body-function scope and FR-302's body check to every operation with `has_body`, but the owning text is unchanged. FR-094 says body functions are built "for one dispatch-eligible operation" by `checked_dispatch`, one per candidate. FR-302, and FR-103's restatement of it, say the body is checked "when dispatch linking first reads a body". No requirement states that an uncalled operation's body node has the same id as when a call reaches it, which FR-576 lines 103-104 and the FR-494 equality in AC-7 rely on. Amend FR-094 (scope and reach-independent identity) and FR-302/FR-103 (check timing) in this PR, and add that id equality to AC-2. | FR-576 Behavior (lines 83-88); FR-576-AC-2; FR-094 "Clause function nodes"; FR-302 Behavior; FR-103 Behavior |
| FND-005 | low | Checking every body changes the outcome for existing units, and FR-576 does not say so. FR-302 refuses a dispatched call in an operation body `ill_typed`/`operator-ineligible` "where the body is checked", so a unit whose uncalled body makes a dispatched call, which compiles today, will now refuse. State this and give it a criterion. | FR-576 Behavior (lines 83-88); FR-302 Inputs |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Round 2 widened FR-302-AC-8 to "every body, reached by a dispatched call or not", but its only test case, TC-1011 ("refuses when dispatch linking first reads the body"), and its only traced test (`unit_compile_checks_a_bodys_arity_and_type_against_its_operation`, `qsl-replay/tests/dispatch_link_compile.rs:681`) cover only a body whose operation is called from a `pre` clause. The matrix will report the widened AC as covered although no test exercises an uncalled body. The uncalled parameter-count mismatch has no criterion anywhere, because FR-576-AC-2 tests only a wrong-typed uncalled body. Add an uncalled-body step to TC-1011, or keep FR-302-AC-8 at its old scope and add the uncalled arity case to FR-576-AC-2. | FR-302-AC-8; TC-1011; FR-576-AC-2 |
| FND-007 | low | FR-576's Description still says FR-302 "checks it when dispatch linking first reads it", that FR-094 "builds a body clause function only for a dispatch-eligible operation", and that an uncalled body "has no body node and is never type-checked today". This PR now amends FR-094 and FR-302, so the Description contradicts them. Reword it as the state before this requirement, or drop those clauses. | FR-576 Description (lines 33-42); FR-094; FR-302 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Behavior lines 72-82 define the operation key as FR-496's `(document, declaring, operation)`, require the same row set as `operation_effects`, and give two aliases of one document one row; FR-496 is now a `depends_on` edge. |
| FND-002 | fixed | Behavior line 92 requires the row's body reference to equal the FR-494 candidate's operation-body identity; AC-7 tests it. |
| FND-003 | fixed | The restating AC-7 and the label-rename half of AC-6 are gone; the body check now defers to FR-302 ("refuses the compile as FR-302 states"), and AC-2 tests only the new uncalled-body case. |
| FND-004 | fixed | Round 2: FR-094 now builds the body function for every operation with `has_body` and states its id does not depend on whether a call reaches it; FR-302 and FR-103 check every body; FR-576-AC-2 asserts the id equality. |
| FND-005 | fixed | Round 2: FR-576 Behavior states that an uncalled body containing a dispatched call now refuses `ill_typed`/`operator-ineligible`, and FR-576-AC-8 tests it with a compiling control. |
