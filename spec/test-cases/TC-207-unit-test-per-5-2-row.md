---
id: TC-207
title: "One unit test exists and passes per ADR-012 §5.2 row"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: verifies
---
# TC-207: One unit test exists and passes per ADR-012 §5.2 row

## Description

Verify that each of ADR-012 §5.2's five explicit registry failure rows has
its own unit test, and that each test actually fails if its row's
documented outcome is not produced. Scope: FR-080-AC-4.

Catches a single combined test that exercises all five cases in one
function without independent assertions per case (so a regression in one
row's behavior can be masked by the others still passing, and coverage
tooling cannot show which row broke), and a test that asserts only "the
call did not panic" rather than the specific documented outcome (refusal
cause, unknown-backend marker, empty candidate set, or invalid-request),
which would stay green even if the wrong outcome were produced.

## Test Procedure

1. Locate the unit test (or tests) covering each of the five §5.2 rows:
   (a) duplicate `BackendId`; (b) a capability kind outside the vocabulary;
   (c) an unregistered named `BackendId`; (d) a capability kind no
   registrant advertises; (e) more than one registrant matches with no
   named backend.
2. Confirm each row has at least one test whose assertions check that
   row's specific documented outcome (the cause, marker, or set shape), not
   merely absence of a panic.
3. For each row's test, mutate the implementation to produce a different,
   wrong outcome for that row only (for example, silently accepting a
   duplicate `BackendId` instead of refusing it) and confirm that row's
   test fails while the other four rows' tests still pass.

## Expected Results

Step 1 finds five distinct tests (or five independently-asserting cases).
Step 2 confirms each asserts the specific documented outcome. Step 3 shows
each test is sensitive to its own row's regression and independent of the
other four.
