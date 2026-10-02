---
id: SR-064
title: "Rust code review — native formal source correspondence"
type: SpecReview
analysis: code-review
scope: "7464c9a: src/formal_source.rs, src/lib.rs, tests/formal_source.rs and FR-014"
review_set: subset
evaluated_revision: "7464c9a"
---

## Summary

The bridge implements the reviewed exact-source contract with existing native
and IR source types. No code or qualification defect was found in this scope.

## Verdict

PASS for FR-014 implementation. This does not qualify the native checker,
runtime, backend or complete state workflow.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | src/formal_source.rs; tests/formal_source.rs |

## Review method

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md and the
user-selected /home/peter/dev/agent-skills/rust-review/SKILL.md, with portable
rust-style defaults and AGENTS.md/Cargo.toml conventions. No applicable
AssuranceProfile or repository-specific rust-style file exists. No subagent or
independent-review claim is made. The optional gap semantic comparison was not
run; mandatory code-review faithfulness and test alignment were inspected.

The twelve Rust checks cover documented ownership signatures and newtypes,
trace attributes, real dependency seams, source/test completeness, unchanged
lints, panic surface, checked numeric conversion, async/blocking applicability,
immutable lifecycle, input trust, bounds and executed gates. New production
code has no panic/unwrap/unsafe/unchecked cast, user callback, thread, lock,
filesystem access or unbounded traversal. The existing Diagnostic error type
and documented manual Error implementation remain the crate boundary.

Every public method is documented and exercised through the public module.
Forward mapping verifies all four native identity dimensions before using the
stored Source index. Reverse mapping rejects a foreign identity before checked
offset conversion, reconstructs the byte-derived span and compares every IR
field. No false coordinate is clamped into a plausible location. Upstream
constructor failures retain structured diagnostics; those defensive branches
are not claimed fault-injected because admitted native coordinates already fit
the typed IR constructors under the existing source ceiling.

The tests use imported bare single-line #[trace] attributes for TC-035–039 and
FR-014-AC-1..5. The generated test executes 156 independently enumerated source
strings in both request orders, including invalid offset pairs. Its oracle
scans Unicode scalars instead of calling Source's index or the bridge. The
false-IR-locus test constructs all adverse inputs through real IR constructors;
an upstream setup refusal cannot count as a bridge rejection. No mocks, skipped
bridge tests, source-inspection substitutes or wall-clock assertions occur.

Reverse-gap discovery maps all five public behaviors (constructor, two getters,
forward mapping and reverse mapping) to FR-014. Defensive conversion and local
diagnostic behavior are specified. No new behavior lacks an owning requirement;
no source/test stub was found. The existing crate root's module declarations
are intentional, not a hollow implementation. No retro or global-skill change
is justified by this scoped review.

## Verification

Actual exit-zero logs are in reviews/data/formal-source. Cargo runs used nice 10,
offline locked dependencies, the existing target caches and -j 1; tests used
--test-threads=1. The first focused run records the expected missing-module
compiler error before implementation, followed by five passing new tests.

| Gate | Actual result |
| --- | --- |
| cargo fmt --all -- --check | Exit 0 |
| cargo clippy --workspace --all-targets --all-features -- -D warnings | Exit 0 |
| cargo test --no-default-features -- --test-threads=1 | 53 passed, 0 failed; 3 named private tests ignored |
| Selected private fixture_audit lane | 3 passed, 0 failed |
| RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features | Exit 0 |
| cargo build --no-default-features --target-dir target/clean | Exit 0, reused existing separate cache |
| cargo deny | Not applicable: no deny.toml; no vulnerability-scan claim |

Cargo.toml/Cargo.lock and dependency grants are unchanged. New Rust source/tests
carry AGPL-3.0-only. Existing workflow_dispatch-only CI is unchanged and was not
dispatched. This review does not claim hosted CI qualification or an all-platform
build; tests ran on the existing local target.
