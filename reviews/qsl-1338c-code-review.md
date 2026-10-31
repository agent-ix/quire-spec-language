---
id: SR-1938001
title: "code review of PR #1938 (QSL-1338 slice c): statistical and timed_trace test speed-up"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1938 git diff origin/main...8458dd8dd (Cargo.toml, qsl-analyze/src/statistical.rs, qsl-analyze/src/statistical/regenerative.rs, qsl-eval/tests/it/timed_trace.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-190
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-242
  type: references
---
# SR-1938001: code review of PR #1938 (QSL-1338 slice c)

## Summary

Ticket: QSL-1338. Code review with the rust-review lane folded in, scoped to the PR diff at head 8458dd8dd43cad71f09d6c36f394b540aa7aac20. Major bugs only. Focused runs: `cargo test -p qsl-analyze --lib statistical` 54/54; `cargo test -p qsl-eval --test it timed_trace::the_repeated_interval_forms` 1/1 (20.1 s); `cargo clippy -p qsl-analyze --all-targets --all-features -D warnings` clean. A local reviewer probe (not pushed) compared `interval_within(z, limit)` with `interval(z).filter(half_width <= limit)` over 3,000 random cycle sets and 30,000 limits, including the limit equal to the bound and the bound plus or minus 2^-70: 30,000 agreements, 18,357 accepted.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Tally held count and estimate | qsl-analyze/src/statistical.rs:287-352 | examined |
| sampled_test Bernoulli and fraction arms | qsl-analyze/src/statistical.rs:597-628 | examined |
| regenerative_test decision | qsl-analyze/src/statistical.rs:700-721 | examined |
| Cycles::interval_within / interval_screened | qsl-analyze/src/statistical/regenerative.rs:66-128 | examined |
| sqrt, quotient, normal_quantile_upper | qsl-analyze/src/statistical/bounds.rs:65-70,173-189,361-373 | context_only |
| the_repeated_interval_forms_agree_with_the_evaluator | qsl-eval/tests/it/timed_trace.rs:1097-1137 | examined |
| dev profile package overrides | Cargo.toml:187-204 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Approve: no findings. Mergeable.**

1. Held count. `held: u64` is bumped only where the old code added `int(1)` to `sum`. `sum` is now only the fraction sum, and the only reader of `sum` is `Tally::estimate`, which adds `held` back once. Rational addition is exact and commutative, so `sum + held` equals the old sum for any mix of samples. No partial sum is read anywhere else: SPRT reads only `test.count(held)`, the Okamoto stop reads `estimate()`. `held <= counted`, and `counted` is a `u64` bumped by the same `+= 1` and capped by `max_samples`, so `held` cannot overflow first. The regenerative path has its own local `held` Rational and never touches `Tally::held`.
2. Squared screen. The exact half-width is h = z·√spread / (L̄·√n). The screen returns `None` only when z²·spread > limit²·L̄²·n, which means h > limit, since z >= 0 (`normal_quantile_upper` searches from [0, 1] upwards), L̄ > 0 (an estimate exists only when Σ L ≠ 0, and weights are non-negative) and n >= 2. The old bound uses `sqrt(spread).upper` >= √spread and `sqrt(n).lower` in (0, √n], so the bound is >= h > limit and the old code rejected too. When the screen passes, the same bound is computed and `.filter(half_width <= limit)` applies the old non-strict test exactly. The screen can only reject runs the old test rejected and never accepts one, so results are identical, including at the boundary. A negative spread still reaches `sqrt` and faults as before. Moving `interval` to `#[cfg(test)]` leaves no production caller behind.
3. Assertions. Only the two ten-clock lasso counts change (80 to 20, accepted by the lead). The doc comment is updated to match. No assertion is removed or loosened. FR-242-AC-1's four formulas and their 1,000 lassos are in a different test and unchanged.
4. Profile. `[profile.dev.package.num-bigint]` and `[profile.dev.package.quire-exact]` set only `opt-level = 3`. A per-package override changes only the keys it names, so `debug-assertions` and `overflow-checks` stay on for both crates, as for the existing `sha2`/`serde_json`/quire-* overrides. Release builds are unaffected. Build speed only.

Rust lane: no `unsafe`, no new panics, no new casts (`Integer::from(u64)`), clippy clean with `-D warnings`.
