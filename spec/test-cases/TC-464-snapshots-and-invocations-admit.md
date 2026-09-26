---
id: TC-464
title: "Snapshot and invocation documents read and admit into an observation set"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-464: Snapshot and invocation documents read and admit into an observation set

## Description

Verify the positive read and admission of FR-106's documents, digest
stability, and that admission reads nothing ambient.

Scope: FR-106-AC-1, FR-106-AC-2, FR-106-AC-6.

## Test Procedure

Compile FR-108's unit against TC-458's fixture package. Documents are
FR-106's examples, labelled as there, held in in-memory provisions keyed by
their `sha256-jcs` digest.

1. Admit the healthy-parent snapshot for `ParentOrder` with `Current {
   snapshot, anchor: {handler, validate}, self: {config_history, child} }`.
2. Re-serialize the same snapshot with other whitespace and reversed member
   order, and admit it with the same selection.
3. Admit the changed-version invocation for `VersionUnchanged`, with its pre
   snapshot (`root` 1, `child` 2 with parent `root`) and its post snapshot
   (`child` 3), `result` `{"boolean": true}`.
4. Repeat steps 1 and 3 in a process whose working directory is an empty
   temporary directory.

Tag the tests `#[trace("TC-464", "FR-106-AC-n")]`.

## Expected Results

- Step 1: one current observation; population `config_history` complete;
  `root.versionNumber` 1, `root.parent` absent, `child.versionNumber` 2,
  `child.parent` present naming `root`; `self` is `child`; the snapshot's
  identity and digest are retained.
- Step 2: the same digest and an equal admitted value.
- Step 3: distinct pre and post observations, `self` `child` in both,
  `result` true, no parameters, empty created and deleted.
- Step 4: equal results to steps 1 and 3.

## Status

Planned (QSL-273).
