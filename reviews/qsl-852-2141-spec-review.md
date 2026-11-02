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

Head cd90368ef reviewed against `git diff origin/main...HEAD`. quire validate on the five changed spec files: exit 0, no errors (warnings are the environment's duplicate-archetype notes). No id collision: FR-383-AC-32 to AC-37 and FR-390-AC-28 to AC-39 are absent from main and from every open PR's diff.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | soundness: AC-38 and C11 reorder the `dependencies` arrays, which QSpec FR-322 fixes as node-id-ascending, so I2 refuses that package | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-011 | medium | untestable-ac: AC-36 and C9 need an operation identity FR-322 admits with no NodeKind mapping, and none is named | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-012 | medium | soundness: term_ordinal grouping and the occurrence order are not reconciled with FR-322's row key, so the ordinal-zero vectors depend on an unstated premise | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-013 | medium | coverage: no emitter-side row covers the E-C composed shapes, though AC-37 needs an emitted E-C package | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-014 | low | compound: AC-38 bundles four properties; AC-32, AC-34, AC-35 and AC-37 bundle several vectors | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-015 | low | trace: TC scope lines claim AC-28 (TC-1073) and the Inspection ACs 36/39 with no procedure row | spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md |
| FND-016 | low | soundness: FR-383-AC-37 states one code and locus for all mutations; FR-322-AC-291 and TC-1063 L6 give the duplicate a different locus | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Old AC-29 replaced. FR-390-AC-31 and AC-32 with TC-1070 C2 to C4 give a halt vector (earlier unavailable halts at the first `earlier.tally`) and a row swap that changes the reported Location, so each can fail. |
| FND-002 | fixed | Removal vectors re-layered per FR-322-AC-291: FR-390-AC-33 and C5 remove the highest-index row (view refusal), AC-35 and C7 remove or duplicate the ordinal-zero row (I2 refusal), FR-383-AC-37 and L5/L6 repeat this at the emitter. The unbuildable "two visits resolve to one row" vector is gone. Residual premise: FND-012. |
| FND-003 | fixed | FR-383 "Protocol term locations" defines the slot walk: NodeKind operand order, not `dependencies` order; shared node walked once per parent; all branches and arms walked; `Local` and literal are leaves; handles identify positions. FR-390 adopts it. |
| FND-004 | fixed | FR-390 requires the slot descriptor to carry root node key and origin, and FR-383 requires every slot's origin to be unique (AC-35). Residual ordinal question: FND-012. |
| FND-005 | fixed | The "refuses no other slot" sentence is gone; whole-view refusal is stated ("leaves no view of the package"); `MissingLocationProjection` is a new reason with Node locus and evidence, added to the ordering list. |
| FND-006 | fixed | The list of required kinds is replaced by "every operation identity FR-322 admits and FR-400 gives a NodeKind projects", with fixture E-C and AC-37/C10 covering let, conditional, case, dereference, arithmetic, presence, collection, record. Residual: FND-011 (unmapped identity) and FND-013. |
| FND-007 | fixed | FR-383-AC-32 split into AC-32 to AC-37, each with a TC-1063 row (L1 to L6) giving code and locus. Residual: FND-014, FND-016. |
| FND-008 | fixed | FR-390-AC-30 and C8 say the swapped body is another slot's real `CheckedBody` taken through the existing API. TC-1073 now scopes AC-28 only (but see FND-015). |
| FND-009 | fixed | The SHALL sentence is split into separate SHALLs; the empty-array case has a SHALL; the protocol-attempt origin form is named. The undefined "occurrence ordinal" survives and is covered by FND-012. |

## Verdict (disposition pass 1)

All nine prior findings are fixed. The head is not yet mergeable: FND-010, FND-011 and FND-012 are medium defects of vectors that cannot be built or that rest on an unstated premise, and FND-013 is a medium coverage gap. No high finding. The three author questions are not decided here: (1) FR-322-AC-291 refuses a row for a non-application node, while FR-400 says a literal read borrows the literal's own Location; whether a `Local` carries a location of its own is not stated in either spec; (2) whether fixture E-C is expressible in QSL surface syntax (it is written in checked-term form); (3) a halt at the second `earlier.tally` visit is unreachable in fixture E (the second visit follows a successful first visit under `and`), so only C2 and C4 observe the second location.
