---
id: SR-11622
title: "Code review of quire-spec-language PR #881: quire-exact NonZeroInteger divisor"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@dffe4bea1e207f2bb08c3e12ad54b8617abe91ed; PR #881 diff vs main: Cargo.lock, qsl-replay/src/certificate/zone.rs"
review_set: subset
---
# Code review of quire-spec-language PR #881

## Summary

Ticket: QSL-803. Code review with the rust-review lane, one pass over
dffe4bea1 vs main. Oracle: quire-exact d18927e..37aa172 (PR #49, FR-374).
Reviewer-only: no builds run.

Examined:
- Cargo.lock: exactly one line moves, the quire-exact source rev
  d18927e8 -> 37aa1721ef087c972b216c54724ac4e48942405b. That commit exists in
  the local quire-exact checkout, is on origin/main, and is "Require borrowed
  nonzero Integer admission for division helpers (#49)". The lock holds a
  single quire-exact entry, so no second copy is pinned elsewhere. Cargo.toml
  is unchanged (branch=main) (clean).
- quire-exact public API changes d18927e..37aa172: `exact_div`,
  `div_rem_truncating` and `div_mod_floor` now take `NonZeroInteger<'_>`;
  new exports `NonZeroInteger` and `ZeroDivisor`. No other public signature
  changes (decimal.rs, division.rs, rational.rs changes are internal)
  (clean).
- Callers: a word-boundary grep of every `.rs` file in the workspace (all
  crates, tests, benches, kani proofs, xtask, tools) finds one call of the
  three helpers, zone.rs:80, the adapted one. No missed caller (clean).
- `NonZeroInteger::try_from(&two)`: the impl returns `Err` only when
  `value.is_zero()`, so the literal two always admits; the `expect` cannot
  fire. `expect` with a stated invariant matches production use elsewhere in
  qsl-replay (witness.rs:107). Workspace clippy is `all = "deny"`, which does
  not include `expect_used` (clean).
- Semantics: `div_mod_floor` still calls num-bigint `div_mod_floor` on the
  same operands; only the argument type changed, so floor results for
  negative or odd dividends are unchanged. `halved` is only called from
  `Zone::bound` on `twice`, which is always even (2c), so floor and
  truncation agree and the Kani i64 `self / 2` stays equivalent (clean).
- Kani cfg(kani) i64 impl (zone.rs:86-111) is untouched and uses no
  quire-exact API; the Integer impl it sits beside compiles the same under
  kani cfg (clean).
- Tests: `Zone` defaults to `W = Integer`; the zone tests decode random
  zones with constants in [-4, 4] through `reference::raw_of` ->
  `decode` -> `Zone::bound` -> `halved`, so the adapted line is exercised
  behaviorally, negatives included (clean).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS, no findings.** The pin moves one crate to the real 37aa172, the only
caller is adapted, the admission of two is infallible, the result of
`halved` is unchanged, and existing zone tests exercise it.

Recorded, not blocking: the token could be built once per call without the
named `two` binding, a style nit only.
