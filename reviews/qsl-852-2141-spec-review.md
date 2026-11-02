---
id: SR-2141001
title: spec-review of QSL-852 composed-root reader spec (PR 2141)
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; QSL-852"
review_set: subset
---

## Summary

Ticket: QSL-852; PR 2141 at 121bf7be. Reviewed the diff only: FR-390 'Composed roots from the reader' with AC-28/29, FR-383 'Protocol term locations' with AC-32, and the scope lines of TC-1063/1070/1073. Sub-analyses applied inline: EARS, integrity, criterion-strength (Jev unavailable; manual judgement).

## Verdict

**FAIL** - not mergeable until FND-001, FND-002 and FND-003 are fixed. quire validate: no errors. No id collisions with main or open PRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | untestable-ac: AC-29: halting location per visit, in `earlier.tally < 3` / | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-002 | high | soundness: Removing the row of one visit, or making two visits resolve | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-003 | high | ambiguous: the k-th visit of that application, in a left-to-right pre-o | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-004 | medium | soundness: the rows of that application whose origin is the slot's site | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-005 | medium | other: It refuses no other slot of the package that the view could | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-006 | medium | exceeds: a kind that QSL emits and TC-1070's fixtures E and L use (`a | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-007 | medium | untestable-ac: a variant whose row is removed or duplicated is refused by t | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-008 | low | other: A reader view whose source package has had its `CheckedBody` | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-009 | low | compound: The emitter SHALL write, in the `checked_term_data.protocol_ | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | soundness: QSpec FR-322 requires an expression node's `dependencies` to be exactly the unique node-id-ascending reference targets of its body; a reordered array  | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-011 | medium | untestable-ac: The vector (TC-1070 C9) needs an operation identity that FR-322 admits and FR-400 has no NodeKind for | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-012 | medium | soundness: FR-322 keys a row by (application node_id, occurrence key, term_ordinal) with no origin in the key, and FR-383/FR-390 separate slots that share an app | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-013 | medium | coverage: FR-383-AC-32 to AC-37 and TC-1063 L1 to L6 cover only W, E and L | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-014 | low | compound: AC-38 carries four independent properties; AC-32, AC-34, AC-35, AC-37 (eight slot shapes) and FR-383-AC-32 each carry several vectors, so a failure do | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-015 | low | trace: TC-1073 adds AC-28 to its scope but gains no procedure row; TC-1070 scope lists FR-390-AC-39 and TC-1063 scope lists FR-383-AC-36 (both Inspection) wi | spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md |
| FND-016 | low | soundness: FR-322-AC-291 refuses a duplicate key at the first mismatching row, not at the array-level missing-predecessor locus; TC-1063 L6 says so but AC-37 sta | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | round 1: FR-390-AC-32 (replaces old AC-29): Evaluating the retry relation of `Refund` with `earlier` unavailable halts at the first `earlier.tally` visit, and the reader result carries the `Location` of the te |
| FND-002 | fixed | round 1: FR-390-AC-33: With the highest-index `earlier.tally` row of the retry origin removed ... view creation returns `ViewFailure::Unsupported` with `MissingLocationProjection` ... FR-390-AC-35: With the or |
| FND-003 | fixed | round 1: FR-383: A slot walk is the sequence of positions a pre-order walk visits in the checked expression tree of one protocol slot expression ... Children are taken in the operand order of the node's FR-400 |
| FND-004 | fixed | round 1: FR-390: The slot descriptor the selector returns SHALL carry the slot's root node key and the slot's origin (the `protocol_site` origin of QSpec FR-322), and the reader SHALL take both from it. |
| FND-005 | fixed | round 1: FR-390: Because view creation is package-wide, this refusal leaves no view of the package, as every other unsupported projection does. (reason list now includes `MissingLocationProjection`) |
| FND-006 | fixed | round 1: FR-390: Every operation identity that QSpec FR-322 admits for an expression node and that FR-400 gives a `NodeKind` SHALL project to that kind. No `NodeKind` variant is outside the reader's surface |
| FND-007 | fixed | round 1: FR-383-AC-37: The mutated emitted package of fixture E with the highest-ordinal `earlier.tally` row of the retry origin removed is admitted by QSL's I2 reader; with the ordinal-zero row removed, with  |
| FND-008 | fixed | round 1: FR-390-AC-30: A reader view whose source package holds, for a slot, the real `CheckedBody` of a different slot of the same package (taken through the checked package's existing API, with no new `const |
| FND-009 | fixed | round 1: FR-383: The emitter SHALL write, in the `checked_term_data.protocol_locations` array of QSpec FR-322, one row for every term of every protocol slot expression it emits. (separate SHALLs follow for row |
| FND-012 | fixed | round 2: Anchor withdrawn gives Unavailable (AC-2); decision closure/completeness/support withdrawn keeps derived activation with a Missing non-value (AC-3). |
| FND-013 | fixed | round 2: 14m/15m are selection cases (definition.identity, definition.authority); 28m states invalid_package/source-integrity at the CheckedSourceRef pointer. |
| FND-014 | fixed | round 2: Ordering paragraph gates implementation on the Contract IR requirement (ticket to be filed); AC-1 and AC-4(b) conditioned on it; counts 19 reader, 14 selection. |
| FND-015 | fixed | round 2: FR-519-AC-5 is behavior; FR-515-AC-8 is compile-fail only; TC-1310 row 8 and TC-1314 row 5 match. |
| FND-016 | fixed | round 2: FR-499-AC-7, FR-500 AC-7 and writer.rs staging row cite FR-521 rows 2 and 7. |
| FND-017 | fixed | round 2: ActivationGuardAbsent removed from FR-515 and TC-1310 row 5. |
| FND-018 | fixed | round 2: Row 14 names the legacy manifest format identity; FR-517-AC-10 is an Inspection with a merge condition; 66-reference counts removed. |
| FND-017 | fixed | round 3: FR-383-AC-48: In fixture E-C the application `m.count >= 1` is one graph node held by the two slots `Composed.shareA` and `Composed.shareB`; its group holds rows of both slots' origins with `term_ordi |
| FND-018 | fixed | round 3: FR-383-AC-44: ... every slot's per-application row count equals its visits in the slot walk ... FR-383-AC-45: For fixture E-C, each row's `Location` equals the checked clause's ... FR-383-AC-46: ... t |
| FND-019 | accepted-no-change | round 3: FR-390 now states no test vector exercises MissingOperationProjection because no package can reach it (the identity-to-NodeKind map is required to be total) and AC-41 is its only check. |
| FND-020 | fixed | round 3: FR-383-AC-43: Duplicating a retry-origin `earlier.tally` row is refused by QSL's I2 reader with `invalid_package`/`conflicting-definition` at `/checked_term_data/protocol_locations/<index of the dupli |
| FND-021 | fixed | round 4: FR-383 orders rows across origins by ascending origin string, and FR-390-AC-53 and C14a remove the highest-term_ordinal row (the lone remaining row is ordinal zero, so I2 admits it). Residual: FND-024. |
| FND-022 | fixed | round 4: the merge-gate sentences are gone from FR-383 A1 and FR-390. Residual: FND-026 (the cited QSpec ids are not yet on QSpec main). |
| FND-023 | fixed | round 4: FR-383-AC-48 is split into AC-49, 50, 51 and FR-390-AC-53 into AC-53 and AC-54. Residual: FND-027. |
| FND-024 | fixed | round 5: the origin string is replaced by FR-322's structured origin with a defined origin order (type_declaration before protocol_attempt; declarations by name; attempts by protocol then attempt, numerically). |
| FND-025 | fixed | round 5: FR-390-AC-38 now adds a Refund-origin row at term_ordinal 1, the next consecutive ordinal after the group's one ordinal-zero row. |
| FND-026 | fixed | round 5: PR 2141's body states "MERGE ORDER: ... Merge #603 first, then this PR"; QSpec PR 603 is still open. |
| FND-027 | fixed | round 5: FR-383-AC-51 now states only the ordinal values 0, 1, 2 with no gap and no restart; the order is in AC-52. |
| FND-028 | fixed | round 5: TC-1070 declares Composed (protocol position 0), the capture order, run Main and attempts Tried 0 and Picked 1. New conflicts: FND-029, FND-030. |
| FND-029 | fixed | round 6: the shared-origin vectors move to fixture E (FR-383-AC-49/50, FR-390-AC-52/53, C14/C14a: `target >= 0` is held by Refund's `retry.relation` and `trigger.guard`, both in fixture E's source) and E-C uses one `constraint` slot per attempt control, as FR-390's table gives. Slot order uses member names `registration_captures`, `trigger.guard`, `activation_captures`, `retry.relation`, `recovery.predicate`, `constraint`, all in FR-390's slot table. |
| FND-030 | fixed | round 6: E-C is its own package with Composed at protocol position 0 and shares nothing with Refunds. |
| FND-031 | fixed | round 6: the occurrence rule is normative (non-decreasing occurrence ordinal, consecutive ordinals within an occurrence) and FR-383-AC-54 and L9 test strictly increasing pairs. |
| FND-032 | fixed | round 6: FR-390-AC-53 is split into AC-52, 53 and 54. Residual: FND-036. |
| FND-033 | fixed | round 6: FR-383-AC-32 now names W, fixture E and protocol L. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-021 | medium | untestable-ac: The group of m.count >= 1 holds one row per origin with term_ordinal consecutive from zero across both | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-022 | low | other: A process gate and a 'not yet merged' status sit in normative spec text, in FR-383 and again in FR-390 | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-023 | low | compound: AC-48 joins the shared-node premise, the cross-origin ordinal rule and the one-row-per-origin count; FR-390-AC-53 joins the shareB refusal and the not | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

## Verdict (disposition pass 3, superseded by pass 4)

Round 1 and round 2 left FND-001 to FND-016 fixed. Round 3: FND-017, FND-018 and FND-020 fixed, FND-019 accepted-no-change. FND-021 (medium) is open, FND-022 and FND-023 (low) are open. Head 91175115c on main c18814e85; quire validate on the changed spec files has no errors; no id collisions. Not mergeable until FND-021 is fixed, and not before QSpec PR 603 (open at review time). Left open by design for the code lane: shared-application hash-consing in the emitter and E-C surface-syntax expressibility.

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-024 | medium | ambiguous: "origin string in UTF-8 byte order" has no definition; FR-322's closed origin is structured, and AC-51, AC-53 and C14a assert shareB sorts after shareA without a stated string | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-025 | medium | untestable-ac: FR-390-AC-38 adds a row "at ordinal zero after any rows of its group", which is incoherent and which I2 refuses as a duplicate key | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-026 | low | other: A1 and FR-390 cite QSpec FR-322-AC-310 to AC-312, absent from QSpec main (PR 603 open); the gate was removed and PR 2141's body does not name the merge order | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-027 | low | compound: FR-383-AC-51 joins ordinal values and byte order | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-028 | low | ambiguous: "Composed's run Main" and the attempts Tried and Picked of E-C are not declared; Tried is fixture E's attempt | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md |

## Verdict (disposition pass 4, superseded by pass 5)

FND-021, FND-022 and FND-023 are fixed. Head 8de494964 on main c18814e85 (unchanged): quire validate has no errors; no id collisions with main or open PRs; every AC has a TC row and the scope lines match. No high finding. FND-024 and FND-025 are medium vector or term defects, so the PR is not mergeable yet. QSpec PR 603 is still open and FR-322-AC-310 to 312 are not on QSpec main, so the PR must also wait for it (FND-026).

## New findings (disposition pass 5)

Head e2aaf6de4 on main c18814e85 (unchanged). quire validate: no errors. No id collision with main or open PRs. Every FR-390-AC-28 to 55 and FR-383-AC-32 to 52 has a TC row; scope lines match.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-029 | high | untestable-ac: E-C slots `Tried.constraints[0]`, `Tried.constraints[1]`, `Picked.constraints[0]` do not exist in FR-390's slot table (an attempt has one `constraint`, index none), so the shared-origin vectors cannot be built | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md |
| FND-030 | medium | ambiguous: E-C is fixture E plus Composed at protocol position 0 with attempt Tried at attempt 0, but fixture E's Refunds is protocol 0 and its run Main already has a Tried | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-031 | low | soundness: the rule that (occurrence ordinal, term_ordinal) order equals visit order is unsatisfiable when visits interleave occurrences, which A2's second sentence admits | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-032 | low | compound: FR-390-AC-53 and FR-383-AC-52 each join two claims | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-033 | low | other: FR-383-AC-32 covers W and L only, while L1 and the text apply it to fixture E | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

## Verdict (disposition pass 5, superseded by pass 6)

FND-024 to FND-028 are fixed. FND-029 is a high defect: the fixture slots the shared-origin vectors need are not slots in FR-390's table. The PR is not mergeable until FND-029 is fixed and FND-030 resolved. It must also merge after QSpec PR 603 (stated in the PR body). Open questions for the planner, not defects: the origin-walk reading, assumption A2, and whether a compensation such as Refund counts as a protocol attempt in FR-322's origin.

## New findings (disposition pass 6)

Head c0eb6c213 on main 38f3020e8 (includes PR 2145). quire validate: no errors. No id collision with main or open PRs. Every FR-390-AC-28 to 56 and FR-383-AC-32 to 54 has a TC row; scope lines match. PR body states merge order (603 first), the attempt-position QSpec question and the coverage gap.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-034 | high | soundness: FR-383 gives every slot a protocol_attempt origin, but only attempt controls and compensate templates have an attempt position; other slots (clause, role, channel, effect, commit, finish, choice, check) and all of protocol L have none | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-035 | low | ambiguous: E-C Tried and Picked are attempt controls with no result binder declared | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md |
| FND-036 | low | compound: FR-383-AC-52 joins row counts and ordinals; AC-53 repeats AC-52's ordinals | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

## Verdict (disposition pass 6)

FND-029 to FND-033 are fixed. FND-034 is a high defect: fixture L (in FR-383-AC-32) has no attempt-bearing node, and fixture E's effect, commit and finish constraints have none, so the origin the spec requires does not exist for those slots. Not mergeable until FND-034 is fixed. It must also merge after QSpec PR 603. Open questions: the origin-walk reading, whether a compensation is an attempt position in FR-322 (in the PR body), and whether the producer-defect classification `invalid_package`/`invalid-value` with a pointer fits as evidence on `ViewFailure::Unsupported`.
