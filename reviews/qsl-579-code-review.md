---
id: SR-1213
title: "Code review of quire-spec-language PR #597: zone engine DBM (qsl-analyze) and checker DBM"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f26315fe45119052cc22bc46e56a8834868cd091; PR #597 diff against origin/main: qsl-analyze/**, qsl-replay/src/certificate.rs, qsl-replay/src/certificate/zone{.rs,/proofs.rs,/reference.rs,/tests.rs}, qsl-replay/src/lib.rs, qsl-replay/Cargo.toml, Cargo.toml, xtask/src/{string_edge,typestate_scan}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #597

## Summary

Ticket: QSL-579. The PR adds two pieces of code.
- The new layer-A crate `qsl-analyze` holds EN-6's `Dbm`. It uses an enum
  `Bound` over `BigInt` with a strictness flag, plus `scale_constants`.
- `qsl_replay::certificate::zone` holds the checker's own `Zone`. It encodes
  each bound as one integer (`2c` for `<`, `2c+1` for `<=`, `None` for
  `∞`), written once over an `Exact` trait. There are 18 Kani harnesses
  and a Fourier-Motzkin reference.

What the coordinator asked to check:
- **Independence: confirmed.**
  - `qsl-replay` has no dependency on `qsl-analyze`, and `qsl-analyze`
    depends only on `quire-exact` plus num-*/thiserror. Neither crate
    imports the other.
  - The encodings differ: an enum plus flag in the engine, a parity-encoded
    integer in the checker.
  - The algorithms differ. The engine's `constrain` relaxes in place
    through the two endpoints. The checker's uses the one-pass
    `prev[a][i] + new + prev[j][b]` formula over a snapshot. The engine's
    `close` exits early on a negative diagonal, while the checker's closes
    fully and then tests.
  - The only shared code is the kernel's exact integer types. That is the
    numeric substrate, not DBM logic.
- **Kani over i64 versus production over `Integer`: no real gap.**
  - The matrix code uses only `Exact::{of, plus, minus, negated, even,
    halved}`, comparisons and `Option`. No branch depends on the
    magnitude of a value, only on parity and order.
  - The i64 and `Integer` impls agree on every input the code reaches.
    `halved` truncates on i64 and floors on `Integer`, but it is called
    only on an even value (`bound` strips the strictness bit first), where
    the two agree.
  - So a run proven overflow-free over i64 computes the same result over
    `Integer`, and `Integer` cannot overflow.
  - Parity encoding on negatives is right: `-3 % 2 == -1` is odd on i64,
    and `Integer::is_even` is the kernel's own.
  - The encoded `plus` (`e1 + e2 - (n1 | n2)`) is exact for all four
    strictness combinations. I checked it by hand.
- **The Kani reference is a sound oracle.** `up` and `reset` are computed by
  Fourier-Motzkin elimination of the delay or reset clock, read by point
  membership, with no closure anywhere in the reference.
  - Points lie on a 1/6 grid. With at most 2 clocks and integer constants,
    clock regions need fractional parts on a 1/3 grid at most. The
    coordinate range (|p| <= 32 units) reaches past every sum of two
    bounds in [-8, 8] plus the reset value.
  - Canonical form is proven as a fixpoint of `close` together with
    pointwise equality. A relaxation fixpoint satisfies every triangle
    inequality, so it is canonical.
  - `includes` and aLU are checked against entrywise transcriptions. Their
    semantics are grounded separately: `includes` by the one-way point
    check in the harness, and both by `tc_700_checker_zone_agrees_with_the_reference_on_a_grid`,
    which compares aLU against the definition (`simulated`) on 600 seeded
    zones, with both verdicts above 50 occurrences each.
- **Engine `Dbm`.**
  - `close` is Floyd-Warshall. `constrain` closes over its two endpoints,
    which is correct on a canonical input.
  - `reset` and `up` are the standard canonical-preserving forms.
  - `includes` is entrywise on canonical, non-empty inputs.
  - No index can go out of range (every public index is checked first), so
    nothing panics, and `BigInt` sums cannot overflow.
- **xtask scan lists.** `qsl-analyze/src` is in `string_edge::crate_roots`
  (and that function's test) and in `typestate_scan::QSL_CRATES`.
  arch-lint is another matter; see SR-1214 FND-001.
- Rust lane (rust-review): `forbid(unsafe_code)`, `clippy::all` denied,
  docs present on public items, no `unwrap` or `expect` on production
  paths.
- Gates run at this head with a worktree-local target dir: `cargo test -p
  qsl-analyze` passed (5 tests, including the 10,000-DBM windowed AC-3
  test), as did `cargo test -p qsl-replay --lib certificate` (2 tests)
  and `cargo test -p xtask`. `cargo clippy -p qsl-analyze -p qsl-replay
  --all-targets -D warnings` was clean. I did not re-run Kani.

Rulings applied and not raised: the `Result` signatures, the windowed AC-3
enumeration, and Kani staying out of `make ci`.

## Verdict

Changes requested: two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `Exact` is a public trait, and `Zone<W: Exact = Integer>` is public, so a downstream crate can implement `Exact` for a machine integer with wrapping arithmetic and run the checker on it. That voids the "no bound too large" guarantee the module doc states. Production only ever needs `Integer`, and the i64 impl exists only under `cfg(kani)`. Seal the trait with a private supertrait, so the in-crate Kani impl still compiles and no outside impl can. | qsl-replay/src/certificate/zone.rs:27-45; qsl-replay/src/certificate/zone.rs:104-109 |
| FND-002 | low | `pub mod certificate` adds checker internals (`Zone`, `LuBounds`, `ZoneError`, `Exact`) to the public API of the layer-6 facade crate that CG reaches, with no consumer yet. qsl-replay's crate doc still lists the facade's public API as the replay entry and the four envelopes. Either keep the module crate-private until the checker uses it, or name it in the crate doc as deliberate public API. | qsl-replay/src/lib.rs:6-16; qsl-replay/src/lib.rs:32 |

## Dispositions

Round 1, reviewed at `3653e30e5fc134c8090168417efd3a099657cc80`. The branch is rebased onto main b8a79a9d.
`git range-diff` shows the four original commits identical (`=`), plus
the fix commit cf5e9a89 and 3653e30e, which only commits review files. The
fix deletes nothing beyond the `pub` on `certificate` and two `.clone()`
calls in a test. The arch-lint touches only add `qsl-analyze/src`: to
`qsl_scan_src_roots` and its two fixtures in api_surface.rs, to the test
`ROOTS` in canonical_encoder.rs (count 11 to 12), and to the tc_021 fixture
in main.rs. arch-lint, qsl-analyze, qsl-replay certificate tests and clippy
pass.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cf5e9a89d9dce876f7d54fe985996c567129bd88 |
| FND-002 | fixed | cf5e9a89d9dce876f7d54fe985996c567129bd88 |
