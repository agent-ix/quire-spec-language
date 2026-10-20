---
id: SR-12233
title: 'Code review of PR #1114: s3.rational_bits limit replaces the interim work-budget ceiling (QSL-883), code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@b71362855e2e88200dfa6999f51fadd84e390227; git diff origin/main...HEAD (PR #1114, branch qsl-883-impl): Cargo.lock, qsl-foundation/src/diagnostic/stage.rs, qsl-foundation/src/setting.rs, qsl-replay/src/limits.rs, qsl-replay/src/spine.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/probabilistic.rs, qsl-semantics/src/check/probabilistic/exact.rs, qsl-semantics/src/check/probabilistic/model.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/settings.rs, qsl-semantics/tests/it/probabilistic_declarations.rs; ticket QSL-883'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-255
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-096
  type: reviews
---

## Summary

Ticket: QSL-883. PR: quire-spec-language#1114, head b71362855. Methods: code-review with the rust-review lane folded in. Spec-to-code agreement and test coverage are in the gap analysis (SR-12234).

What was checked:

- **No interim path is left.** `RATIONAL_BITS_CEILING` and the work-budget refusal are gone from production code. The only `65_536` literals left in `qsl-semantics` are in test assertions of the default (`exact.rs` tests, `probabilistic_declarations.rs`), and in unrelated text-length fixtures. The two `pending QSL-883` markers left are in spec AC rows (SR-12234 FND-001).
- **Every rational-bits check uses the configured limit.** There are four call sites: weights in `model.rs` (random-parameter support and workload weights), the running weight sum (`model.rs` `probabilities`), and quantity literals in `typing.rs`. All four take `limits.rational_bits()` from the caller's `CheckingLimits`. Each passes that same value as `limit` into `limit_refusal`, which reports `CheckingLimitKind::RationalBits`, so the refusal names `s3.rational_bits` and the configured bound. In production code `CheckingLimits::default()` is used only under `#[cfg(test)]`.
- **The mappings are complete.** `LimitKind::RationalBits` is in `ALL`, has a `catalog_cause` and a `noun`. `Setting::S3RationalBits` is at stage `S3`. `checking_bounds`/`set_checking_bound`, `CheckingLimitKind::setting` and `TryFrom<Setting>`, and the replay spine stage arm are all updated. FR-255-AC-3's table-driven test (`qsl-replay/src/limits.rs`) reads FR-255's table, so the new row and its kind text `rational bits` are checked against the code.
- **The transitive quire-exact bump is acceptable.** `Cargo.lock` moves quire-exact from 2134917 to 1c6cbcb (branch=main), along with quire-semantic-value 48e1bc3 to ac0e137. The quire-exact source change is additive. It adds `ChargePoint::TemporalVisit` (with `ALL` going from 62 to 63) and a `Meter::cancel()` accessor, and changes test-only code in `equality.rs`. No QSL code uses `ChargePoint::ALL` or matches exhaustively on `ChargePoint`. The QSL matches found are over QSL's own charge enums. `cargo check --all-targets` passes for `qsl-analyze`, `qsl-bench` and `qsl-cst`, which are outside the PR's gate list. So the bump changes no QSL behaviour. The quire-semantic-value change adds `rational_bits` to `CheckingLimits` and switches `CheckedEquality` to `planned_equality_with_plan(...).equal()`, which gives the same verdict.
- **Narrow tests pass.** The commands, each run through `locked-build.sh`, were: `cargo test -p qsl-semantics --test it probabilistic` (26 passed), `cargo test -p qsl-semantics --lib -- refusal::tests exact` (20 passed), `cargo test -p qsl-foundation -p qsl-replay --lib -- limit stage setting`, and the `cargo check` above. All pass: 26, 20, 8 + 55 tests, and the check finishes clean.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| LimitKind::RationalBits | qsl-foundation/src/diagnostic/stage.rs | examined |
| Setting::S3RationalBits | qsl-foundation/src/setting.rs | examined |
| limit_refusal / exact_refusal / quantity_magnitude | qsl-semantics/src/check/probabilistic.rs | examined |
| exact::value | qsl-semantics/src/check/probabilistic/exact.rs | examined |
| weight / probabilities | qsl-semantics/src/check/probabilistic/model.rs | examined |
| ExprNode::Quantity arm | qsl-semantics/src/check/check/typing.rs | examined |
| CheckingLimitKind mappings | qsl-semantics/src/check/refusal.rs | examined |
| checking_bounds / set_checking_bound | qsl-semantics/src/check/settings.rs | examined |
| spine stage arm, limits test | qsl-replay/src/spine.rs, qsl-replay/src/limits.rs | examined |
| quire-exact 2134917..1c6cbcb, quire-semantic-value 48e1bc3..ac0e137 | Cargo.lock | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `limit_refusal(bits: u64, ceiling: u64, location)` takes two adjacent `u64` arguments that a caller can swap with no compiler error. The four call sites are correct today. Passing the `CheckingLimits` (or a small newtype for the ceiling) would make a swap impossible. | qsl-semantics/src/check/probabilistic.rs:415 |

## Verdict

Mergeable: no high or medium code findings. The interim path is deleted. Every check reads the configured `s3.rational_bits` and reports it. The transitive quire-exact bump is additive and does not change behaviour.
