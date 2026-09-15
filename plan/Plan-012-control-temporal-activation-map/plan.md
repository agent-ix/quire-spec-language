---
id: Plan-012
title: "Compiled protocol v3 activation mapping"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-054
    type: references
---
# Implementation Plan: Compiled protocol v3 activation mapping

## Requirements Summary

- [ ] **FR-054**: Publish a strict `/3` control-to-temporal activation mapping
  while retaining `/1` and `/2` unchanged.

## Dependency Graph

- `QSpec FR-052 + FR-300 -> FR-054`
  Reason: the mapping semantics and its Protocol consumer boundary are owned by
  the accepted system clauses.
- `FR-050 -> FR-054`
  Reason: `/3` carries an unchanged admitted `/2` package plus a new mapping
  table; it cannot alter the frozen `/2` representation.

## Test Plan

- [ ] **TC-142**: canonical `/3` round trip; every mapping mutation; exact
  cross-version refusals; no profile/clock or coordinate inference.

## Remaining Work

### Track A: Critical Path (serial)

- **A1 = Task-041** Strict `/3` activation mapping — Hard; exit: the producer
  and independent reader accept only exact authored mappings while `/1` and
  `/2` remain byte-identical and reject `/3`.

## Parallel Execution Summary

```text
Task-041: specification fixtures -> `/3` wire/producer/reader -> TC-142 -> QProtocol pin
```

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-041 | A | FR-054 | TC-142 | not_started |

## Coordination Rules

Keep `/1` and `/2` code, schemas, fixtures and public signatures frozen. The
new mapping comes only from an explicit authored selection and is published in
the new `/3` artifact; QProtocol must not begin consumption until this task has
an admitted owner view.
