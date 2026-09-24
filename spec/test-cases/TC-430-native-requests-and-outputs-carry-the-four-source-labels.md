---
id: TC-430
title: "Native run and compile requests and their outputs carry the four source labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
---
# TC-430: Native run and compile requests and their outputs carry the four source labels

## Description

Verify that the native-v1 request wire requires all four FR-001 labels, that
native-run-result/1 and native-linked-package/1 render them, and that a
two-label request refuses as malformed. This catches a label defaulted by
the lane and a two-label request admitted.

Scope: FR-026-AC-6, FR-027-AC-4, FR-031-AC-5.

## Test Procedure

1. Run a native-run/1 request whose program source, model source and
   snapshot selection each name `authority` `agent-ix`, their own
   `identity`, `revision_namespace` `git` and `revision` `1`.
2. Run it again without the program source's `authority`, then without the
   snapshot selection's `revision_namespace`, then with a blank `authority`.
3. Compile a native-compile/1 request with the same four labels, and
   validate the package bytes against the native-linked-package/1 schema.
4. Compile it again without `revision_namespace`.
5. Run an extraction request whose body record names the four labels, then
   one without the body's `authority`.

Tag the tests `#[trace("TC-430", "FR-026-AC-6")]` and
`#[trace("TC-430", "FR-027-AC-4")]`, and step 5
`#[trace("TC-430", "FR-031-AC-5")]`.

## Expected Results

- Step 1: the result renders each selection with its four labels.
- Step 2: the first two refuse at the request stage with `invalid-request`,
  exit 20; the blank label refuses with `invalid_source_identity`.
- Step 5: the first runs; the second refuses with `invalid-request`, exit 20.
- Step 3: the package `source` names the four labels and the schema
  accepts it.
- Step 4: `invalid-request`, exit 20.

## Status

Partial. ADR-013 §7 slice S-4b (QSL-233). Steps 2 to 5 pass locally. Step 1 passes with the model source's revision value `draft:1`, not `1`: the program's model import pins the model artifact, which binds that label.
