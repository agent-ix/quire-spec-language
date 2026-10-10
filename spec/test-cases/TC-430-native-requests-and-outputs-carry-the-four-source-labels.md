---
id: TC-430
title: "Native run and compile requests and their outputs carry the two source labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
---
# TC-430: Native run and compile requests and their outputs carry the two source labels

## Description

Verify that the native-v1 request wire requires both FR-001 labels, that
native-run-result/1 and native-linked-package/1 render them with the source
digest and no other identity member, and that a request missing a label
refuses as malformed. This catches a label defaulted by the lane. It also verifies that an
extraction body record names a document identity plus a content digest.

Scope: FR-026-AC-6, FR-027-AC-4, FR-031-AC-5.

## Test Procedure

1. Run a native-run/1 request whose program source, model source and
   snapshot selection each name `authority` `agent-ix` and their own
   `identity`.
2. Run it again without the program source's `authority`, then without the
   snapshot selection's `identity`, then with a blank `authority`.
3. Compile a native-compile/1 request with the same two labels, and
   validate the package bytes against the native-linked-package/1 schema.
4. Compile it again without `identity`.
5. Run an extraction request whose body record names `authority`
   `agent-ix`, `identity` `b`, `document` `B` and, as `digest`, the
   raw-artifact digest of the extracted body's bytes. Then run it without
   the body's `authority`, without its `digest`, and with a `digest` of
   other bytes.

Tag the tests `#[trace("TC-430", "FR-026-AC-6")]` and
`#[trace("TC-430", "FR-027-AC-4")]`, and step 5
`#[trace("TC-430", "FR-031-AC-5")]`.

## Expected Results

- Step 1: the result renders each selection with its two labels and its
  digest, and no other identity member.
- Step 2: the first two refuse at the request stage with `invalid-request`,
  exit 20; the blank label refuses with `invalid_source_identity`, cause
  `blank-label`, `label` `authority`.
- Step 5: the first runs and renders the native identity (`agent-ix`, `b`,
  digest) and the formal identity (`B`, digest); the second and third refuse
  with `invalid-request`, exit 20; the fourth refuses
  `stale_dependency`/`content-mismatch`, naming both digests.
- Step 3: the package `source` names the two labels and the source digest,
  and the schema accepts it.
- Step 4: `invalid-request`, exit 20.

Step 2's blank-label refusal renders a byte-0 span in the run output: the native `Diagnostic`'s retained debt (FR-001, "Where an S0 refusal is located"), not this requirement's behaviour.
