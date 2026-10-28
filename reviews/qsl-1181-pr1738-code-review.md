---
id: SR-1738001
title: "Code review of quire-spec-language PR #1738: memory-class catalog entries and memory rows (QSL-1181 slice 5)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f3178357 (the slice's own commit; the #1727 merge 7d5bb8caf excluded), 2 files, against FR-375-AC-19/23/26, FR-375 steps table, FR-432 as amended by STD-524/525 (FR-432-AC-22), FR-322"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-375
    type: reviews
---
# Code review of quire-spec-language PR #1738

## Summary

Ticket: QSL-1181 (slice 5).

**Scope.** I reviewed only `2f3178357`: 2 files. That is `git log origin/qsl-1181d-rows..origin/qsl-1181e-memory` minus the #1727 merge (`7d5bb8caf`, reviewed under SR-1727).

**Memory rows without `source_node` are spec-correct.** FR-375's `steps` table says `source_node` "is required for a class with a declaration node and absent for a memory step class, which names no declaration". `memory_rows_go_last_drain_before_flush` checks both halves: every memory row lacks the member and every other row has it. It also checks that reversing the rows changes neither the node id nor the package id (FR-375-AC-19).

**The typed-field keying cannot drop or duplicate a class today.**
- The catalog's memory classes are built in exactly one place, `ProtocolClasses::catalog_classes`. For each model in the set `memory_models` it writes one `memory_drain(model)`, which carries `drain: Some(store-buffer-flush)`. For `tso` only (`memory_step`) it adds one `memory("memory flush", tso)`, with `drain: None`.
- So `(Some(_), model)` matches exactly the drains, one per model, and `(None, Tso)` matches exactly the one flush. `(None, Sc | Ra)` cannot happen and faults.
- The set keeps a second `sc` from adding an entry. `memory_catalog_entries_are_one_drain_per_model_then_flush` shows the `ra`, `sc`, `tso` drains, then flush, with the second `sc` adding nothing.

What remains is that flush is identified by elimination, not by type (FND-001).

**Read-back.** `memory_rows_read_back`, run with `--include-ignored`, refuses `invalid_semantic_graph` at `class_catalog/concrete/0`. That is the IR-874 catalog-rebuild gap, as claimed.

**Gates** (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics: lib 804, it 1328, doc 30, all pass.
- qsl-package lib: 385 pass, 5 ignored, and only c14/i2 fail (QSL-1190; now `MissingImport` at `/lock/model_selections/0/digest`, after the IR main repin under this stack).
- Workspace clippy `-D warnings`: exit 0.

## Verdict

**No defect.** Mergeable, with one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `catalog_class_keys` keys `KeyedMemoryClass::Flush` by elimination: any memory class under `tso` without a typed `drain`. FR-432's class table admits further memory classes where a source-default model defines their step (FR-434, FR-436, FR-437). If one is added under `tso`, it would be keyed silently as `Flush`, where the old spelling match faulted on an unknown class. Give `RefinementCatalogClass` a typed memory step kind (for example `MemoryStepKind`) beside `drain`, and match it exhaustively, so a new class fails to compile rather than being mis-keyed. | qsl-semantics/src/check/lowering/protocol.rs:1336-1356; qsl-semantics/src/check/refinement/internal_list.rs (`memory_step`); QSpec FR-375 "Checked class catalog" |
