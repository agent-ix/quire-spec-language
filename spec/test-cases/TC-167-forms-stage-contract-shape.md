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
extent unchanged; and that a leading token with no dispatch entry is
refused. The test uses a test-only leading-token-kind variant and a
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
4. Build a lossless CST (no error or recovery node) whose leading token
   selects no dispatch entry (`record` in M-3a), and call the forms stage on
   it.
5. Using the same test-only leading-token-kind variant and stub production
   function as step 1 (CON-2 forbids a real family production function in
   M-3a, so the stub is the only available producer), build two lossless
   CSTs identical except for their recorded edition — literal values
   `"0-draft"` and `"1-draft"` — call the forms stage on each, and read each
   resulting parsed form's edition back.
6. Using the same test-only variant and stub, build two lossless CSTs whose
   root construct declares a bound or extent, identical except for that
   declared value, call the forms stage on each, and read each resulting
   parsed form's carried bound or extent back.

## Expected Results

- Step 1: the forms stage returns a parsed form built from the CST.
- Step 2: the forms stage returns a refusal with diagnostics; no parsed
  form is returned, even though the same dispatch-table entry exists.
- Step 3: the span accessor exists and returns the originating node's span;
  the identity-reading test fails to compile, because no such accessor
  exists on the parsed-form type.
- Step 4: the CST is refused with `NoDispatchEntry` and no parsed form is
  returned.
- Step 5: the `"0-draft"` CST's parsed form reads back edition `"0-draft"`,
  and the `"1-draft"` CST's parsed form reads back edition `"1-draft"` —
  each form's edition matches the CST it was built from, not a shared
  constant and not the other CST's value.
- Step 6: each parsed form's carried bound or extent matches the CST it was
  built from, not a shared default and not the other CST's value.
