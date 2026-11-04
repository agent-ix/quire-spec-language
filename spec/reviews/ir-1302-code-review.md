---
id: SR-202611030001
title: "Code and Rust review of IR-1302 PR #2221"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7487fd4ab387d8bb5dc426986f3cc5459af0b86b; PR #2221 diff against origin/main (4d8a11c05); Ticket: IR-1302; FR-091-AC-34"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---

## Summary

Ticket: IR-1302. Reviewed PR #2221 against origin/main. The unit-graph topology now walks QSpec FR-142's turn list and reports one first defect. I ran six extra FR-142 vectors (U11n, U11o, U11p, U11w, U11x1, U11x2) through a scratch copy of the PR; all gave the spec result. One defect: declaration order is sorted by byte offset alone, but FR-142 orders by source document first.

## Verdict

**PASS WITH FINDINGS** — behavior matches the spec on every single-document vector I ran. FND-001 is the only code defect. No new allow, unwrap, expect or production panic; fmt, clippy -D warnings (qsl-semantics, all targets) and arch-lint qualified-core pass.

## Assurance Context

No applicable AssuranceProfile was found under spec/ for the changed code.

## Scope

- `qsl-semantics/src/check/assemble/units.rs`: examined.
- `qsl-semantics/src/check/assemble.rs`: examined.
- `qsl-semantics/src/check/assemble/tests.rs`: examined.
- `QSpec FR-142 (turn list, step -1, rows 1-6, base-dimension rule)`: examined at cceef5918.

## Checks

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked -p qsl-semantics --all-targets -- -D warnings`: passed. The --no-default-features and whole-workspace clippy runs were not repeated.
- `cargo run --locked -p arch-lint -- qualified-core --qsl .`: passed.
- `cargo test -p qsl-semantics --tests` with QSPEC_DIR set: integration 522/522; lib 713 passed, 58 failed. The same 58 fail on origin/main with QSPEC_DIR set (709 passed, 58 failed; identical failure list, protocol parse errors). QSPEC_DIR is not the cause and the PR adds no failure; its 4 added tests pass.
- Added-line grep for allow, unwrap, expect, panic: only test-code `panic!` in let-else.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Walk::turns` orders dimension and unit declarations by `span.start` alone, and `dimension_cycle` takes the least dimension index, but FR-142 defines source order as source documents in request order, then byte offset. `units::assemble` receives forms from every source document (`source_owners`), whose spans each start at 0, so with more than one document the turn list, the first defect and the cycle start can come out in the wrong order. Order by (document, offset), or show that forms of one unit never span documents. | qsl-semantics/src/check/assemble/units.rs:466 |
