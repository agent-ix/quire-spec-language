---
id: SR-909
title: "QSL-340 code review of PR 542 (IR reader depth ceiling consumed; tc_440 cases un-ignored)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a305f56ffc793e2470708c45af4c77fba094f3a1; Cargo.lock; qsl-package/src/checked_v2.rs; qsl-package/src/checked_v2/tests.rs; qsl-package/src/emit/extent_agreement.rs; quire-contract-model at ead72675 checked_package/shared.rs, common.rs, v2/lower.rs (read, context); qsl-semantics/src/family/requirements.rs (read, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: reviews
---
## Summary

Ticket: QSL-340. PR: quire-spec-language#542 at a305f56f, diff
`origin/main...HEAD` (merge base 7d0497be). Code review with the
rust-review lane folded in.

What was checked:

- **Lock move.** `Cargo.lock` changes one line: `quire-contract-model`
  7c700411 -> ead72675. No `Cargo.toml` change; both manifests track
  `branch = "main"`. No pin added.
- **Clamp.** `SERDE_JSON_RECURSION_LIMIT` is gone; `enforced()` clamps at
  `usize::try_from(CheckedPackageReadLimits::MAXIMUM_DEPTH).unwrap_or(usize::MAX)`.
  The constant is referenced, not copied. The fallback is exact: if `usize`
  cannot hold 16384, every `usize` depth is below 16384 and no clamp is
  needed, which `min(usize::MAX)` gives. On 32/64-bit the conversion never
  fails. This matches IR's own `read_value` (`limits.depth.min(MAXIMUM_DEPTH)`,
  common.rs:245) and `for_ir()`'s `u64::try_from(depth).unwrap_or(u64::MAX)`.
- **Incomplete arm.** `Incomplete(Depth, limit, consumed, path)` maps to
  `LimitExceeded(NestingDepth)` with a locus built by parsing IR's pointer
  text only; no re-parse of the deep bytes, so no serde recursion on the QSL
  side.
- **Test oracles.** `depth_far_past_the_default_limit_is_incomplete_not_malformed_wire`
  asserts `(128, 201)`: the configured bound and IR's measured depth for 200
  arrays around a scalar. `depth_past_the_charged_maximum_is_incomplete_naming_the_maximum`
  builds 16384 arrays as raw bytes (no deep `Value` on the test stack) and
  asserts `(16384, 16385)`, which proves IR charges a caller value above 128
  and names the charged maximum. `a_verified_read_records_depth_as_the_charged_maximum`
  exercises only QSL's own `enforced()` clamp; it fails if the clamp is removed
  (`usize::MAX != 16384`). Together the three bind QSL's recorded limit to
  IR's charged one.
- **Red runs.** At the old lock the crate does not compile
  (`MAXIMUM_DEPTH` absent, E0599, reviewer run), so "red at the old lock" is
  compile-red, not an assertion failure per test. Not a defect.
- **Ignored quantity test.** Run with `--ignored`, it fails at
  extent_agreement.rs:294 with `NamesAbsentNode` for the unit node, exactly
  as its ignore reason says. Out of scope here.
- **Rust idioms.** No new `unwrap`/`expect` in shipped code (test helper
  `maximum_depth()` unwraps, fine), no `unsafe`, no panics, clippy clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The TC-440 doc comment says IR's first unbounded node for `RangedTree` is "the member of the recursion group IR reaches first". IR picks the lowest `node_id` among reachable nodes (`ordered.sort_by(node_id)`, then `find(is_recursive_type ...)`, lower.rs:446-483). Which member that is depends on digest order, not on traversal. | qsl-package/src/emit/extent_agreement.rs:220-224 |
| FND-002 | low | The two new depth tests have no `#[trace]` tag. They are the only tests for NFR-007's new v2-reader exception text (incomplete naming the charged ceiling). Their neighbours are also untagged, but these two are new. | qsl-package/src/checked_v2/tests.rs:1114-1141 |
| FND-003 | low | `V2ReadLimits::depth` now lets a caller admit packages up to 16384 deep, but its doc drops IR's caveat (shared.rs:18-25): a package admitted past depth 128 is recursed over by every clone, comparison, `Debug` and drop of it, on the caller's stack. `V2Read` derives `Clone, Debug, Eq, PartialEq` and holds the admitted package. No production caller raises `depth` today (all use `default()`), so this is latent. | qsl-package/src/checked_v2.rs:184-186,563 |

## Verdict

No correctness bug in the shipped code. The clamp, the incomplete mapping
and the depth tests are right and their oracles are independent of the code
under test. Three low findings. The `RangedTree` expectation is filed under
gap analysis (SR-910 FND-001).
