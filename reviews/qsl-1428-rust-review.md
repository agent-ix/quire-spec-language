---
id: SR-1957035
title: "Rust review — QSL-1428 decision scope fixture"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/protocol_clause/tests/graph.rs; FR-218-AC-17; FR-218-AC-34; TC-1056"
review_set: subset
---

## Summary

Ticket: QSL-1428. Rust-review lane for PR #2010 at head `6abd763c8ecd69cce6e90d470ff58f72e83b82d3`, base `463cf467ab5bdd4c6321a112091591da40badd3b`. The only Rust change is a test fixture expression; it keeps the scope refusal path reachable and introduces no source, API, dependency, wire, or resource-boundary change.

## Verdict

**PASS** — the test remains deterministic, trace-bound, and behaviorally specific; the composite visible expression complies with the cited contract.

## Rust review

The existing helper constructs a real protocol and checker path. The changed `true and true` visible entry is a valid composite constant expression, so S3 reaches the sibling-branch guard read. The `unscoped` assertion still checks `missing_declaration`/`missing-name`, the inner `MissingName("rx")`, and the exact read location. No new panic path, unsafe code, public API, async behavior, or dependency surface is introduced. Formatting and changed-crate Clippy/test checks pass.

## Scope examined

- `qsl-semantics/src/check/protocol_clause/tests/graph.rs:739-741, 900-925`
- `spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md` (FR-218-AC-17 and FR-218-AC-34)
- `spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md` (rows 50 and 52)
- Frozen diff at PR #2010, head `6abd763c8ecd69cce6e90d470ff58f72e83b82d3`, base `463cf467ab5bdd4c6321a112091591da40badd3b`.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No findings (placeholder) | -    |

## Verification

- `cargo fmt --all -- --check` — exit 0.
- `CARGO_TARGET_DIR=/private/tmp/qsl-1428-review-target QSPEC_DIR=/Users/peter/dev/quire-specification cargo test --locked -p qsl-semantics a_decision_reads_only_listed_facts_of_its_owner --lib -- --nocapture` — exit 0; log: `/private/tmp/qsl-1428-review/focused-test.log`.
- `CARGO_TARGET_DIR=/private/tmp/qsl-1428-review-target cargo clippy --locked -p qsl-semantics --all-targets -- -D warnings` — exit 0; log: `/private/tmp/qsl-1428-review/clippy.log`.
- `cargo deny check` — unable to acquire `/Users/peter/.cargo/advisory-dbs/db.lock` because that path is read-only in this environment. The diff does not change dependency manifests or lockfiles.
- `cargo deny --locked check bans licenses sources` — exit 4: workspace license policy rejects existing AGPL-3.0-or-later and other workspace dependencies; the source and ban checks pass. No dependency files changed in this PR.
