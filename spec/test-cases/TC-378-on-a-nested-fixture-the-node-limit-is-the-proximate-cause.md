---
id: TC-378
title: "On a nested fixture, the node limit is the proximate cause of a function-declaration stop"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-378: On a nested fixture, the node limit is the proximate cause of a function-declaration stop

## Description

Verify FR-062-AC-7 against a body that nests: holding a fixture of N nodes
nested N deep fixed and varying only the configured `CheckingLimits` node
limit by exactly one shows the limit, not the fixture's depth, to be the
proximate cause of the stop. The `Limit` carries its setting and the
`Locus::Region` of the node whose entry failed (FR-096-AC-11).

Scope: FR-062-AC-7, FR-096-AC-11.

## Test Procedure

1. Parse a function declaration whose body is the source text
   `not not not true` (three `Not`s wrapping a `Boolean` leaf, four
   expression nodes) under a unit reference, so its forms carry spans, and
   check it through `ValueFunctionFamily::check` with the `CheckingLimits`
   node limit configured to 3.
2. Check the identical declaration again with the node limit configured to
   4 and nothing else changed.

## Expected Results

- Step 1: `StageFailure::Limit` with kind node count, bound 3, actual 4,
  setting `s3.nodes`, and `Locus::Region` over the span of the node whose
  entry failed, reported as `stage_limit_exceeded`/`node-count-exceeded`.
- Step 2: checking succeeds; no `Limit` outcome is returned.

Tag the tests `#[trace("TC-378", "FR-062-AC-7")]` and
`#[trace("TC-378", "FR-096-AC-11")]`.

## Status

Planned for the node-limit form above. The present
`the_typer_depth_stop_is_located_at_the_node_whose_entry_failed`
(`qsl-semantics/src/check/family.rs`) drives the depth limit ADR-030
deletes and is replaced by this case.
