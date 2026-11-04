---
id: SR-20261104190000
title: 'Review of PR #2317: QSL-1495/QSL-1496 TC-144 evidence-route rows and TC-149 strict checks, code-review and gap-analysis'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@939c441d9 trial-merged with origin/main 554d629af; git diff origin/main...HEAD: qsl-eval/tests/it/checked_route_owner.rs, checked_route_trigger.rs, temporal_relation.rs, main.rs; qsl-replay checked_route_owner; tests/it/compiled_protocol_v2.rs; tickets QSL-1495, QSL-1496'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1495, QSL-1496. PR: quire-spec-language#2317, head 939c441d9. Test-only. Every AC the catch-all test used to tag (FR-052-AC-9 to AC-14, FR-053-AC-7 to AC-9, TC-144, TC-149) is tagged by a value-asserting test in the computed matrix. Gates on the trial merge are green. One medium finding: the TC-144 row 4g mismatch refusal asserts only the error variant.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| TC-144 rows 2, 3, 4g, 9 | qsl-eval/tests/it/checked_route_owner.rs | examined |
| TC-149 strict checks | qsl-eval/tests/it/checked_route_trigger.rs | examined |
| TC-144 row 5 Timestamped | qsl-replay/src/spine/clause/tests/temporal/checked_route_owner.rs:223-264 | examined |
| Catch-all untagged | tests/it/compiled_protocol_v2.rs:1552 | examined |
| Causes and members | qsl-eval/src/temporal/evidence/call.rs, activation.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Row 4g second step (`activation-mapping-mismatch`) asserts only `matches!(mismatch, Err(CallFailure::Admission(_)))` plus an empty charge log. TC-144 row 4g states the cause. Mutation: renaming the cause string at call.rs:284 to `activation-mapping-missing` leaves all tests green. Fix: reuse `refused(...)` with the overridden `activation_map` (it needs a subject parameter) and assert cause and member `control`. | qsl-eval/tests/it/checked_route_owner.rs:176-186 |
| FND-002 | low | TC-144 row 5 (main) says the out-of-order Timestamped listing has the `result_identity` of row 4a. It cannot: the result identity covers observation identities, and listing 0,5,2 with levels 0,2,1 attaches different observation documents to the stamps than the in-order listing, so the identity differs (probed: sha256 00a62a.. vs c8010b..). Outcome, basis, support and premises are equal and asserted. The same observations reordered give an identical report including identity, which the test asserts and which is what FR-501-AC-18/19 and the Canonical order text require. The test is correct; the TC row needs "the same observations" wording. | spec/test-cases/TC-144-*.md row 5 |
| FND-003 | low | Row 2 compares `r1` with a snapshot of itself; the values are immutable so that assertion cannot fail. The ledger lookup after it is the real check. | checked_route_owner.rs:92-130 |
| FND-004 | low | Row 4 anchor redelivery edits the capture anchor under `if let ... CaptureFact::Value`, which is silently skipped for another variant. Use `let ... else panic`. | checked_route_trigger.rs:247-251 |

## Verification

- Matrix: all 10 FR ids plus FR-501-AC-19 show tagged binders; the catch-all carries no tags.
- Causes and members exist: activation-mapping-missing/mismatch (member control), no-ledger (ledger), predecessor-unknown/not-latest, unchanged-input (relation), delivery-contradiction (deliveries, contradicted_identity), surplus-capture (captures, surplus_capture_identity/index), trigger-required/unknown (trigger).
- Mutation 1 (rename mismatch cause): survived, FND-001. Mutation 2 (redelivery honours guard false): killed by no_other_field_substitutes_for_the_identity (guard). Both reverted.
- Gates: fmt, check --workspace --all-targets, clippy -D warnings (default and no-default-features) green; qsl-eval it checked_route_owner, checked_route_trigger, temporal_relation 20/20; qsl-replay checked_route_owner 9/9; root `cargo test --test it` exit 0.

## Verdict

Mergeable after FND-001 (small fix in this PR); FND-002 is a spec wording follow-up, FND-003 and FND-004 optional.
