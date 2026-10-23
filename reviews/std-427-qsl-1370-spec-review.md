---
id: SR-16268
title: "Spec-review of authenticated temporal clock premises and the TL bridge input (quire-spec-language PR 1370)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5bf6d5f16289e3c43309ba488b581a881117b4f7; spec/functional/FR-043-evaluate-bounded-native-temporal.md; spec/functional/FR-045-classify-temporal-mapping-support.md; spec/test-cases/TC-122-evaluate-bounded-native-temporal.md; docs/native-temporal-evaluation.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-122
    type: references
---

# Spec-review of authenticated temporal clock premises and the TL bridge input (quire-spec-language PR 1370)

## Summary

Ticket: STD-427. Head 5bf6d5f1. origin/main is 6 commits ahead and is not an ancestor. The
merge is clean. Gate `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation`: exit 0, no warnings on the touched files.

The FR-043 `/2` authentication was checked against `src/temporal.rs` `authenticate_v2` and
`parameter_dimension`. The order is profile, then clock name, then exact key set, then the first differing
value (`epoch`, `period`, `unit` for fixed-sample). The dimension names and the retained
`AuthenticatedBinding` fields match the code. The FR-043-AC-14 vector has one mutation per dimension, and
each mutation reaches its stated dimension. FR-043-AC-14 is unique across open PR heads.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [other] scope-deferral (standing rule). STD-427 asks FR-045 to state who emits the TL formula, the valuation request and the correspondence record, with no deferral left. The PR says the Contract-IR bridge emits them, and its Downstream bullet cites quire-contract-ir#63, #64 and #71. All three issues are closed. The PR body and STD-436 say the bridge's spec (IR FR-025 to FR-027) was deleted in IR #210, so the emission is specified nowhere, and its owner is an open owner question (STD-436). The Blocked marker has been renamed, not resolved, and the spec now names an implementation that does not exist. The bullet also cites GitHub issue numbers as spec content. Fix: specify the emission (STD-436's draft FR, in QSL beside FR-045 or in IR) in this change set. Otherwise the ticket is not done. | spec/functional/FR-045-classify-temporal-mapping-support.md:28-32; spec/functional/FR-045-classify-temporal-mapping-support.md:156-160 | missing-requirement |
| FND-002 | medium | [ambiguous] AC-14 ends: 'The same mutated traces through the /1 evaluate are not refused on those parameters'. TC-122 step 14 says: 'Evaluate the same mutated traces through the /1 evaluate ... require no refusal on those parameters'. The mutated set includes another profile and another clock name. FR-043's first Profile-and-clock rule (line 140) makes /1 refuse both, and so does `src/temporal.rs`. Read literally, the step fails, or it is open to two readings. Fix: restrict the /1 contrast to the parameter mutations (key set, epoch, period, unit, sequence_authority, timestamp_unit), and state that the profile and clock-name mutations refuse under /1 as well. | spec/functional/FR-043-evaluate-bounded-native-temporal.md:303; spec/test-cases/TC-122-evaluate-bounded-native-temporal.md:132-134; spec/functional/FR-043-evaluate-bounded-native-temporal.md:140-142 | wrong-requirement |
| FND-003 | medium | [other] STD-427 requires that no deferral wording remain in FR-043. The section 'Verification limit recorded on compiler #38' still records FR-043-AC-10's substitution clause as having 'no expressible verification', and ties that to compiler #38. Fix: either state AC-10's substitution clause as a behaviour the evaluator has by construction (it accepts no basis from the caller) and verify it by Inspection, or remove the unverifiable clause from AC-10. In both cases, drop the #38 heading. | spec/functional/FR-043-evaluate-bounded-native-temporal.md:46-55 | missing-requirement |

## Verdict

Not mergeable. The FR-043 `/2` authentication is correct and complete against the code. FR-045, however, replaces the remaining-work marker with a citation of a bridge that is specified nowhere, and its owner is an open owner question (STD-436). Under the standing rule that is a scope-deferral. The AC-14 `/1` contrast and the remaining #38 section need fixing as well.
