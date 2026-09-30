---
id: SR-828
title: "QSL-336 evidence review of PR 539 (FR-122 ACs vs TC-517)"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language@15d200d648a0fc06ec3d5a6c4270debfb40ee913; spec/functional/FR-122-replay-a-state-clause-counterexample.md; spec/test-cases/TC-517-replay-a-state-clause-counterexample.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
## Summary

Ticket: QSL-336. PR: quire-spec-language#539 at 15d200d6.

Every FR-122 AC is verified by Test (TC-517). TC-517 step n covers AC-n
for n = 1 to 6, and its Expected Results match each AC's outcome, code and
retained identities. Each AC has a failing mutant: skipping the envelope
identity check fails AC-3; admitting before it fails AC-3's
absent-invocation half; ignoring the occurrence key fails AC-3's ordinal-1
case; settling by the proved verdict alone fails AC-2; a payload that is a
product, not a sum, fails AC-6. Every named fixture exists: FR-108's
ConfigVersion unit and cases, `sameIdentity`, and TC-466 step 3's `probe`
unit (qsl-replay/src/spine/clause/tests.rs:2802-2861). The type-shape half
of AC-6 is verified by compiling, as FR-116-AC-5 is.

## Verdict

Clean. Evidence is complete and each AC is falsifiable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
