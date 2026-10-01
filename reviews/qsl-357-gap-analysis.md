---
id: SR-942
title: "QSL-357 gap analysis of PR 552: ticket scope against the change and its no_std check"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4e2dfb7d3ebffe389f1d8d1ceb3a1bbf2855626a; QSL-357 scope bullets (no_std + alloc, std feature only if needed, no_std check in make ci, no API change); quire-exact/Cargo.toml; quire-exact/src/lib.rs; Makefile; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-357. PR: quire-spec-language#552 at 4e2dfb7d.

No FR or TC owns this change. The ticket's scope bullets are the acceptance criteria,
and ADR-011 §6.1 "K is a leaf" is the one spec statement it amends. Each bullet against
the code:

- **quire-exact is `#![no_std]` with `extern crate alloc`, `std::` replaced by
  `core::`/`alloc::`.** Done (`quire-exact/src/lib.rs:99-107`). No `std` path remains
  outside `cfg(test)`.
- **A `std` feature only if something truly needs std.** No std feature was added, and
  none is needed: every dependency has `default-features = false` and the library
  builds for a target with no `std`. `Cargo.lock` is unchanged by the diff.
- **A no_std build check in `make ci`.** `quire-exact-no-std` is in the `ci`
  prerequisites (`Makefile:189-195`). Oracle strength: the target's sysroot ships no
  `std`, so any `std` use in the crate, or any dependency with its std feature back on,
  fails the build. I measured the dependency case: `--features unicode-normalization/std`
  and `--features num-traits/std` both fail with E0463. The coder measured the source
  case (E0433). A check that only built for the host with `--no-default-features` would
  not have caught either, since quire-exact has no default features. Reproducibility of
  the check is SR-941 FND-001.
- **No API change for QSL callers.** No public signature changed type. The rest of the
  workspace compiles with no edits (the coder's `make ci` log on 4e2dfb7d, exit 0).
- **Purpose: RT can depend on quire-exact directly.** The crate builds on
  `thumbv7em-none-eabi`, which has pointer atomics, so `alloc::sync::Arc` is available.
  A consumer target without atomic CAS (for example `thumbv6m`) would not have `Arc`.
  That is outside this ticket, since the ticket names no target.

ADR-011's bullet now states the crate is `#![no_std]` and uses only `core` and
`alloc`. That matches the code.

Not part of this PR: the ticket's post-merge step (tell ir-plan so RT IR-342/IR-345
can delete `src/exact`). That falls to whoever merges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Every scope bullet is met and backed by a check that fails on a regression. The only
reproducibility gap in the check is recorded as SR-941 FND-001.
