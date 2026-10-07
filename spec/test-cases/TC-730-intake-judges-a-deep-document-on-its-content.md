---
id: TC-730
title: "Intake judges a deep package document on its content"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: verifies
---
# TC-730: Intake judges a deep package document on its content

## Description

Verify that semantic-IR intake reads once through the shared reader and
refuses or admits on content, never on depth.

Scope: FR-260-AC-1, FR-260-AC-2, FR-260-AC-5.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. With `intake.input_bytes` raised to fit, admit a package document whose
   declarations are valid and which holds an array nested 100,000 deep at a
   member the semantic-IR schema does not admit.
2. Admit every corpus package FR-056's tests admit, and read each digest.
3. Read a package document holding `"\udc00"` in a string through
   `PackageDocument::parse`, intake's one read. FR-154 admission digests
   such bytes raw (FR-056) and does not reach this refusal.
4. Build both derived views of a reader tree nested 100,000 deep, as arrays
   and as objects, and drop the `serde_json` view. Drop a
   `PackageDocument` holding a 100,000-deep `serde_json` view. Run a view
   build whose leaf conversion fails after a 100,000-deep sibling has
   finished. Build a shallow document's views and compare them with what
   `serde_json` reads and with the expected `Json`.

Tag the tests `#[trace("TC-730", "FR-260-AC-1")]`, `#[trace("TC-730", "FR-260-AC-2")]`, `#[trace("TC-730", "FR-260-AC-5")]`.

## Expected Results

- Step 1: intake ends with no declaration and retains the
  `agent-ix-semantic-ir` reader's diagnostic for that member with its IR
  node, artifact id and span (FR-056's reader-refusal rule); no
  `resource_exhausted` cause, and no intake outcome names a depth.
- Step 2: each is admitted with the `sha256-jcs` digest it had.
- Step 3: refused `invalid_model_binding`/`malformed-declaration` at `$`,
  carrying the escape's byte offset.
- Step 4: both views build and the `serde_json` view drops with no stack
  overflow; the failing leaf's failure is returned; the shallow views equal
  the expected trees, members in document order and numbers by lexeme.

## Status

Steps 1 to 4 are implemented. Intake has no depth limit: a document nested
100,000 deep is read, judged by the semantic-IR reader and dropped on a
512 KiB stack.
