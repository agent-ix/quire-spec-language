---
id: SR-20261104212208
title: "Gap analysis of PR #2297: FR-201 bounded adapter and FR-498 claim binding traces"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@bb72749c3; git diff origin/main...bb72749c3 (PR #2297): qsl-replay/src/probability_certificate.rs, qsl-replay/tests/it/probability_certificate.rs; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md, spec/test-cases/TC-1288-a-probabilistic-claim-carries-its-checked-package-identity.md, spec/test-cases/TC-636-probability-certificate-checker.md; ticket QSL-1498"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1288
    type: reviews
date: '2026-11-04'
---

## Summary

Ticket: QSL-1498. This gap analysis is limited to the PR diff. Quoin's matrix run was not used. Instead, each trace tag the diff adds was checked by hand against the text of the AC it names. The production code in the diff has an owning requirement: FR-201 for the checker and FR-498's consumer rule ("No consumer selects or joins a claim's identity by its name, span, display text or declaration order") for the binder. The trace tags overclaim. FR-201-AC-1 is tagged twice and FR-498-AC-2 once, but neither test checks what those criteria state.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-201-AC-1 | spec/functional/FR-201-check-a-probability-certificate.md:180 | examined |
| FR-201-AC-3 | spec/functional/FR-201-check-a-probability-certificate.md:182 | context_only |
| FR-498-AC-2 | spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md | examined |
| FR-498 Behavior (consumer join rule) | spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md | examined |
| TC-1288 | spec/test-cases/TC-1288-a-probabilistic-claim-carries-its-checked-package-identity.md | examined |
| TC-636 | spec/test-cases/TC-636-probability-certificate-checker.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Both new integration tests are tagged `FR-201-AC-1`, but neither checks that criterion. It requires the §15.4 model at threshold 0.95 with the `Exact` certificate 24/25, 4/5, 1, 0 accepted `AtLeast`, the 9/10 variant rejected at the first differing state, and the item settling `inconclusive`/`CertificateRejected`. The tests use a one-state vault fixture, an empty certificate and a trivially reached target. This inflates coverage: the matrix will show FR-201-AC-1 as covered. Fix: remove `FR-201-AC-1` from both tags and keep `TC-636`. | qsl-replay/tests/it/probability_certificate.rs:23; qsl-replay/tests/it/probability_certificate.rs:134 |
| FND-002 | medium | `claim_binding_uses_the_checked_node_not_name_or_source_order` is tagged `FR-498-AC-2` and `TC-1288`. FR-498-AC-2 requires the emitted node, the row and the checked claim to share one identity, rename, move and token-change behaviour, and distinct identities for identical twins. The test checks none of these. Its reorder-stability check repeats FR-498-AC-1's permutation clause, which `qsl-package/src/emit/tests/probabilistic.rs:158` already covers. TC-1288 is the producer test case. The behaviour this test really verifies is FR-498's consumer join rule, which has no AC. Fix: retag to `TC-636`, or add an FR-498 AC for consumer binding (a spec change) and tag that. | qsl-replay/tests/it/probability_certificate.rs:134 |

## Verdict

Conditional. Both findings are tag edits on test attributes and change no code. The test logic is sound: mutation M1 (first-match or source-order binding) fails it, as recorded in the code-review SR for this ticket. No production code is left without an owning requirement, and the diff has no stubs.

## Dispositions

- FND-001: fixed. `FR-201-AC-1` is removed from both tests; they keep `TC-636`.
- FND-002: fixed. The binder tests are tagged `TC-636` only; no FR-498 AC was added.
