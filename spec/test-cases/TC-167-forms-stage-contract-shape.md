---
id: TC-167
title: "The S2 forms stage refuses on a recovering CST and mints no identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: verifies
---
# TC-167: The S2 forms stage refuses on a recovering CST and mints no identity

## Description

Verify that the `forms` core, together with a family form builder, produces
parsed forms from a lossless CST; that it refuses rather than partially
building a form when the CST carries an error or recovery node; that a
parsed form carries only the span of its originating CST node, no
check-time-minted identity, and its CST's edition and any declared bound or
extent unchanged; and that each entry in the dispatch table is a thin,
single-call seam. The test uses a test-only leading-token-kind variant and a
stub production function so the seam is exercised without depending on any
family having migrated onto S2 forms yet. Scope: FR-067-AC-1, FR-067-AC-2,
FR-067-AC-3, FR-067-AC-7, FR-067-AC-8.

## Test Procedure

1. Wire a test-only leading-token-kind variant and a stub production
   function into the forms core's dispatch table. Build a lossless CST (no
   error or recovery node) whose root construct's leading token is that
   test-only variant, and call the forms stage on it.
2. Take the same CST, mark one of its nodes as a recovery node, and call the
   forms stage on it again.
3. Inspect the parsed-form type returned in step 1 for an accessor that
   exposes the originating CST node's span. Separately, attempt to compile
   a test that reads a semantic node identity, a declaration key, or any
   other check-time-minted value off that same parsed form.
4. Inspect the dispatch-table entry for the test-only variant: count its
   calls into the stub production function, and check for any conditional,
   lookup or loop in the entry outside that one call.
5. Build two lossless CSTs identical except for their recorded edition (the
   existing top-level and `composed` edition constants), call the forms
   stage on each, and read each resulting parsed form's edition back.
6. Build two lossless CSTs whose root construct declares a bound or extent,
   identical except for that declared value, call the forms stage on each,
   and read each resulting parsed form's carried bound or extent back.

## Expected Results

- Step 1: the forms stage returns a parsed form built from the CST.
- Step 2: the forms stage returns a refusal with diagnostics; no parsed
  form is returned, even though the same dispatch-table entry exists.
- Step 3: the span accessor exists and returns the originating node's span;
  the identity-reading test fails to compile, because no such accessor
  exists on the parsed-form type.
- Step 4: the entry makes exactly one call into the stub production
  function and holds no other conditional, lookup or loop.
- Step 5: the two parsed forms' editions differ, matching the two CSTs'
  recorded editions; neither reads back a shared constant.
- Step 6: the two parsed forms' carried bound or extent values differ,
  matching the two CSTs' declared values; neither reads back a shared
  default.
