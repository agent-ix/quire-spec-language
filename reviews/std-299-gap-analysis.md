---
id: SR-15533
title: "gap analysis of STD-299 no_std build contract (QSL PR #1223)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@44f497b8ced379b560d10c2707c97b643ca609e4; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md, spec/test-cases/TC-1110-qsl-foundation-builds-no-std-on-the-governed-target.md, spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1111
    type: reviews
---
# SR-15533: gap analysis of STD-299 no_std build contract (QSL PR #1223)

## Summary

Ticket: STD-299. This is a spec-only PR. Plan completion: not assessed. AC-to-TC trace:

- TC-1110 covers AC-1 (step 2), AC-2 (step 3), AC-3 (step 4) and AC-8 (steps 5 and 6).
- TC-1111 covers AC-4 (steps 1-3), AC-5 (step 4), AC-6 (step 5) and AC-7 (step 6).

Each step has an expected result. Nothing is implemented yet: on main, `qsl-foundation` and
`qsl-package` have no `std` feature and there is no `build-no-std` target.

The gate design holds. Each command selects one package with `-p`. The reviewer measured AC-2 on a
scratch copy with `-p qsl-foundation --no-default-features --target thumbv7em-none-eabi`, and the
feature graph had no `std` feature anywhere. Resolver 2 unifies only the selected package's graph,
so the root crate's `std` edges do not leak in.

The existing `ci-clean-build` (`--workspace --no-default-features`) still builds, because every
dependent takes these crates with default features.

Coder note: with the exact root attribute AC-1 names, `cargo test --no-default-features` (AC-3)
compiles the unit-test modules `no_std`. On main, `setting.rs:340`, `diagnostic.rs:1492-1531` and
`diagnostic/stage.rs:232-300` use `vec!`, `Vec`, `String` and `to_owned` from the prelude, and they
need `alloc` imports.

`make check-spec-validation` with `QSPEC_DIR` set passes (exit 0).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: AC-2 (and TC-1110 expected result 3) says the `cargo tree ... -e normal,features` normal dependencies "are exactly `quire-exact`, `serde`, `sha2`, `thiserror` and the `ix-trace-rs` proc macro". Without `--depth 1`, that tree lists 33 crates, as measured, so the AC cannot pass as written. Add `--depth 1` for the exact-list half, and keep the full tree for the "no `std` feature" half. In AC-4, the `--depth 1` check that the tree "names neither ... nor `serde_json`" can never fail for `serde_json`, which is never a direct dependency. It is still in the closure, through `qsl-semantics`. Either drop it or say it is the crate-tier check. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:175, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:177, spec/test-cases/TC-1110-qsl-foundation-builds-no-std-on-the-governed-target.md:22-23 |
| FND-002 | low | minor: `make build-no-std` runs `cargo build` and `cargo check` only. The crates set `clippy::all = "deny"` and `missing_docs = "warn"`, but neither runs with `std` off, and `ci-default-features` and `ci-all-features` never build that configuration. Unused imports and dead `cfg` code that exist only in the `std`-off build would show up as warnings and never fail. Add `-D warnings` (or a `cargo clippy ... --no-default-features` line) to the gate and to AC-8. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:181, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:138-145 |

## Verdict

Two minor findings in this method. Every AC has a TC step with an expected result. The majors are
SR-15530 FND-001 and FND-002.

## Dispositions

Round 1 reviewed QSL #1223 at c6e3c941e. `make check-spec-validation` with `QSPEC_DIR` set passes
(exit 0). `cargo tree -p qsl-semantics -e normal -i quire-contract-model` finds no path, so the new
AC-4 inverse-tree check can pass once `qsl-package` gates its own edge.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
