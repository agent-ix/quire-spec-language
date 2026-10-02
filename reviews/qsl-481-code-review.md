---
id: SR-1217
title: "Code review of quire-spec-language PR #600: identity preimages of a checked body have a fixed depth"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0657b2b9a9065d4517a0c99ecbceedda8a414afd; PR #600 diff against origin/main: qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/identity_depth.rs, qsl-semantics/src/check/family.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/lowering/tests.rs, qsl-semantics/src/check/lowering/tests/{depth,expression_depth,identity_depth}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #600

## Summary

Ticket: QSL-481 (B0). The PR adds tests only. They check that every identity
preimage of a checked body has a depth fixed by its schema, whatever the
source depth. No production code changes.

What the coordinator asked to check:
- **The oracle proves the property.** Each test compares the deepest JSON
  nesting at a shallow and a deep source depth:
  - 24 expression forms at 4 and 63 levels;
  - nested `Option` at 4 and 100;
  - a record chain at 4 and 30;
  - the emitted v2 wire for `and`, `else if` and `let` at 4 and 100.
  A preimage that nested its operands grows about one level per source level,
  so "equal max depth" catches it. Two sample points cannot prove "fixed for
  every depth", but they catch every linear growth, and linear growth is the
  regression to guard against.
- **`json_depth` counts correctly.** It skips brackets inside strings and
  handles escapes, including `\"` and `\\`. On valid JSON the count is exact,
  and malformed input saturates instead of panicking. The unit test covers an
  escaped quote inside a string.
- **The coder's mutation evidence is misattributed.** I inlined
  `typed_application`'s term on its own. The check then refuses at 4 levels
  with `NodePreimage(OwnedApplication)` and panics at the "checks" unwrap
  (identity_depth.rs:57). That is the existing node-key guard, not the depth
  assertion. I then also disabled the `OwnedApplication` guard
  (node_key/mod.rs:1347). With both changes the depth assertion fails:
  `And: left 14, right 132`. So the oracle works, but the evidence for that
  is this double mutation, not the single one. Both mutations are reverted
  and the tree is clean.
- **The widening is test-only.** `Form`, `FORMS`, both
  `check_on_small_stack` functions and `chain` become `pub(super)` inside
  `lowering/tests`, which is `#[cfg(test)] mod tests` (lowering.rs:4122).
  `json_depth` lives in `family::fixtures`, which is
  `#[cfg(any(test, feature = "test-support"))]`. Its re-export in
  `check/mod.rs` has the same gate. qsl-package takes `test-support` only as
  a dev-dependency.
- **The tests do not depend on `MAX_CHECKING_DEPTH`.** No new code names
  it. `DEEP = 63` is below today's cap, so every form is admitted at the
  default limits, and those inputs stay admitted after the cap is deleted.
  The package test's 100 levels is also below 128.
- Rust lane (rust-review): no panics outside tests. `json_depth` is a
  single pass with no recursion.
- Gates run at this head with a worktree-local target dir:
  - `cargo test -p qsl-semantics --lib identity_depth` passed (4 tests).
  - `cargo test -p qsl-package --lib identity_depth` passed (1 test).
  - `cargo clippy -p qsl-semantics -p qsl-package --all-targets -D warnings`
    was clean.
  - `cargo fmt --check` was clean.

## Verdict

Mergeable with one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `wire_depth` measures the whole emitted v2 wire, not the identity preimage the test is named for. As an upper bound it is sound, because `identity_preimage` is a member of the wire, so growth there grows the wire. But a deeper non-identity section (`source_map`, `diagnostics`) would fail a test named for the identity preimage. Fix: measure the wire's `identity_preimage` member, or rename the test to the wire. | qsl-package/src/emit/tests/identity_depth.rs:23-27 |

## Dispositions

Round 1, reviewed at d48ce4286bedc187447f31e33b35e73cc86ef3e7.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d48ce428: `identity_preimage_depth` measures the wire's `identity_preimage` member only. |
