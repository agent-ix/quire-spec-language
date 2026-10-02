---
id: SR-1201
title: "Code review of quire-spec-language PR #591: fallible ModelCorrespondence::record"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@08430f21bacf87c9708c2ef9eeeaa7d5a60792d8; PR #591 diff against origin/main: qsl-semantics/src/check/identity.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/lowering/model/tests.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/state_clause.rs, spec/spec.md, spec/tests.md"
review_set: subset
---
# Code review of quire-spec-language PR #591

## Summary

Ticket: QSL-496. `ModelCorrespondence` gains a reverse map, and `record` now
returns `Result<(), Box<CorrespondenceConflict>>`. It is idempotent on an
identical pair and refuses a node rebound or a declaration rebound without
changing anything. Lowering now builds the `ModelCorrespondence` directly,
replacing the overwrite-on-insert `BTreeMap<DeclarationKey, NodeKey>`. That
removes `correspondence_entries` and the re-record loop in `check`, so a
conflict now faults at `model_node` as `KeyFault::CorrespondenceConflict`.

- Rust lane (rust-review): no panics, no unsafe code. The conflict is boxed for
  the `Result` size, and the error carries both pairs. `record` checks both
  directions before either insert, so a refusal leaves both maps unchanged.
  Derived `Eq` over both maps stays consistent because they are written
  together. `record` stays `pub(super)`. The lowering's
  `self.correspondence.record` is reachable because `lowering` is a child of
  `check`.
- Removing the intermediate Vec and the second record pass is a real
  simplification, not churn.
- Gates run at this head with a worktree-local target dir: `cargo test -p
  qsl-semantics --lib` passed (418 tests) and `cargo clippy -p qsl-semantics
  --all-targets -D warnings` was clean.

Per the team-leader ruling, FR-303-AC-4 is accepted as tested at the lowering
level, so it is not raised here.

## Verdict

Clean for code-review and rust-review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
