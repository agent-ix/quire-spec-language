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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | high | [soundness] FR-045 now requires that 'Each generated artifact SHALL be admitted by its owner's public reader before the projection is returned', and TC-1240 runs the real tl-syntax, tl-mltl and quire-mltl readers. No QSL crate depends on those crates today. ADR-011's dependency graph (lines 1050-1100) lists every allowed edge, and it has no QSL to tl-syntax, tl-mltl or quire-mltl edge. STD-436 gave this as the reason the emission sat in quire-contract-ir, which may depend on QSL and the TL crates together. Fix: amend ADR-011 with the new edges and the crate that holds them, checking that the TL crates do not depend back on QSL. Otherwise, specify the admission so it does not need them. | spec/functional/FR-045-classify-temporal-mapping-support.md:222-226; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1050-1100 | wrong-requirement |
| FND-005 | high | [coverage] `read_tl_projection` and `read_tl_join` reject 'unknown, duplicate, missing or out-of-order members' and 'noncanonical' bytes. No spec defines those bytes. Neither FR-045 nor QSpec FR-095 (#421) defines the bridge profile `quire.native-temporal-correspondence/v1`, the projection and join record members and their order, the canonical encoding, the preimage of the 'derived reference' over the preserved key, or the types `TlSelection`, `ExpectedTlProjection`, `ExpectedTlJoin`, `TlMappedResult` and `Limits`. Two implementations could not exchange a projection or a join. AC-9's 'changes the correspondence reference' and AC-12's 'noncanonical projection or join' have no oracle. Fix: specify the wire and the reference digest (QSpec owns the correspondence meaning, so this may need a QSpec object FR beside FR-095) and name the types. | spec/functional/FR-045-classify-temporal-mapping-support.md:158-170; spec/functional/FR-045-classify-temporal-mapping-support.md:220-226; spec/functional/FR-045-classify-temporal-mapping-support.md:262-270 | missing-requirement |
| FND-006 | medium | [ambiguous] 'The decision SHALL be one row of this table', but the rows overlap. One result unsupported and the other incomplete matches two rows. A refused result with unequal premises matches `refused` and `conflict`. An unavailable producer beside a failed result matches two more. No evaluation order is given, so two implementations can return different decisions. Fix: say that the rows are evaluated in order, first match wins, and put them in the intended order. | spec/functional/FR-045-classify-temporal-mapping-support.md:240-252 | wrong-requirement |
| FND-007 | medium | [soundness] AC-8 and TC-1240 step 2 project 'an infinite-trace declaration ... classified Unsupported'. FR-045's support table, which says every admitted declaration matches exactly one row, has rows only for the timestamped-event, event-position and fixed-sample profiles. QSpec FR-095 has a `quire.temporal.infinite-trace/v1` row (unsupported, no TL target). The classifier therefore has no stated disposition or dimension for an infinite-trace declaration. Fix: add FR-095's infinite-trace row with its dimension, and restate the totality claim. | spec/functional/FR-045-classify-temporal-mapping-support.md:283; spec/functional/FR-045-classify-temporal-mapping-support.md:120-142 | wrong-requirement |
| FND-008 | medium | [trace] FR-045 depends on `tl-mltl/FR-007` as the 'trace and history wire'. tl-mltl FR-007 is 'Bind typed signals and caller context into temporal results'. The `trace/v1`, `position-history/v1` and `history-requirement/v1` wire is tl-mltl FR-018 ('publish temporal owner wire'). FR-045 also cites tl-syntax FR-011 and FR-012 for 'the formula editions'. FR-012 is history anchor and progress, and `tl-syntax.formula/v2` is published by FR-013 and FR-014. The quire-mltl request and result mapping is cited by no id. Fix: correct the edges and the Dependencies bullet, and cite quire-mltl's FR. | spec/functional/FR-045-classify-temporal-mapping-support.md:12-14; spec/functional/FR-045-classify-temporal-mapping-support.md:302-305 | wrong-requirement |
| FND-009 | medium | [coverage] The limits are 'caller-configured', and FR-045 names eight dimensions. None has a limit name, a request setting or a published default, which QSpec FR-461 requires of every limit, and none says which limit-stop cause it reports. AC-12 cannot be written against exact values. Fix: name each limit and its setting, give its default, and give its non-admitted cause. | spec/functional/FR-045-classify-temporal-mapping-support.md:100; spec/functional/FR-045-classify-temporal-mapping-support.md:266-270 | missing-requirement |
| FND-010 | medium | [soundness] A history-cutoff boundary yields `incomplete` with cause `history-not-origin-complete`. QSpec FR-095 (#421) says only 'incomplete', and grep finds the code in no QSpec cause catalogue (FR-272, FR-331). The cause is therefore QSL-local and uncatalogued. Fix: add it to QSpec's vocabulary through #421, or reuse an existing incomplete cause. | spec/functional/FR-045-classify-temporal-mapping-support.md:193-196 | wrong-requirement |

## Dispositions

Round 1, against head ae1210a5. Gate `make check-spec-validation` at ae1210a5 exits 0, both with QSPEC_DIR at the shared clone and at QSpec #421's head.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
