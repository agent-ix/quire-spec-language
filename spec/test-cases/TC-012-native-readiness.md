---
id: TC-012
title: "Located admitted native grammar"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: verifies
---

## Description

Located admitted native grammar. Type: Integration; priority P1. Traces: FR-002-AC-1, FR-002-AC-2, FR-002-AC-4, FR-002-AC-5, FR-002-AC-7, NFR-001-M-2, NFR-001-M-3, NFR-001-M-4.

## Test Procedure

Run the `tests/fixtures/spine-compile.native` fixture through the `parse` operation, and the precedence, chained-comparison, grouping/name locus, node-limit and joined 512 KiB stack cases through `qsl_cst::parse`. Serialize the operation's result for the fixture and for a unit holding `a < b < c` through `OutcomeDocument::from_parse`, and map each document's category through FR-285. In the no-default-features build `make ci-clean-build` makes, run the fixture case.

## Expected Results

The fixture parses admissible with its exact bytes; admitted examples parse with original spans; chained comparisons are `invalid_syntax` at the second `<`, and an exhausted node limit is `stage_limit_exceeded` naming `s1.nodes`. The fixture's document has category `success` and `last_stage` `S2`, exit code 0; the chained comparison's has category `refusal`, `last_stage` `S1` and one `invalid_syntax` diagnostic at the second `<`, exit code 20; the no-default-features run gives `success`. Flat 20000-operator chains complete on the selected stack. Right-associative `implies`, prefix `not`, `let … in` and `if … else` chains at the longest length the default token and node limits admit parse; one element longer names the token or node limit and its setting. No outcome names a nesting depth.

