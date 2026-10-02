---
id: SR-738
title: "PR 490 kernel refusal records code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; quire-exact/src/outcome.rs; quire-exact/src/decimal.rs; quire-exact/src/division.rs; quire-exact/src/ieee.rs; quire-exact/src/numeric.rs; quire-exact/src/text.rs; quire-exact/src/lib.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-foundation/src/diagnostic.rs; qsl-foundation/src/diagnostic/stage.rs; qsl-foundation/tests/kernel_refusal_record.rs; qsl-semantics/src/value/quantity.rs; qsl-replay/src/spine/call.rs; qsl-replay/src/spine/call/tests.rs; qsl-package/src/emit.rs; src/linking/composed/definition_source.rs; the 1-draft.8 comment edits; the ~60 changed test sites"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#490, diff `origin/main...HEAD`.
Code review with the Rust lane (rust-review).

These parts are sound:

- **Raise sites:** each of the ten payloads is filled from the right target. `finish_rounding` passes the result width. `convert_width` passes `target` as the target and `value.width` as the source. `to_exact` passes the grammar-named `Rational` domain. `integer_arithmetic` and `rational_arithmetic` pass the failed bound. `check_length` passes the declared `TextType`. The decimal sites pass the `DecimalType` being placed at. `paired` and `euclidean_remainder` pass the bounded consumer interval. `refused_interval` returns `CheckedInvariant` for the unbounded `Mathematical` domain, which is fine because that domain can never refuse a member.
- **Integer-target retargeting:** in `qsl-semantics` `quantity.rs`, an integer target's strict-`exact` refusal is rewritten to `InexactTarget::Integer(declared Int)`. Removing that rewrite makes `integer_target_places_at_scale_zero_then_admits_the_integer_domain` fail.
- **Coerce and count:** both carry the node's own `Int` domain.
- **`sum` in qsl-eval:** the seed decision is uncharged and located at the body node. An addition's `IntegerOutOfDomain` becomes `Undefined::SumOutOfDomain` at the `sum` node. Nothing is charged after the failed decision, because `integer_arithmetic` refuses before `result-retain`. `Halt::Located` follows the same imported-call override as `Halt::Stop`. `Halt` has only one match site, and it handles the new variant.
- **Copy and Hash removal:** the whole workspace compiles, including `qsl-route`. No crate outside this workspace depends on `quire-exact`: a grep of every `Cargo.toml` under ~/dev finds only QSL checkouts. Nothing used `Refusal` as a hash key. `code()` and `cause()` now take `&self`.
- **Boxing and size:** the bigint domains are boxed. `TextType` and `IeeeWidth` are small `Copy` values. `ForeignReference` (2 x 32 bytes) already set `Refusal`'s size.
- **Panics and integer conversions:** no new `unwrap` or `expect` on a production path, no new integer casts, and no unbounded work. The spelled strings are bounded by bounds that checking already admitted.
- **`kernel_refusal_record`:** it reads the code and cause from `Refusal::code()` and `Refusal::cause()`, and each field from the variant. The spellings match catalog 1-draft.8 (`Int[lo, hi]`, `Decimal[lo, hi; smin, smax]`, `Rational[lo, hi; dmin, dmax]`, `Text[min, max; profile]`, `binary32`/`binary64`, and flags in vocabulary order joined by `,`). `CATALOG_CATEGORIES` gains the ten codes as refusal.
- **Test-site edits:** the `assert_eq` asserts that became `matches!(.. { .. })` compared payload-less variants before this PR. They lost no oracle they had. What is still missing is an oracle for the new payload (FND-001).
- **Revision claim:** `definition_source.rs`, `emit.rs` and their tests now read 1-draft.8. None of QSpec's JSON fixtures names the diagnostics revision, so the conformance vectors are unaffected.

## Verdict

**Changes requested** (one medium finding). The code is correct. The weakness is in the tests: a wrong payload at a raise site would pass the whole suite.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Most raise sites have no test that checks the new payload, so a wrong payload passes the suite. Two mutants survive `cargo test -p quire-exact -p qsl-semantics -p qsl-eval -p qsl-replay -p qsl-foundation`. (1) Swapping `target`/`source` in `convert_width`'s `IeeeNanPayloadNotRepresentable`: every NaN-refusal test matches `{ .. }`. (2) Hard-wiring `IeeeNotExact { target: IeeeWidth::Binary32 }`: every Binary64 site matches `..` (`float_rounding.rs:124`, `ieee_profiles.rs:1526`). `ModuloOutOfDomain`, `RationalOutOfDomain`, integer-arithmetic, Coerce, count and quantity-integer `IntegerOutOfDomain`, and decimal `InexactDecimal` have no payload oracle at any raise site. The only payload checks are `DecimalOutOfDomain` (decimal.rs unit test), `TextLengthOutOfDomain` (text.rs), `DivisionPairOutOfDomain` for signed-64, `IeeeRationalOutOfDomain` (f27), Binary32 `IeeeNotExact`, and integer-target `InexactDecimal`. The record tests in `kernel_refusal_record.rs` build the refusals by hand, so they cannot catch this. Fix: assert the whole value at the three NaN sites (`target: Binary32, source: Binary64`), bind `target` at a Binary64 `IeeeNotExact` site, and compare the domain at one modulo, one rational and one integer-arithmetic or Coerce site. | quire-exact/src/ieee.rs:1156-1159; quire-exact/src/ieee.rs:1673-1676; qsl-semantics/tests/it/ieee_profiles.rs:878; qsl-semantics/tests/it/ieee_profiles.rs:2025; qsl-semantics/tests/it/ieee_profiles.rs:2062; qsl-semantics/tests/it/ieee_profiles.rs:1526; qsl-eval/tests/it/float_rounding.rs:124 |
| FND-002 | low | The blanket `1-draft.7` to `1-draft.8` swap made one comment wrong. `LimitKind`'s doc now says "the four `1-draft.8` kinds", but the QSpec catalog says `token-count-exceeded`, `edge-count-exceeded`, `occurrence-count-exceeded` and `diagnostic-count-exceeded` are revision `1-draft.7`. The claimed revision on line 18 is right; line 19's "four `1-draft.8` kinds" should read `1-draft.7`. | qsl-foundation/src/diagnostic/stage.rs:18-19 |

## Dispositions

Round 1, re-checked against the fix diff. Mutants were re-run in a scratch worktree, which was then deleted.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | (with the asymmetric rational domain): the new `qsl-eval/tests/it/kernel_refusal_payloads.rs` raises each value refusal through the real kernel entry point and checks every rendered field. `coerce_and_count_refusals_carry_the_declared_domain` covers Coerce and count, and the quantity integer sites now compare the domain. Four mutants that previously survived or were never checked are now killed: the NaN target/source swap, `IeeeNotExact` hard-wired to Binary32, modulo hard-wired to `Int[0, 0]`, and Coerce hard-wired to `Int[0, 0]`. |
| FND-002 | fixed | `stage.rs:19` reads "the four `1-draft.7` kinds" |
