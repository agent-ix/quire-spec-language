---
id: SR-2026111204485700000001
title: "code review of PR 2686 PathGuard constructor and path-condition test seam"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/claims.rs, qsl-semantics/src/check/claims/tests.rs"
review_set: subset
---

## Summary

Ticket: IR-1522. PR agent-ix/quire-spec-language#2686 at head 4cd23e9dc, diff origin/main...HEAD over 2 files (+41 lines). The PR adds an ungated, documented `PathGuard::new(OccurrenceKey, bool)` and a `RequirementRecord::with_path_condition(Vec<PathGuard>)` behind `#[cfg(any(test, feature = "test-support"))]`, which is the same gate as the existing `with_owner` seam, plus one unit test. The rust-review lane is folded in. Gates on the head are listed in the Verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `PathGuard::new` doc comment describes things the code does not do. It says a caller "gets a record the driver refuses". That is the behaviour of a downstream crate (quire-driver), which QSL neither implements nor tests, and the constructor returns a guard, not a record. It also does not say that the occurrence's role is unchecked, although the struct doc says a guard is the node's `expression` occurrence, so a caller can build a guard with any role. Fix: say that neither the node nor the role is validated against a checked graph (the checker records `expression`), and drop the driver claim. | qsl-semantics/src/check/claims.rs:295-297 |

## Verdict

CONDITIONAL. The only finding is a low doc-wording defect. It does not block merge and can be fixed in the same PR or left as it is.

On the design note (gated `with_path_condition`, ungated `PathGuard::new`): this does satisfy "public" for the consumer. ADR-011 X-6 requires `qsl-semantics`' test-only items to sit behind `test-support`, which only a `[dev-dependencies]` entry enables. The ticket itself calls the identically gated `with_owner` a "public owner-replacement test seam". qsl-replay already reaches `with_owner` through this gate from a downstream crate (`call_requirements.rs` tests, FR-395-AC-10). Ungating a record mutator in production would let a caller forge checked records, which is worse. Downstream follow-up, not a defect of this PR: quire-driver's `Cargo.toml` on main has no `qsl-semantics` dev-dependency with `test-support` yet. Its FR-020-AC-14 test needs one: `qsl-semantics = { git = "...quire-spec-language", branch = "main", features = ["test-support"] }` under `[dev-dependencies]`. Cargo unifies that feature into test builds only. quire-driver's `tests/dependency_edges.rs` checks only normal and build edges, so this dev-only edge does not trip it. A driver test can then do `records.values_mut()` + `with_path_condition(vec![PathGuard::new(ghost, b)])`, which is exactly the qsl-replay pattern.

What is sound: no checking behaviour changed. The checker still builds guards through the private struct literal (claims.rs:1125), and no production path accepts a caller-built guard into a record. Both items are `#[must_use]` and documented, which matches `with_owner`. `PathGuard::new` stays ungated, as the ticket asks, and building a guard value alone confers nothing. The test is not tautological where it matters. It first asserts that the ghost node (node_key 250) is absent from every occurrence of the checked graph, then that the record carries exactly that guard and outcome. It has no `#[trace]` tag, but no QSL criterion owns this seam and the repo does not require a tag on every test. No unsafe code, no new dependencies, no vendoring, no ceremony.

Gates (focused, per brief) all pass; see Coverage.

## Coverage

- `cargo fmt --check -p qsl-semantics`: pass.
- `cargo test -p qsl-semantics --lib a_record_can_carry_a_guard`: pass (1 passed, 0 failed).
- `cargo clippy -p qsl-semantics --lib --tests --features test-support -- -D warnings`: pass (no warnings).
- Full workspace clippy, `cargo deny` and full tests were not run: the brief asked for focused checks only, and the full gate runs once before merge.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in 78af12401: the doc no longer claims driver behaviour or calls the guard a record, and says neither node nor role is checked against a graph and that a guard is the node's `expression` occurrence. |
