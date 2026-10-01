---
id: SR-941
title: "QSL-357 code review (with rust-review lane) of PR 552, quire-exact as no_std + alloc"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4e2dfb7d3ebffe389f1d8d1ceb3a1bbf2855626a; quire-exact/Cargo.toml; quire-exact/src/*.rs (lib.rs crate attributes, core/alloc imports); Makefile (quire-exact-no-std, ci); spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (K is a leaf); rust-toolchain.toml (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-357. PR: quire-spec-language#552 at 4e2dfb7d, diff `d81193f9...4e2dfb7d`.

What I checked, and what I found:

- **No std left in the library.** `grep std` over `quire-exact/src` finds only the
  `cfg(test)` crate import and the four `HashSet` uses in unit tests (`node.rs:130`,
  `identity.rs:342,366,436`). The crate has `#![no_std]` and `extern crate alloc`.
- **The `cfg(test)` std import cannot reach a non-test build.** `#[cfg(test)]
  #[macro_use] extern crate std;` exists only when rustc compiles this crate's own unit
  tests. Doctests and integration tests are separate crates. Measured: the library
  builds for `thumbv7em-none-eabi`, a target whose sysroot has no `std`, with no
  features and with `test-support`, and prints no warning.
- **The check really proves no std is linked, including through dependencies.** I
  mutated the dependency features without editing the branch:
  `--features unicode-normalization/std` and `--features num-traits/std` both fail with
  `E0463: can't find crate for std`. The coder's source-level mutation (a
  `std::collections::HashMap` use failing with E0433) covers the crate itself. The
  workspace uses resolver 2, so `-p quire-exact` resolves features for this crate alone,
  which is what a standalone consumer such as RT gets.
- **Dropping default features loses nothing at run time.** I read every
  `cfg(feature = "std")` site in the five dependencies at the locked versions.
  `num-bigint` 0.4.8: std only changes a `Vec` capacity estimate in radix parsing and
  printing, the initial guess of `nth_root`, and adds `std::error::Error` impls for
  `ParseBigIntError` and `TryFromBigIntError`. Results are identical. quire-exact calls
  `to_str_radix(10)` and `BigInt::from_str`, and charges by the output length, not the
  capacity, so metering is unchanged. It never exposes the two error types.
  `num-integer`/`num-traits`: std gates float methods and float roots, which quire-exact
  does not call (`ieee.rs` works on bits). `unicode-normalization` 0.1.25: std only
  switches `extern crate alloc as std` to `extern crate std`. The tables are always
  compiled. `thiserror` 2.0.20: std adds `Path` display and backtrace support, neither
  used.
- **Workspace unification.** In the std workspace, `num-bigint`'s std feature is turned
  back on through `num`/`fraction` (jsonschema). So QSL's own tests of quire-exact run
  num-bigint's std path, not its no_std path. Since the two paths give identical
  results, this is not a defect. `unicode-normalization` has no other dependent, so
  QSL's tests already exercise its no_std configuration.
- **No public signature changed type.** `alloc::sync::Arc`,
  `alloc::collections::BTreeMap`, `core::cmp::Ordering`, `core::fmt` and `core::str`
  are the same items std re-exports. The only `BTreeMap` signature is the private
  `match_members`.
- **Error impls still work with `?` and `Box<dyn Error>`.** thiserror 2 without std
  derives `::thiserror::__private::Error`, which is `pub use core::error::Error`
  (`thiserror-2.0.20/src/private.rs:11`). Since Rust 1.81, `std::error::Error` is that
  same trait, and the crate's `rust-version` is 1.98.
- **No CI workflow file changed.** The diff touches `Makefile` only. `.github/workflows/ci.yml`
  does not run `make ci`, so the new check runs in the local gate only. Local `make ci` is
  the gate at this stage.
- **QSL-356 (PR #554) overlap.** #554 edits ADR-011 at lines 562 (FB-05 row) and 1059
  (X-10 row). This PR edits lines 751-755. The hunks are far apart, so git merges them
  cleanly, and the two texts agree that K is a leaf.

Gate: the coder's `make ci` log on 4e2dfb7d ends `exit=0`, and its third step is the
`quire-exact-no-std` build. I did not re-run it. My own focused runs were the plain
no_std build (exit 0), the `test-support` build, and the two dependency-feature
mutations.

Rust-review lane. The changes are import moves only. There is no new `unwrap`, panic,
`unsafe` or integer conversion. `#![forbid(unsafe_code)]` stays. `core::iter::repeat_n`
and `core::mem::needs_drop` are the same functions as before.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `make ci` now needs the `thumbv7em-none-eabi` target installed, but `rust-toolchain.toml` does not list it (`components` only, no `targets`). On any machine that lacks the target, such as a fresh clone or another agent host, the first cargo step of `make ci` fails with `E0463: can't find crate for core`, not with a no_std regression. The gate passed here only because this host already had the target installed. Add `targets = ["thumbv7em-none-eabi"]` to `rust-toolchain.toml`, so rustup installs it with the pinned toolchain. | rust-toolchain.toml:1-4; Makefile:192-193 |
| FND-002 | low | `--no-default-features` in the check does nothing, because quire-exact declares no `default` feature. It would also do harm if a default feature were added later: the check would then skip exactly the configuration that a plain `quire-exact = { ... }` dependency, such as RT's, gets. Drop the flag, so the check builds what a consumer gets. | Makefile:193 |
| FND-003 | low | The edited "K is a leaf" bullet leaves a line of about 100 columns (`` `core` and `alloc`, so a no_std consumer depends on it directly. Each kernel `Value` and `ValueType` ``). The paragraph around it is wrapped at about 78. Re-wrap the paragraph. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:754-755 |

## Verdict

Correct and complete for the ticket's purpose. The crate links only `core` and
`alloc`, the check is strong and proven to fail on both a source and a dependency
regression, and no behaviour or public type changed for QSL's std callers. One
medium finding: the gate is not reproducible on a host without the bare-metal target.
Fix FND-001 (one line in `rust-toolchain.toml`) before merge. FND-002 and FND-003 are
low and can be fixed in the same round.

## Dispositions

Round 1, reviewed at 41ad03cc749475720bc919ae176677f65bb5981f (fix commit 41ad03cc on top of 676cee4d, rebased onto 6360c035). `git range-diff` shows 676cee4d is identical to 4e2dfb7d, and `quire-exact/` and the Makefile have no diff between them, so the library code the full gate passed on 4e2dfb7d is unchanged. I ran `make quire-exact-no-std` on 41ad03cc: it passes.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 41ad03cc: rust-toolchain.toml now lists `targets = ["thumbv7em-none-eabi"]` |
| FND-002 | fixed | 41ad03cc: the target is now `cargo build --locked -p quire-exact --target thumbv7em-none-eabi` |
| FND-003 | fixed | 41ad03cc: the "K is a leaf" paragraph is re-wrapped to about 78 columns |
