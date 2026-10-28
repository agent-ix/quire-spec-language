---
id: SR-1708001
title: "Code review of quire-spec-language PR #1708: drain-bearing source-default memory classes in the class catalog (QSL-1144, semantics slice)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@edb5ff719; PR #1708 diff 313953d5...qsl-1144-classes: qsl-semantics/src/check/refinement/{catalog.rs,internal_list.rs}; qsl-semantics/tests/it/refinement_protocol.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-375
    type: reviews
---
# Code review of quire-spec-language PR #1708

## Summary

Ticket: QSL-1144 (STD-494). The Rust review lane is folded into this
review. I reviewed only `git diff 313953d5...origin/qsl-1144-classes` at
edb5ff719, against QSpec `origin/main` (792993ab). That version contains #459
(2b991805), which defines catalog uniqueness. The specs are FR-375 "Checked
class catalog" with AC-23 and AC-26, FR-432's class table and AC-3/AC-8,
and TC-310 CC-12/CC-15.

Gates: `cargo test -p qsl-semantics --features test-support` passed at
edb5ff719 (EXIT=0: 1322 + 799 + 30, none failed). I did not run clippy.

I re-measured each claim:
- Each model a `parallel` selects (`sc`, `tso`, `ra`) gets a
  `memory drain` entry with `drain()` = `store-buffer-flush`; `memory flush`
  is only for `tso`. This matches FR-375's memory-entry shapes.
- `ProtocolClasses.memory_models` is a `BTreeSet<MemoryModelName>`, so
  repeated models collapse to one entry (AC-26).
- The catalog key is (`class`, memory-model label, empty for a declared
  class), compared as bytes with `class` first. That gives drain(ra),
  drain(sc), drain(tso), then flush(tso), matching FR-375 and CC-12. Both
  the `BTreeMap` key and `canonical()` use the same key.
- The two new tests pass, and so do the updated expectations in the two
  existing catalog tests.

## Verdict

**The catalog half is correct.** The rest of the checker does not yet know
the new class (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The catalog now lists `memory drain` for every model, but nothing else in the checker knows the class. My local probe on `Full` (`parallel par memory tso`) shows two refusals. The internal-list entry `"memory drain"` refuses `InternalList Missing` at `/0/internal/0`, even though the abstract catalog holds `memory drain` (tso). FR-432-AC-8 and its class table say a `memory` entry makes exactly its class internal, and the table lists the cross-model drain class. `ProtocolClasses::resolve` only reads `self.memory`, which is `memory_step` (flush/tso), and `MemoryStepKind` has only `Flush`, so no `step memory drain` row can name the class either. FR-432-AC-3 and FR-375-AC-19 both admit such a row. The catalog therefore holds a class that no row or internal entry can decide, and FR-375-AC-23's row-join will refuse that once V2 emission (QSL-1181) lands. Fix this in the PR, or open a ticket that QSL-1181 depends on: add drain to the internal-list memory set (for every model) and as a memory step class. | qsl-semantics/src/check/refinement/internal_list.rs:566-577, 716-722, 778-783; qsl-semantics/src/check/refinement/protocol.rs:81-101, 568-582 |
| FND-002 | low | `canonical()` uses `sort_by_key`, which calls `catalog_key()` on every comparison. That clones the class `String` and turns it into bytes each time. Compare borrowed keys with `sort_by` instead, which gives the same order with no allocation. | qsl-semantics/src/check/refinement/catalog.rs:152-157 |

## Dispositions

Round 1, fix commit e5a023bad. I reviewed `git show e5a023bad`, leaving out the main merge 73bd3a684/2c4aeda3c. `cargo test -p qsl-semantics --features test-support` passed at e5a023bad (EXIT=0: 1324 + 801 + 30), run through locked-build with jobs 4, debug info off and QSPEC_DIR qspec-e3d48d76. My probes were re-applied locally and not committed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `MemoryStepKind::Drain` is now a memory step of every source-default model (`of_model`: `sc`/`ra` give `[Drain]`, `tso` gives `[Drain, Flush]`). `ProtocolClasses::memory` adds `drain` for each `parallel`. My probe on `Full` (tso) now admits the internal entry `"memory drain"` (spellings `["memory drain"]`), where it was refused `Missing` at `/0/internal/0`. Repeating the entry refuses `Conflicting` at `/0/internal/1`, naming `/0/internal/0`. `a_memory_drain_row_resolves_under_every_source_default_model` resolves a `step memory drain` row under `sc` (the default), `ra` and `tso` (beside flush), and still checks with no drain row. A probe with two drain rows is refused. Drain rows are `Need::Optional`, which matters only in the missing-row check (`by_operation = true`). Resolution, duplicate-row refusal and step ordering are unchanged, and nothing outside qsl-semantics reads `MemoryStepKind`, so an absent drain row hides no other defect. Making the row mandatory, per the ruling, is a one-line change in `MemoryStepKind::need` once the QSpec AC lands. |
| FND-002 | fixed | `canonical()` now uses `sort_by` over borrowed `(class bytes, model label bytes)` keys, with the same order and no allocation. |
