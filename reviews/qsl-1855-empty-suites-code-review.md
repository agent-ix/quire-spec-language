---
id: SR-2026111012423400000001
title: "Code review — QSL-1855 default-workspace empty-suite declarations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; branch fix-empty-suites; xtask/src/test_suites.rs WORKSPACE_EMPTY; NFR-014; TC-950"
review_set: subset
---

## Summary

Ticket: QSL-1855. Code-review lane for branch `fix-empty-suites` at head `715b60c69b8e8ba92e477f03b2b06022226339d4`, base `19bf7f0b9`. The diff adds two `EmptySuite` declarations to `WORKSPACE_EMPTY`: `qsl-command/src/lib.rs` doctests (`NO_DOC_EXAMPLE`) and the `tools/qsl-probability-check/src/main.rs` binary test suite.

## Verdict

**PASS** — both suites are empty, the reasons are accurate, and the lane check is not weakened. Two low, cosmetic placement notes below; neither blocks merge.

## Review

- `qsl-command/src/lib.rs` doctests: no fenced or indented code block in any doc comment under `qsl-command/src`; `cargo test -p qsl-command --doc` reports `running 0 tests ... 0 filtered out`. The crate's `#[test]` items live in unit tests (`src/command/output.rs`), which are a separate `Tests` suite and are not declared empty. `NO_DOC_EXAMPLE` is accurate.
- `tools/qsl-probability-check/src/main.rs`: the package is bin-only (`src/main.rs` is the sole source file, no `#[test]` or `#[cfg(test)]`), and its tests are the `[[test]] it` target, whose `bounded.rs` and `roundtrip.rs` run the built binary. `cargo test -p qsl-probability-check --bin qsl-probability-check` reports `running 0 tests ... 0 filtered out`. The reason text matches the existing `qsl-string-edge/src/main.rs` entry for the same shape.
- No weakening: declarations are exact `(source, kind)` identities, and the gate still fails a declared suite that gains any test, so these entries cannot hide future tests. Both `default-workspace` and `all-features-workspace` share `WORKSPACE_EMPTY`, which is right because neither suite changes under features. The `qsl-command` lib `Tests` suite stays required non-empty.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low      | The `qsl-probability-check` `Tests` entry is placed inside the doctest run of the list; neighbouring `Tests` entries for binaries (`qsl-bench-probe`, `qsl-string-edge/src/main.rs`, `xtask/src/main.rs`) are grouped at the top. Cosmetic only. | xtask/src/test_suites.rs:248-252 |
| FND-002 | low      | `qsl-command/src/lib.rs` doctests is inserted after `qsl-foundation`, breaking the otherwise alphabetical `qsl-*` doctest run (`qsl-analyze`, `qsl-bench`, `qsl-foundation`, `qsl-inspect`, ...). Cosmetic only. | xtask/src/test_suites.rs:243-247 |

## Verification

- `CARGO_TARGET_DIR=target/ci-default-features cargo test --locked -p qsl-command --doc` — `running 0 tests`, `0 filtered out`.
- `CARGO_TARGET_DIR=target/ci-default-features cargo test --locked -p qsl-probability-check --bin qsl-probability-check` — `running 0 tests`, `0 filtered out`.
- `cargo test --locked -p xtask --lib every_lane_names_each_empty_suite_once` — 1 passed (no duplicate ids, non-empty reasons).
- The full `cargo xtask test-suites default-workspace` lane was not re-run in this review.

## Dispositions

| FND | Disposition |
| --- | --- |
| FND-001 | Fixed: the entry moved beside the other binary Tests entries. |
| FND-002 | Fixed: the qsl-command doctest entry moved ahead of qsl-foundation. |
