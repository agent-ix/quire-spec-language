---
id: SR-(commit)
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
| FND-004 | low | The committed review records are inconsistent. In reviews/qsl-601-code-review.md, FND-007 and FND-008 have no Dispositions row, although FND-008 is fixed in docs/inspect-render.md (Re-land section removed, `used + n`). FND-007 is only a "Known gap" sentence with no ticket. Both records' `scope` still cite commit (commit), which is SHA recording the repo's CLAUDE.md asks to keep out of review logs. | reviews/qsl-601-code-review.md:6, 54-70; reviews/qsl-601-gap-analysis.md:6 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The code now contradicts NFR-014, which is still in force. NFR-014-AC-5 requires a declared-empty suite's running, passed, failed, ignored, measured and filtered-out counts all to be zero. NFR-014-AC-7 says a declared-empty suite "gaining any registered, passing ... tests refuses as declaration drift". The renamed test `a_declared_suite_that_gains_a_passing_test_is_fine` and the `over_declared` doctest fixture keep `#[trace("TC-950", "NFR-014-AC-5", "NFR-014-AC-7")]` but assert the opposite of both ACs. Either amend NFR-014-AC-5/AC-7 and TC-950 in this PR to the new rule, or restore the rule. | xtask/src/test_suites.rs:1088, 1900-1909, 4118-4127; spec/non-functional/NFR-014-check-native-ci-suite-emptiness.md:78-80 |
| FND-006 | low | The FND-004 edit garbled a sentence in reviews/qsl-601-code-review.md: "Compared with the stale-branch the stale-branch commit:". | reviews/qsl-601-code-review.md:19-20 |
| FND-007 | low | The `evaluate` doc comment says a declared-empty suite "may not hide any it does not pass: none ignored, measured or filtered out". The code `passed == 0 && (running != 0 \|\| filtered != 0)` accepts ignored, measured or filtered-out tests whenever at least one passes. | xtask/src/test_suites.rs:1062-1065, 1088 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The rule was dropped in code and one list serves both lanes, but NFR-014-AC-5/AC-7 still require it, so the code now violates its spec (FND-005). The fix needs the NFR-014/TC-950 amendment in the same PR, or a revert. |
| FND-002 | fixed | `workspace_empty!` is removed; `WORKSPACE_EMPTY` is a plain const slice. |
| FND-003 | fixed | The doc comment now reads "The suites that pass no test in the `make ci` workspace lanes; one list serves both ..."; the provenance prose is gone. |
| FND-004 | fixed | FND-007 (accepted-no-change) and FND-008 (fixed) have disposition rows; SHAs are removed from both records' scope (garbled sentence filed as FND-006). |
| FND-001 | rejected | NFR-014-AC-5 and NFR-014-AC-7 mandate that a declared-empty suite gaining any test refuses as drift, so the rule is a spec requirement, not ceremony. The finding was wrong against the spec. The guard is restored and both workspace lanes share `WORKSPACE_EMPTY`, with the default lane adding `ROOT_DOCTESTS_EMPTY` through `Lane::extra`. |
| FND-005 | fixed | `evaluate_declared` again flags `suite.running != 0 \|\| suite.counts.filtered != 0`. `a_declared_suite_that_gains_a_passing_test_fails` and the `wrongly_declared` doc-workspace fixture again assert NFR-014-AC-5/AC-7. |
| FND-006 | fixed | reviews/qsl-601-code-review.md reads "Compared with the stale-branch implementation, dropping `package_id` ...". |
| FND-007 | fixed | The `evaluate` doc comment reads "A declared-empty suite must register nothing: no test run, ignored or measured, and none filtered out", which matches the restored code. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | Regression of FND-003. Reverting the guard also restored `WORKSPACE_EMPTY`'s old doc comment. It again says the default-feature and all-feature runs "listed the same suites", which is false: the default lane adds `ROOT_DOCTESTS_EMPTY`. It again carries the "retained `make ci` log of the QSL#670 gate ... older head" provenance prose. | xtask/src/test_suites.rs:191-194 |
