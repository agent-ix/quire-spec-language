---
id: SR-418
title: "Code and Rust review of ConfigVersion numeric backend parity"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#84; IT-010; FR-033; FR-034; tests/configversion_backends.rs; tests/integer_lowering.rs; Cargo.toml; Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/IT-010
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-034
    type: reviews
---

## Summary

PASS for issue #84's Rust-only integration increment. The reviewed tests start
from actual checked Quire source, use `lowering::lower_for`, pass the selected
bytes through the strict IR reader, compile and execute generated Rust, run the
generated proptest campaigns, execute cargo-kani, decode its concrete playback,
and replay the exact pre/post pair through `runtime::execute`. Production native
runtime and lowering code are unchanged; the dependency graph is reconciled to
one reviewed contract-IR revision and the merged numeric codegen revision.

## Verdict

**PASS** — the four review findings were repaired and no high, medium or
actionable low Rust finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Closed: the generated-function parser leaked a boxed syntax node; the helper now returns the owned `syn::ItemFn`. | tests/configversion_backends.rs |
| FND-002 | high | Closed: outside-domain checks now require `InvalidRuntimeInput` and the exact nominal-bounds diagnostic before confirming no truth. | IT-010-SC-03; tests/configversion_backends.rs |
| FND-003 | high | Closed: instrumentation retains every `(pre, post, verdict)` triple and all generated observations replay through `runtime::execute` with equal verdicts. | IT-010-SC-04; tests/configversion_backends.rs |
| FND-004 | medium | Closed: the same lowered plain-integer clause now generates syntax-checked boundary strategy and Kani artifacts with asserted `0..=1000` bounds. | FR-033-AC-4; tests/integer_lowering.rs |

## Rust review

- No unsafe, async, lock, filesystem mutation, public API, production panic or
  unchecked integer-conversion surface is added. Temporary generated crates are
  test-owned and every subprocess exit is checked.
- The shared corpus is the complete cross-product of `-1`, `0`, `1000` and
  `1001`. In-domain native and compiled-oracle verdicts agree; out-of-domain
  inputs stop at validation and never reach the Boolean oracle.
- Generated satisfying, violating, broad and boundary campaigns execute with
  zero discards. Every exact generated observation is range-checked and replayed
  natively; the test derives its expected count from the generated boundary
  report rather than assuming a fixture cardinality.
- ParentOrder stops at its first unrepresentable `present` object read before
  `deref`; NoCycle stops at `reaches`. Both retain authored clause/source loci
  and produce no upstream or IR artifact. No approximation is admitted.
- cargo-kani 0.67.0 is pinned by executable SHA-256
  `7f143a251d11c7e6e232bbf2cbccf56f9ce66a5f0107eeb3008698e6715f55d9`.
  The graph fixes `-Z function-contracts`, `-Z concrete-playback`, exact harness,
  `--exact`, `--unwind 2`, `--solver cadical`, regular output, and concrete
  playback printing. Identity proves; the changed subject fails and its actual
  decoded counterexample returns false natively.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| Full all-target minimal-feature suite | pass; three inherited private-packet tests and two stripped-release tests ignored |
| Full all-target/all-feature suite before review fixes | pass; review fixes were then covered by the full minimal suite and focused all-feature tests |
| Strict Clippy, all targets, minimal features | pass with `-D warnings` |
| Strict Clippy, all targets, all features | pass with `-D warnings` |
| Changed specification and review documents | pass with pinned Quire 0.31.0 |

All Cargo invocations used `--target-dir target-codex-backends`. No hosted
workflow was dispatched, `publish = false` is asserted, `.github` is unchanged,
and `resources/native-v1/` is unchanged.
