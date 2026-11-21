---
id: SR-202611210906541
title: "base spec review of FR-1044 composition item through analyze (QSL-2739 PR1)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md; PR3287 ae565a5dd5ece5f080d7e6625b7a32b2242a8d82 over f87b2a5c43"
review_set: subset
---

## Summary

Ticket: QSL-2739. Base review of FR-1044 against FSL FR-620, FR-621, FR-630, FR-507, FR-285, FR-281 and FSpec FR-290, FR-331 (FSpec origin/main f73bd9f3). The selection shape (`claim` by declared name in the existing `items`, no new verb, request family or member, no caller-supplied proof) matches the architecture ruling and the FR-630 precedent. Unknown-name and non-compose `inapplicable-kind` refusals reuse FR-620/FR-507 exactly. Four contract defects remain: the item has no declaration family under the current node-to-family table, the `premises` refusal invents a code instead of FR-620's computed-member rule, the absent-`kind` carve-out contradicts FR-620's per-family rule, and the default-items exclusion silently overrides FR-620 with a baseline-style AC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The composition item's claim node (`correspondence`/`component_composition`) has no row in FSpec FR-290's node-to-family table nor FR-507's FSL claim-form table, so under FR-507 "Where the family is read" it has no family and every kind settles `inapplicable-kind`, not the `unsupported` at S6b that FR-1044 requires; FR-1044 must state the family (`protocol`, per FSpec FR-290's claim-form row for non-assume-guarantee composition) and amend FR-507, with the upstream node-row gap routed to arch. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:109 |
| FND-002 | high | A caller-written `premises` member is refused per item with an invented `invalid_runtime_input`/`invalid-value` code; FR-620's existing rule for FSL-computed members (`request_index`, `extent`, `candidates`, `temporal.fairness`) refuses the whole request at the request stage with `invalid-request`, exit 20, naming the member, and runs no item. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:124 |
| FND-003 | medium | "`kind` MAY be absent: a composition declaration admits one kind" contradicts FR-620's absent-kind rule, which is decided by the declaration family; the `protocol` family admits seven kinds, and the FR-630 precedent requires `kind` `refinement`. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:82 |
| FND-004 | medium | The assume-guarantee refusal's "detail names the rule `assume-guarantee`" invents a payload member; FR-507 and FSpec FR-290 close the `inapplicable-kind` payload to `request_index`, `capability` and `family`, and an assume-guarantee composition has no family (FR-290 claim-form row), so `family` is absent. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:121 |
| FND-005 | medium | Excluding compose declarations from default items overrides FR-620's "one item per checked claim of the program ... across all declaration families" without amending FR-620, and AC-8's "the exit code it gave before this requirement" is a baseline, not a checkable value. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:130 |
| FND-006 | low | The negotiation diagnostic is named only by its cause `unsupported-requested-capability`; FR-281-AC-3 and FR-331 spell the full code `unsupported_projection`/`unsupported-requested-capability`. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:93 |
| FND-007 | low | AC-3 and AC-6 say only "FR-285's code"; FR-285's order 30, 20, 21, 22, 10, 11, 0 gives exit 20 for AC-3 (refusal + unsupported + success) and 21 for AC-6 (unsupported + violation + success), and the ACs should state those values. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:143 |

## Verdict

Changes required before code (arch: "spec contract must be clear before code"). Clean and examined: the no-new-verb/member selection by declared name (Description, Decisions bullet 1), no caller-supplied proof (Decisions bullet 2), unknown-name `invalid_capability`/`unknown-claim` with siblings still running (FR-620 "Resolving items"), non-compose name with `kind` `composition` settling FR-507 `inapplicable-kind`, FR-348-AC-4's warning being the unsupported disposition's diagnostic (FSpec FR-331 "an `unsupported` ... disposition carries its FR-272 diagnostic; the warning an unsupported disposition shows is that diagnostic"), AC-1's exit 21, AC-2's empty candidates, AC-7's `package` route equality (FR-850), and FR-348-AC-1/AC-2 not claimed.

FND-001 needs an arch decision: FSpec FR-290 line 155 (claim-form table) gives "Composition of component guarantees under `protocol-refinement`, `refinement-transfer` or `interface` (FR-491) ... | `composition` | `protocol`", but its node-to-family table (lines 241-254) has no `correspondence` row and line 256 says "A claim that names a node of no row has no declaration family". Recommended bounded option: FR-1044 states the `protocol` family for a non-assume-guarantee `correspondence`/`component_composition` claim node, adds the row to FR-507's FSL tables, and arch files the missing FR-290 node row upstream (no upstream edit here).

