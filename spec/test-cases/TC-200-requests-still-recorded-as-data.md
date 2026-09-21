---
id: TC-200
title: "Requests are still recorded as data after negotiation removal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-077
    type: verifies
---
# TC-200: Requests are still recorded as data after negotiation removal

## Description

Verify that removing backend negotiation from `requests::report` does not
remove request recording: every admitted requested pair (declaration,
capability kind, `required` flag, request index) still appears in the
report's output. Scope: FR-077-AC-3.

Catches an over-aggressive removal that deletes the recording logic along
with the negotiation logic (since both previously lived in the same
function), leaving the report empty or partial even though admission still
produced the pairs.

## Test Procedure

1. Admit a set of at least four requested capability pairs spanning at
   least three distinct capability kinds and both `required` values (true
   and false).
2. Call `requests::report` over the admitted set with no backend argument.
3. Compare the report's recorded pairs (declaration, capability kind,
   `required` flag, request index) against the input set.

## Expected Results

The report contains exactly the four input pairs, each with its original
declaration, capability kind, `required` flag and request index preserved;
none is dropped, merged, or reordered relative to its original request
index.
