---
id: SR-112
title: "PR review — verified native package reconstruction"
type: SpecReview
analysis: code-review
scope: "PR #12; Task-017/018 reader, runtime integration and producer review SR-111"
review_set: subset
evaluated_revision: "eb97a871f107eecebf49b1a53cc46e23ddc1d7ae"
review_date: "2026-09-09"
---

## Summary

The reader accepts packages only after exact byte selection, closed Serde
decoding, external binding checks and actual parse/link/check reconstruction.
Freshly reconstructed packages execute the qualified native runtime workflows.
This completes the native payload implementation in PR #12.

## Verdict

**PASS** for the PR's implementation. Independent FS05 acceptance, executable
IR lowering, compiled ConfigVersion/backend qualification and Quire integration
remain separate work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved implementation findings. | Task-017, Task-018 |

The PR gate found a concrete diagnostic-order defect: a tagged key containing
an unused field before an ill-typed name reported the later name error.
The failing control now passes with Serde-derived closed variants. Intake
requires string discriminants before Serde buffering; record decoding requires
objects even inside buffered variants. Numeric enum indices and positional
record arrays refuse. These are contract corrections, not changed semantics.

## Method and evidence

Applied the actual `/home/peter/dev/agent-skills/code-review/SKILL.md` and
`rust-review/SKILL.md`, Rust-style defaults subordinate to repository guidance,
and implementation-gap discovery. No applicable AssuranceProfile, repository
Rust override or deny.toml exists. The existing specification/reviews remain
41da6e5/69588ad, with source-fixture correction 2c6b9b8/1c3aa50. SR-111 supplies
the completed producer audit; optional semantic gap review remains declined.

| Contract | Executed evidence |
| --- | --- |
| FR-020 AC-2..4 | `package_reading.rs` and `package_reading_cases/{mod,raw}.rs`: raw grammar/duplicate/UTF-8 failures; every retained record's extra, missing and wrong-typed members; real Draft 2020-12 validation; exact u64 spelling; simultaneous selector defects, definition digests and feature-set behavior. |
| FR-020 AC-5..8,10 | `package_reading_cases/{mod,authority}.rs`: external identity/authorship axes, conflicting or stale models, ordered claims, forged projections/obligations, and original parser/link/type/definedness diagnostics after successful setup. |
| FR-020 AC-9; NFR-007 | `package_reading_cases/limits.rs`, frontend-limit controls and private intake/encoding tests: all package passes and all 18 frontend limit fields, actual usage, hard/elevated boundaries, unchanged inputs and retries. Private string/pass controls explicitly identify maxima unreachable through earlier public ceilings. |
| FR-021 AC-2,4,5 | Representation/feature permutations preserve static identity and retain distinct original bytes; unchanged-identity projection forgeries and actual raw/IR/JCS digest substitutions refuse. Producer vectors/static mutations retain SR-111's AC-1,3,6 evidence. |
| FR-020 AC-1,11; IT-007 SC-01..08 | `package_runtime.rs`, frozen vectors and reader controls: original package dropped; current parent/graph/aggregate truth, exact events/costs, pre/post deleted captures/results, dangling/stale/incomplete/frame/budget refusals and fresh retries; original unlowered IR obligations remain addressable. |

The reader's initial successful compiler setup and genuine missing-API failure
remain in `data/native-packages/reader-successful-setup.txt` and
`reader-missing-api.txt`. Runtime integration assertions also preceded their
first execution. New qualification is Rust; no tests replace the compiler or
runtime with a double.

Production has no new unsafe, panic on external input, I/O, shared mutable
state, concurrency, fallback decoder or speculative cache. Serde owns grammar;
private adapters own budgets, closed shapes and native conversions. Separate
raw/static/IR identities and constructor privacy remain intact. Every new
behavior maps to FR-020/021 or NFR-007; no unowned behavior or stub was found.

## Local gates

All commands terminated with exit 0. Cargo phases used `nice -n 10`,
`--locked --offline --target-dir target -j 1`; test phases used one test thread.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed. |
| `cargo test --no-default-features -- --test-threads=1` | 258 ordinary tests and three compile-fail doctests passed. Final reader run below adds two controls; unchanged suites were not repeated. |
| `cargo test --no-default-features --test package_reading -- --test-threads=1` | All 18 passed, including the final inventory and ordered-array controls; 260 ordinary tests exercised across these runs. |
| `cargo build --no-default-features` | Passed using the existing cache. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --no-default-features --no-deps` | Passed. |
| `cargo run --bin fixture-audit -- self-test` / `model-bytes tests/fixtures` | Six negative controls/duplicate refusal and five historical model digests/pin passed. |
| `cargo run -- parse test:parent fixture:1 tests/fixtures/parent.native` / `format ...` | Both passed. |
| `cargo test --test fixture_audit -- --ignored --test-threads=1` | All three private audits passed; QUIRE_STATE_CORE selected an immutable git archive of standard e897f810a7356d4ce8fd19026221ebda7b65596f `proposals/state-core`. Temporary archive removed. |

Rust is pinned at 1.98.1 and IR at 690bde7. The existing serde_json pin gains
only the already-reviewed unbounded_depth feature; the reader's own 128-container
guard remains enforced. Cargo.lock and dependency versions/grants are unchanged.
The sole hosted workflow still accepts only workflow_dispatch; none was run.
