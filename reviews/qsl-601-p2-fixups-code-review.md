---
id: SR-20261109202150
title: "Code review — l5/p2-render follow-up: doctest lane declarations and checked_handoff fixtures"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; branch l5/p2-render vs origin/l5/integration; xtask/src/test_suites.rs, src/checked_handoff/corpus.rs, src/checked_handoff/inventory.rs, reviews/qsl-601-code-review.md, reviews/qsl-601-gap-analysis.md"
review_set: subset
---

## Summary

Ticket: QSL-601. Code review with the rust-review lane (rust-style loaded)
over `git diff origin/l5/integration...origin/l5/p2-render`. The render code
itself already landed on the integration base (#2598). This diff holds three
things: the two committed review records, the empty-doctest lane declarations
in `xtask/src/test_suites.rs` (it drops `qsl-analyze` and `qsl-inspect`, which
now carry `compile_fail` doctests, and declares `src/lib.rs` doctests empty in
the default-feature lane only), and two `checked_handoff` test fixtures that
gain the now-required `operand_rows`. Confined runs: `cargo test -p
quire-spec-language --features checked-handoff --lib checked_handoff::`
passed 5/5, and `cargo test -p xtask test_suites` passed 75.

## Verdict

**Changes requested (one medium).** The fixtures are correct: an empty node
list pairs with empty `operand_rows`, and one node pairs with one row. The
lane split matches the tree. `src/lib.rs` gates `checked_handoff`, the only
module in the root crate with doc examples, behind `checked-handoff`, and
`qsl-analyze` and `qsl-inspect` now hold `compile_fail` doctests. The medium
finding is the guard rule that made this xtask change necessary. It fails on a
healthy change: adding a doctest to a crate.

## Scope

- **xtask/src/test_suites.rs:26-30 — examined**: "A suite a lane declares empty must show `running 0 tests` and `0 filtered out`; a declared suite that gains a test of any kind ... fails, so a stale declaration cannot hide new tests."
- **xtask/src/test_suites.rs:182-298 — examined**: the `workspace_empty!` macro, `WORKSPACE_EMPTY`, `ALL_FEATURES_EMPTY`, `LANES`.
- **src/checked_handoff/corpus.rs:1008-1012 — examined**: fixture `semantic_graph: { nodes: [], operand_rows: [] }`.
- **src/checked_handoff/inventory.rs:365-367 — examined**: fixture with one node and `operand_rows: [[]]`.
- **reviews/qsl-601-code-review.md — examined**: findings FND-007, FND-008 and the Dispositions table.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Value test: the guard rule "a declared-empty suite that gains a test fails" (test_suites.rs:26-30) fails on a healthy change. Adding doctests to qsl-analyze and qsl-inspect turned the gate red, and this PR's xtask edit and its new macro exist only to bring the inventory back in line with the tree. A declared-empty suite that runs and passes at least one test hides nothing. Drop that failure, keeping the ignored/filtered-out detection if wanted. One list then serves both workspace lanes, and the macro and `ALL_FEATURES_EMPTY` go away. | xtask/src/test_suites.rs:26-30, 186-298 |
| FND-002 | low | `macro_rules! workspace_empty` exists only to share a const-array prefix. rustfmt does not format the macro body, which is already mis-indented under `&[`. `$($extra:expr),*` rejects a trailing comma. A per-lane `extra: &[EmptySuite]` slice chained with the shared list at the check site (or FND-001's single list) is plainer Rust. | xtask/src/test_suites.rs:186-278 |
| FND-003 | low | The macro's doc comment says the default-feature and all-feature runs "listed the same suites". After this PR they differ on `src/lib.rs`, so the claim is stale. The "retained make ci log of the QSL#670 gate ... older head" sentence is provenance prose with no value to a reader. | xtask/src/test_suites.rs:182-185 |
| FND-004 | low | The committed review records are inconsistent. In reviews/qsl-601-code-review.md, FND-007 and FND-008 have no Dispositions row, although FND-008 is fixed in docs/inspect-render.md (Re-land section removed, `used + n`). FND-007 is only a "Known gap" sentence with no ticket. Both records' `scope` still cite commit e508a53e1, which is SHA recording the repo's CLAUDE.md asks to keep out of review logs. | reviews/qsl-601-code-review.md:6, 54-70; reviews/qsl-601-gap-analysis.md:6 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The declared-empty suite that passes a test is no longer a finding; a declared suite that passes nothing yet registers ignored, measured or filtered-out tests still fails. One `WORKSPACE_EMPTY` list serves both workspace lanes and `ALL_FEATURES_EMPTY` is gone. |
| FND-002 | fixed | `workspace_empty!` is removed; the list is a plain const slice. |
| FND-003 | fixed | The stale "same suites" claim and the retained-log provenance are removed from the list's doc comment. |
| FND-004 | fixed | reviews/qsl-601-code-review.md gains FND-007 (accepted-no-change, listed in the PR body) and FND-008 (fixed) dispositions; commit SHAs are removed from both review records' scope. |
