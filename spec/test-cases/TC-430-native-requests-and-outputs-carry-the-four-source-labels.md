---
id: TC-430
title: "Native run and compile requests and their outputs carry the four source labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
# TC-430: Native run and compile requests and their outputs carry the four source labels

## Description

Verify that the native-v1 request wire requires all four FR-001 labels, that
native-run-result/1 and native-linked-package/1 render them, and that a
two-label request refuses as malformed. This catches a label defaulted by
the lane and a two-label request admitted.

Scope: FR-026-AC-6, FR-027-AC-4.

## Test Procedure

1. Run a native-run/1 request whose program source names `authority`
   `agent-ix`, `identity` `p`, `revision_namespace` `git` and `revision` `1`.
2. Run it again without `authority`, then without `revision_namespace`.
3. Compile a native-compile/1 request with the same four labels, and
   validate the package bytes against the native-linked-package/1 schema.
4. Compile it again without `revision_namespace`.

Tag the tests `#[trace("TC-430", "FR-026-AC-6")]` and
`#[trace("TC-430", "FR-027-AC-4")]`.

## Expected Results

- Step 1: the result renders the program source with the four labels.
- Step 2: each refuses at the request stage with `invalid-request`, exit 20.
- Step 3: the package `source` names the four labels and the schema
  accepts it.
- Step 4: `invalid-request`, exit 20.

## Status

Planned. ADR-013 §7 slice S-4b (QSL-233).
