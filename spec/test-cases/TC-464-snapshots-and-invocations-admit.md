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

Scope: FR-106-AC-1, FR-106-AC-2, FR-106-AC-6, FR-106-AC-8.

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
5. Over TC-466 step 3's `probe` unit, admit the chain `a -> b -> c` pre
   snapshot for `ReachesTarget` with `PreCall { snapshot, self: a,
   parameters: {target: a} }`; the same selection for `VersionUnchanged`;
   with a snapshot that says `post`; with no `target`; with `target`
   naming `ghost`; with the snapshot
   marked `complete: false`.

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
- Step 5: one pre observation, `self` `a`, `target` naming `a`, no post
  observation, result or delta, and no frame check run;
  `wrong_snapshot`/`wrong-observation` twice; `invalid_runtime_input`/
  `missing-member`; `dangling_reference`/
  `absent-target-in-complete-population` naming `ghost` and
  `config_history`; `Incomplete` with `incomplete_population`/
  `incomplete-scope`.

## Status

Planned (QSL-273).