## Scope and evidence

Spec-only PR; no code is written, so absent tests are not findings and no test baseline is claimed. `quire validate --scope=. --summary --strict` on FR-1044: 1/1 grammar-clean, 0 findings.

- FR-1044 (spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md): examined, Description and Decisions.
- FR-1044 Inputs and Behavior bullets 1-9: examined.
- FR-1044-AC-1 to FR-1044-AC-8: examined.
- FR-620 (spec/functional/FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md): context_only, lines 55-63, 92-99, 125-153.
- FR-507 (spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md): context_only, lines 66-146.
- FR-630, FR-621, FR-285, FR-281: context_only.
- FSpec FR-290 lines 131-157, 219-270; FSpec FR-331 lines 488-595, 980-990: context_only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-507's new node bullet gives a compose node the family protocol only when its rule is not assume-guarantee, saying 'An assume-guarantee node derives no request item'. But FSpec FR-491 derives an item-backed fixed-point slot for an admitted cycle under any rule, 'one FR-331 composition item anchored at S's node', and FR-507's own new claim-form row gives that fixed point protocol. So an assume-guarantee declaration's fixed-point item reads no family and settles inapplicable-kind, not unsupported as FR-1044 line 173 requires. Fix: the bullet gives protocol to every routed composition item on the node, including the fixed-point item of an assume-guarantee declaration; only the planner-local assume-guarantee item reads no family. Align with the pending FSpec FR-290 node row (separate FSpec PR, arch-owned). | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:105 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | LEAD DECISION. FR-620 now refuses a caller-written premises member at the request stage. But FR-620 lines 67-69 say FSpec FR-520 'Document members' defines the analysis-item members, and FSpec FR-520 (origin/main f9d88c77) still lists premises as a caller item member at 0..1 (line 78; Behavior lines 91-93). FR-520 names only request_index, extent and candidates as members an item 'never carries' (line 95, FR-520-AC-2). So FSL's document reader now departs from the standard wire it says the standard owns, and the code's current acceptance of premises for protocol kinds becomes a refusal. The fix text does not settle this. Options: (a) keep the refusal, have arch file an FSpec FR-520 amendment moving premises into the computed-members list (consistent with FSpec FR-331 line 586, 'carried only by a composition item a composition declaration derives'), and have FR-620 cite it as a known pending departure; (b) admit premises only when byte-equal to the derived slots, which carry keys and no result, so no caller proof, and refuse any other value invalid-request at /items/<i>/premises. Recommend (a), which matches the ruling's derived-only intent. | spec/functional/FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md:55 |
| FND-010 | medium | LEAD DECISION. For SysLat, a routed interface item over an admitted cycle, Behavior lines 158-161 make the routed item's premises 'the declaration's slots in order'. FSpec FR-491 lines 131-135 put one fixed-point slot in that order. But FSpec FR-331 lines 586-592 close a premises entry's role to component-guarantee, rely, liveness-dependency, fairness and environment, with no fixed-point, and say 'an item that breaks either rule refuses before consumption'. So AC-7's index-0 item cannot be written as a valid FR-331 request item. The fix report itself names this as arch-owned, but FR-1044 neither states nor bounds it. Options: (a) arch adds role fixed-point to FSpec FR-331 premises before PR1 code (the lattice case lands only after that); (b) FR-1044 states the routed main item's premises carry the four FR-489 lists only (FR-491 table line 129), the fixed-point slot being represented solely by the separate fixed-point item, and records the FR-331/FR-491 mismatch as remaining work. Recommend (b) for PR1 (FR-491 line 129 already says 'FR-489's four lists are its premises member'), with arch confirming. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:158 |

## Dispositions

Round 1, fix commit a0d6cd85503d2d32235981d82d262ac51bfb166e over ae565a5dd5ece5f080d7e6625b7a32b2242a8d82; verified against the committed text, not the author's report.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FSL text fixed (FR-507 row and node bullet, FR-1044 Decisions); FSpec FR-290 node row still pending in a separate arch-owned FSpec PR; see FND-008 |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | round 2, fix commit 9884f67c9d |
| FND-009 | fixed | round 3, fix commit 1598638ee1 |
| FND-010 | fixed | round 3, fix commit 1598638ee1 |
