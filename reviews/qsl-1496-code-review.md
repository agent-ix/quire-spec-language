---
id: SR-20261104173645
title: 'Code review of PR #2306: QSL-1496 TC-149 checked-route trigger identity tests, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@3ca35d2f5; git diff origin/main...3ca35d2f5 (PR #2306, branch qsl-1496-checked-route-trigger), trial-merged with origin/main 60ab2e913: qsl-eval/tests/it/checked_route_trigger.rs, qsl-eval/tests/it/main.rs, qsl-eval/tests/it/temporal_relation.rs; ticket QSL-1496'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1496. PR: quire-spec-language#2306, head 3ca35d2f5. Method: code-review with the rust-review lane. The PR is test-only. It adds six TC-149 tests over `evaluate_temporal_evidence` and adds the FR-052-AC-12 tag to three FR-526 correction-refusal tests.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Row 1 distinct identities, instance-bound valuation | qsl-eval/tests/it/checked_route_trigger.rs:67-99 | examined |
| Row 2 repeated delivery, in-call and across ledger | qsl-eval/tests/it/checked_route_trigger.rs:100-138 | examined |
| Row 3 one-byte change, retained IDB instance | qsl-eval/tests/it/checked_route_trigger.rs:139-175 | examined |
| Row 4 substitution attempts | qsl-eval/tests/it/checked_route_trigger.rs:176-251 | examined |
| Row 5 empty/absent identity refusals | qsl-eval/tests/it/checked_route_trigger.rs:252-289 | examined |
| Row 6 refusals leave no state | qsl-eval/tests/it/checked_route_trigger.rs:290-333 | examined |
| FR-052-AC-12 tag additions | qsl-eval/tests/it/temporal_relation.rs:160-247 | examined |
| Shared helpers call/expect_refusal/delivery/request | qsl-eval/tests/it/temporal_activation.rs:224-277,845-868 | context_only |
| Refusal record fields contradicted_identity, surplus_capture_* | qsl-eval/src/temporal/evidence/call.rs:1130-1155 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Row 4 coordinate check cannot catch a ledger keyed by (control, identity). Each `control` value runs a fresh request with one delivery and no ledger, so one instance is guaranteed whatever the route does with `control`. TC-149 row 4 asks for two deliveries of one identity with the coordinate varied. Fix: run c1, pass `result.ledger` into the c2 call with `IDA` under receipt `r2`, and assert `ledger.instance_count() == 1` and two receipts on the instance. | qsl-eval/tests/it/checked_route_trigger.rs:218-228 |
| FND-002 | low | Row 6 assertions on `before` cannot fail. `before` is cloned into each request and `evaluate_temporal_evidence` takes `&EvidenceRequest`, so no refused call can change it. The type system already gives "the refusal left no state"; the asserted lines guard nothing. Fix: delete the `before.instance_count()`/payload asserts, or keep only the final `ok` call (instance_count 2), which is the real check. | qsl-eval/tests/it/checked_route_trigger.rs:315-324 |
| FND-003 | low | Row 4 runs the contradiction request twice: once through `call` for the cause, once again with its own meter for `is_err` and the charge log. Run it once with a meter and pass the result to `expect_refusal`, as `temporal_relation.rs:174-177` does. `TriggerDelivery` is also spelled fully qualified three times (lines 48, 182) while the other evidence types are imported; import it. | qsl-eval/tests/it/checked_route_trigger.rs:202-217 |

## Verdict

Conditional. No correctness bug. FND-001 is a real test-strength gap on an AC-8 clause and should be fixed in this PR. FND-002 and FND-003 are cleanups.

- **Row 1 is a real differential.** The same request with `trigger` IDA gives `truth: None` and with IDB gives `Some(True)`, because only IDB has an `instance_valuations` entry for the instance-bound `settled`. A route that shared valuations across identities fails it.
- **Rows 2 and 3 carry the ledger across calls** and assert `instance_count`, receipts and retained captures by identity lookup. Dropping the receipt append or keying on receipt fails them.
- **Row 5 covers each refusal cause** (`trigger-required`, `trigger-unknown`, `surplus-capture`), the `EmptyIdentity` constructor and `trigger_of`.
- **Trace tags** match the AC each test asserts. FR-052-AC-12 on the three refusal tests is correct for the self/non-direct/cross-obligation/identity-mismatch refusals.
- **No ceremony, no compat layer, no weakened gate, no QSpec content** (QSpec named by id only in the existing spec text). No production code changed.
- **Gates on the trial merge with origin/main 60ab2e913:** `cargo check --locked --workspace --all-targets` passed; `cargo clippy --workspace --all-targets -D warnings` passed with default features and with `--all-features`; `cargo test -p qsl-eval --test it -- checked_route_trigger temporal_relation` passed 15/15.

## Dispositions

- FND-001: fixed. Row 4 runs c1, passes its ledger into c2 with IDA under receipt r2, and asserts one instance with two receipts.
- FND-002: fixed. The `before` asserts in row 6 are deleted; the final `ok` call (two instances) is the check.
- FND-003: fixed. The contradiction request runs once with a local meter (`refusal_fields`); `TriggerDelivery` is imported.
