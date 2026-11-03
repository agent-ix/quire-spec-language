---
id: SR-20261103161225
title: "QSL-1568 r1 code review (Rust lane) of PR #2204"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2204 head bb13ab56e; qsl-eval/src/temporal/finite_window/run.rs; qsl-eval/src/temporal/finite_window/trace.rs; qsl-eval/tests/it/finite_window.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: references
---

## Summary

Ticket: QSL-1568. Reviewed PR #2204 at `bb13ab56e`: code-review with the rust-review lane folded in. The change is in `Run::require_order`. A same-stamp pair counts as ordered only when both keys exist, come from one authority and differ (`left.key != right.key`). Before, any pair of keys from one authority counted as ordered, with the canonical observation-identity step breaking the tie. The PR also updates the doc comments of `require_order` and `OrderKey`, and replaces the test that pinned the tie-break with `a_repeated_key_of_one_authority_is_not_a_usable_order`, traced to FR-501-AC-26.

Evidence run by this reviewer in a private worktree at `bb13ab56e` (target `~/dev/worktrees/pr2204-r1-target`, `CARGO_PROFILE_DEV_DEBUG=0`, via `locked-build.sh`, `QSPEC_DIR` set to a fresh QSpec worktree of `origin/main`):

- `cargo test -p qsl-eval --test it finite_window`: 29/29 pass.
- Mutation: restoring the identity tie-break (dropping `&& left.key != right.key`) is killed. The new test fails at the `until` assertion (`left: None`, expected `UnorderedTie { position: 2 }`).
- Mutation: calling `require_order` for every scan, so that `eventually` refuses too, is killed by the new test (`expect("evaluates")` on `eventually`) and by three other tests.
- `cargo fmt --check -p qsl-eval`: clean.
- qsl-replay lib tests could not run because the lib test target does not compile at this head: `E0004`, `LimitKind::TickCount` is not covered at `qsl-replay/src/source_inventory.rs:83`. The PR touches no qsl-replay file, so this is main-red and not caused by the PR (same family as QSL-1569). Replay's FR-501-AC-7 and AC-20 tests use distinct keys or no keys and do not depend on the repeated-key rule.

## Verdict

**APPROVE.** `require_order` is correct in both scan directions, for these reasons:

- Candidates arrive in canonical order, forward for future scans and reversed for past scans.
- Within one stamp, canonical order sorts by `(authority, key)` and puts keyless observations last. So any two equal keys of one authority are adjacent among the same-stamp candidates, and any change of authority or a missing key creates an adjacent unordered pair. Checking `windows(2)` is therefore enough, including for three or more observations at one stamp (for example A1, A1, A2).
- The refusal names `max(before, after)`, the later canonical index, in either direction.
- It is still gated on `scan.guard.is_some()`, which holds only for `until`, `release`, `since` and `triggered`. `eventually`, `always`, `once` and `historically` are unaffected.
- The finite-window plan has no next or previous node that could read the identity-fixed index order in any other way.

The canonical sort in `TimestampedTrace::admit`, and its mirror in `qsl-replay` (`ordered_timestamped_positions`), still uses observation identity as the last step, so position indices and the `result_identity` stay independent of listing order. The new test checks this for `eventually` across both listings.

A grep for an identity tie-break across `qsl-eval`, `qsl-replay` and `qsl-analyze` finds no other caller that relied on the old comparability rule. Replay maps `UnorderedTie` to `SameStampOrderUnavailable` unchanged. The doc comments on `require_order` and `OrderKey` now match the code.

The diff adds no `unwrap`, panic path, `#[allow]` or public API.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
