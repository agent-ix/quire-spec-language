---
id: SR-202611210906542
title: "integrity review of FR-1044 composition item through analyze (QSL-2739 PR1)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md; PR3287 ae565a5dd5ece5f080d7e6625b7a32b2242a8d82 over f87b2a5c43"
review_set: subset
---

## Summary

Ticket: QSL-2739. Integrity review (completeness, consistency, atomicity, traceability) of FR-1044. Statement, Outputs, Behavior and ACs agree on the PR1 outcome (`unsupported`, exit 21, never `proved`) and on the derived-after-caller placement, which matches FSpec FR-331 "Items" ("The caller's items come first and the derived items after them"). Three defects: the AC fixture is never defined, the premise items' position among other derived items is unstated, and normative dependencies are missing from `relationships`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ACs use a fixture that is never defined: the two imported component packages, their subjects, guarantee names, the violated guarantee of AC-6, and `Bounded`, which AC-3 calls "a state invariant of the unit" although FR-620's Counter fixture defines `Bounded` as a temporal clause; FR-620 defines its fixture before its AC table and FR-1044 should too. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:143 |
| FND-002 | medium | Outputs place premise items "at the index FR-620 gives a derived item, after the caller's items", but FR-620 and FSpec FR-331 order only FR-368's derived items (structural order); the order of composition premise items relative to FR-368 derived items (for example a subject's deadlock-freedom item) and across several selected compositions is unstated. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:89 |
| FND-003 | low | `relationships` omit requirements the body depends on normatively: FR-507 (the `inapplicable-kind` rule), FR-285 (exit), FR-281 (negotiation diagnostic), FSpec FR-290 (declaration family) and FSpec FR-489 (the premise rule the Decisions cite). | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:5 |

## Verdict

Changes required. Recommended fix for FND-002: state that the premise items follow the caller's items and FR-368's derived items (or precede them; either is fine if stated), grouped per selected composition in caller-item order, each group in FSpec FR-491 slot order. Clean and examined: Description/Outputs/AC-1 agreement on `request_index` 0 and derived indices 1 and 2; the PR1 "never `proved`" invariant stated in Description, Behavior bullet 2 and AC-1; AC-4's statement that the other two routed rules settle as `interface` does; the `Remaining work: QSL-2739, second PR` marker for FR-348-AC-1/AC-2.

## Scope and evidence

Spec-only PR; no code written, so missing tests are not findings. `quire validate` clean on FR-1044.

- FR-1044 (spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md): examined, all sections.
- FR-1044-AC-1 to FR-1044-AC-8: examined.
- FR-620 Acceptance Criteria preamble (fixture) lines 215-220: context_only.
- FSpec FR-331 "Items" lines 489-495: context_only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The fixture doesn't match FSpec FR-491's declaration form. FR-491 makes `component C from I using M` name the subject M of an imported checked package I, and `guarantee C::G` a temporal clause 'of C's package'. The fixture calls test/producer and test/consumer domain packages 'selected as PM and CM', and lists PSafe and CSafe beside the unit, so it is unclear which package declares them. It also provisions no initialization snapshot, so 'proved by EN-1 alone' has no initial states, and it uses FR-620's Bounded without selecting test/counter. Fix: say which checked package declares each guarantee, and provide the snapshots and the Counter selection. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:188 |
| FND-005 | medium | AC-10's 'unit declaring only Sys' contradicts the fixture unit, which declares SysRely, SysAG, SysPR, SysRT and SysCycle and uses Bounded. Its indices also skip FR-620's default rule, under which the unit's own claims come in source order, then the FR-368 derived items per model subject (FR-620-AC-5), with premise items after them (Outputs item 2). Fix: name the unit AC-10 uses and give its full default item list with indices. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:214 |
| FND-006 | low | No AC covers the stated lattice case: a cycle under a profile declaring FR-176's lattice writes the fixed-point slot's item as one more premise item, settled unsupported. Fix: add a lattice-profile variant to AC-7. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:171 |
| FND-007 | low | AC-6 asserts no value for the assumption-bound CSafe premise item's record ('the one negotiation gives it'), so that part can never fail. Fix: state the record Engines::fsl() gives it, e.g. unsupported if no registered engine takes an item carrying that assumption set, as the Decisions say. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:210 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | test/system-default 'declares inc with no pre or post', but inc is the operation of the domain package test/counter. An FSL unit attaches clauses to it, for example 'pre CanInc using v on Counter::Counter::inc { ... }' (tests/it/plan_export_surface.rs:396), and does not declare it. Fix: say the unit declares no pre or post clause on M::Counter::inc. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:228 |
| FND-009 | low | The fixed-point item omits the options naming the declaring profile. FSpec FR-491 lines 150-154 make that options member part of the item ('whose options name the declaring profile; its slot key binds those'). Fix: add 'its options naming the declaring profile' to Behavior lines 189-193 and to AC-7. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:189 |

## Dispositions

Round 1, fix commit a0d6cd85503d2d32235981d82d262ac51bfb166e over ae565a5dd5ece5f080d7e6625b7a32b2242a8d82; verified against the committed text, not the author's report.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | round 2, fix commit 9884f67c9d |
| FND-005 | fixed | round 2, fix commit 9884f67c9d |
| FND-006 | fixed | round 2, fix commit 9884f67c9d |
| FND-007 | fixed | round 2, fix commit 9884f67c9d |
| FND-008 | fixed | round 3, fix commit 1598638ee1 |
| FND-009 | fixed | round 3, fix commit 1598638ee1 |
