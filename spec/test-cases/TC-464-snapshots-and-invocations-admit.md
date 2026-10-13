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

Scope: FR-106-AC-1, FR-106-AC-2, FR-106-AC-6, FR-106-AC-8, FR-106-AC-9.

## Test Procedure

Compile FR-108's unit against TC-458's fixture package. Documents are
FR-106's examples, labelled as there, held in in-memory provisions keyed by
their `sha256-jcs` digest.

1. Admit the healthy-parent snapshot for `ParentOrder` with `Current {
   snapshot, anchor: {handler, validate}, self: {config_history, child} }`;
   then, over the package variant where `Sub` specializes `ConfigVersion`
   and a second population `archive` has member type `Sub` (TC-465's
   `archive` fixture), with `child.parent` naming `k`, a `Sub` object of a
   complete `archive`.
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
   naming `ghost`; over the package variant where `Sub` specializes
   `ConfigVersion` and a second population `archive` has member type `Sub`
   (TC-465's `archive` fixture), with `target` naming `a1` in an `archive`
   listed with no objects and marked `complete: false`; with the snapshot
   marked `complete: false`; in that variant, with `target` naming `k`, a
   `Sub` object of a complete `archive`; over the base unit plus an object
   type `Other` unrelated to `ConfigVersion` and a population `others` over
   it, with `target` naming `o`, an `Other` object of a complete `others`;
   over the variant plus an operation `probeSub(target: Sub)` on
   `ConfigVersion` and a precondition `SubTargetHolds` on it, `PreCall`
   for `SubTargetHolds` with `target` naming `a`.
6. Admit the forbidden-parent-change invocation (TC-465) selected for a
   precondition on `attemptUpdate`, and evaluate it; then admit and evaluate
   `PreCall` over that invocation's pre snapshot, `self` and parameters for
   the same precondition. Repeat the invocation selection with its `post`
   snapshot removed from the provision, and with its `post` snapshot bytes
   replaced by malformed (non-JSON) bytes. Repeat it with the invocation's
   `post`, `result`, `created` and `deleted` members removed, and with each
   of them ill-formed. Over TC-466 step 3's `probe` unit, admit and evaluate
   a `probe` invocation (`self` `a`, `target` `c`, over the chain `a -> b ->
   c`) whose `result` is `{"boolean": true}`, selected for `ReachesTarget`,
   and `PreCall` over its pre snapshot, `self` and parameters.

Tag the tests `#[trace("TC-464", "FR-106-AC-n")]`.

## Expected Results

- Step 1: one current observation; population `config_history` complete;
  `root.versionNumber` 1, `root.parent` absent, `child.versionNumber` 2,
  `child.parent` present naming `root`; `self` is `child`; the snapshot's
  identity and digest are retained. In the `archive` variant, the snapshot
  admits with `child.parent` present naming `k`, typed `Sub`.
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
  `incomplete-scope` naming `archive`; `Incomplete` with `incomplete_population`/
  `incomplete-scope`; admitted with `target` naming `k`, typed `Sub`;
  `invalid_runtime_input`/`wrong-value-kind` at `target` twice.
- Step 6: the invocation admits one pre observation with no post
  observation, result or delta and no frame check run (no
  `frame_violation`); its verdict equals the `PreCall` verdict. The
  missing-post and malformed-post repeats each admit with the same verdict
  and no refusal, and so do the repeats with the four post-side members
  removed or ill-formed: a precondition neither requires nor reads them.
  The `probe` invocation with a result value admits, and `ReachesTarget`
  is `true` under it and under `PreCall`.
