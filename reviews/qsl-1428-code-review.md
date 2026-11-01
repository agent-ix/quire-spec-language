---
id: SR-1957034
title: "Code review — QSL-1428 decision scope fixture"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/protocol_clause/tests/graph.rs; FR-218-AC-17; FR-218-AC-34; TC-1056"
review_set: subset
---

## Summary

Ticket: QSL-1428. Reviewed PR #2010's fixture correction against the decision-scope and visible-entry requirements. The composite visible entry is permitted by FR-218-AC-34, while the test still asserts a typed scope refusal for the sibling-branch `rx` read.

## Verdict

**PASS** — the valid composite reaches the intended scope assertion without changing checker behavior or weakening the oracle.

## Review notes

The changed case still reads `rx.ready` in the guard after the receive binder is declared in a sibling branch. `unscoped` rejects any result other than `missing_declaration`/`missing-name`, checks the inner `MissingName("rx")`, and checks the location. `true and true` is a composite expression over constants; FR-218-AC-34 permits literals inside composite visible expressions. The focused test, formatter, and clippy checks passed.

`quire matrix --strict` emitted initialization warnings and did not complete during this review; it was interrupted. Manual trace inspection confirmed the existing `#[trace("TC-1056", "FR-218-AC-17")]` binding. No production code, dependencies, or requirements changed, so no gap-analysis or dependency gate applied.

## Scope examined

- `qsl-semantics/src/check/protocol_clause/tests/graph.rs:739-741, 900-925`
- `spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md`
- `spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md`
- QSL-1428 issue and PR #2010 diff, head `6abd763c8ecd69cce6e90d470ff58f72e83b82d3`, base `463cf467ab5bdd4c6321a112091591da40badd3b`.

## Findings

| ID      | Severity | Summary                    | Refs |
| ------- | -------- | -------------------------- | ---- |
| FND-001 | low      | No findings (placeholder)  | -    |

## Verification

- `cargo fmt --all -- --check` — exit 0.
- `CARGO_TARGET_DIR=/private/tmp/qsl-1428-review-target QSPEC_DIR=/Users/peter/dev/quire-specification cargo test --locked -p qsl-semantics a_decision_reads_only_listed_facts_of_its_owner --lib -- --nocapture` — exit 0; log: `/private/tmp/qsl-1428-review/focused-test.log`.
- `CARGO_TARGET_DIR=/private/tmp/qsl-1428-review-target cargo clippy --locked -p qsl-semantics --all-targets -- -D warnings` — exit 0; log: `/private/tmp/qsl-1428-review/clippy.log`.
- `quire matrix --strict` — did not complete; interrupted after about 40 seconds with no matrix result.
