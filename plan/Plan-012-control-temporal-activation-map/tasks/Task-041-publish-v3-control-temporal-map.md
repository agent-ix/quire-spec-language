---
id: Task-041
title: "Publish strict v3 control temporal activation mappings"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-054
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-142
    type: verifies
---
# Task-041: Publish strict v3 control temporal activation mappings

## Scope

Implement FR-054's new `/3` wire package, source-authorized producer selection,
independent strict-reader expectation and admitted view without changing `/1` or
`/2` behavior.

## Subtasks

- [ ] Write TC-142 fixture and mutation controls first.
- [ ] Add the versioned mapping row and canonical `/3` encoder/reader.
- [ ] Validate complete expected mappings independently of offered bytes.
- [ ] Prove every adverse mapping axis and cross-version refusal.
- [ ] Run the documented local Rust gates and PR-time reviews.

## Deliverables

- Constructor-private `/3` admitted package and mapping view.
- Exact producer/reader interfaces and TC-142 coverage.

## Notes

QProtocol is the downstream consumer. It cannot infer a relation while this
task is incomplete.
