---
id: SR-2026110215410
title: "QSL-1473 PR #2151 code and Rust review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language@00b1763b11535c7215b64ef25c285e5dc4a2b885; PR #2151 diff: qsl-analyze/tests/it/fill.rs, qsl-analyze/tests/it/rc11_oracle.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
---

## Summary

Ticket: QSL-1473. Reviewed frozen PR #2151 head `00b1763b11535c7215b64ef25c285e5dc4a2b885` against `origin/main` `347e9769efa0add003b806edc82c309a6484fe33`. The helper limits worker spawning to the caller's positive `NonZeroUsize`, creates exactly that many scoped workers when the frontier is wide enough, and preserves deterministic admission order. The LT-16 integration test reads a dedicated environment variable independently of test-thread count, asserts the observed maximum spawned worker count, exact state and edge totals, and the forbidden outcome. No code defect was found in the changed Rust. Coder-reported focused run and compile/Clippy/format/diff checks are not independent reviewer test evidence.

## Verdict

**PASS** — no code or Rust-review findings in the changed files. Full QSL-1473 acceptance remains conditional on the separately tracked scaling runs and profile; see the gap-analysis artifact.

## Assurance Context

No applicable AssuranceProfile was found under `spec/` for the changed test-only Rust files.

## Scope

- `qsl-analyze/tests/it/fill.rs`: examined bounded worker selection, scoped thread creation, worker-count reporting, deterministic frontier-position result collection, and preservation of the existing unrestricted reachability wrapper.
- `qsl-analyze/tests/it/rc11_oracle.rs`: examined environment parsing, the LT-16 worker-count assertion, exact 7,607 state and 19,574 edge assertions, and the unchanged forbidden-outcome assertion.
- FR-223-AC-8: examined as the LT-16 semantic contract; the changed test remains tagged to TC-668 and FR-223-AC-8.
- QSL-1473 AC-1: “The worker count is settable by the caller and honoured by the explorer; a test proves a run with N workers uses N workers.”
- QSL-1473 AC-3: “LT-16 still reports exactly 7,607 states and 19,574 edges with the oracle assertion unchanged, at every worker count. LT-12 and LT-13 keep their assertions.”
- QSL-1473 AC-4: “No state, depth or input-size caps are introduced.”
- `qsl-analyze/tests/it/fill.rs` and `qsl-analyze/tests/it/rc11_oracle.rs`: full changed diff examined; no production code, Cargo configuration, or workflow files changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
