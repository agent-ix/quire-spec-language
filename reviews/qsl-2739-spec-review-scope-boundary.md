---
id: SR-202611210906544
title: "scope-boundary review of FR-1044 composition item through analyze (QSL-2739 PR1)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/filament-spec-language; spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md; PR3287 ae565a5dd5ece5f080d7e6625b7a32b2242a8d82 over f87b2a5c43"
review_set: subset
---

## Summary

Ticket: QSL-2739. Scope-boundary review of the PR1/PR2 split and of the author's open questions against the architecture ruling (lead-supplied `/tmp/qsl-2739-arch-ruling.txt`) and FSpec FR-290/FR-331/FR-348/FR-491/FR-492. PR1 correctly does not implement or claim FSpec FR-348-AC-1/AC-2. But FR-1044 refuses every `assume-guarantee` selection outright, while the ruling admits compose declarations and makes PR2's provider assume-guarantee only, so the spec precludes PR2. Separately, the ruling's AG-only PR2 provider cannot deliver FR-348-AC-2, to which FR-1044 assigns it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1044 SHALL settle an `assume-guarantee` selection `invalid-request` and "SHALL NOT route it to a provider or settle it itself" (AC-4, exit 20), justified by "no analyze request carries an FR-492 settlement request"; the ruling admits compose declarations by name with PR2's provider assume-guarantee only, and FSpec FR-492 lets a same-process planner "pass the core's own records with their certification instead of bytes", so the refusal precludes PR2 and contradicts the ruling. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:120 |
| FND-002 | medium | The Decisions assign FSpec FR-348-AC-1/AC-2 (interface mismatch "refutes, naming the mismatching interface and component") to PR2's provider, but the ruling makes PR2 assume-guarantee only, and an assume-guarantee planner result is "never `refuted`" (FSpec FR-492) with a mismatch settling `inconclusive` (FSL FR-530-AC-2); an AG-only PR2 cannot deliver FR-348-AC-2. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:68 |

## Verdict

Changes required, and both findings need an arch decision; the lead routes them.

FND-001 rules in conflict:
- FSpec FR-491 lines 128-129: `assume-guarantee` derives "one planner-local composition item ... | the planner, from FR-492's settlement request"; the other rules derive the routed FR-331 `composition` item.
- FSpec FR-290 line 154: assume-guarantee composition | `composition` | "none; planner-local, not a request item".
- FSpec FR-492 lines 103-105: "A planner that runs in the same process as the producing core may pass the core's own records with their certification instead of bytes; such a record never comes from a request."
- FSpec FR-492 lines 222-233: decision order; a non-proved slot settles the item `inconclusive`, `premise-unproved` naming the slot; `premise-unproved` is written only by the planner.
- Ruling: "compose selected as item existing analyze request by declaredname ... nonproved premise -> premise-unproved never proved"; "PR2 builtin composition provider assumeguarantee ONLY FR492".

Recommended bounded option: an `items` entry naming an `assume-guarantee` declaration selects its planner-local composition item (no FR-331 request item is written for it, as FR-290 says), and FSL builds the FR-492 settlement in process from the same request's component-guarantee premise records. PR1 settles it `unsupported`, `unsupported_projection`/`unsupported-requested-capability` naming `composition`, exit 21, as for the routed rules. The final contract (PR2) settles it by FR-492's decision order, with a non-proved premise giving `inconclusive`/`premise-unproved` and never `proved`, under `Remaining work: QSL-2739, second PR`. This also answers the author's FR-331 question: `premise-unproved` belongs to this planner path only, and FSL never writes it for a routed `interface`, `protocol-refinement` or `refinement-transfer` item (FSpec FR-331 lines 842-849).

FND-002: arch to confirm either (a) PR2 serves assume-guarantee only and FR-1044's Decisions stop assigning FR-348-AC-1/AC-2 to it (they remain owned by a future routed provider, with their own remaining-work ticket), or (b) PR2 also serves the routed `interface` rule. FR-1044 must not keep the current wording under (a).

Clean and examined: no new verb, request family or member; no caller-supplied proof; FR-348-AC-1/AC-2 not claimed as delivered in PR1; the routed `interface`/`protocol-refinement`/`refinement-transfer` vs assume-guarantee semantic split kept distinct (FR-348 refutation vs FR-530/FR-489 inconclusive), as the ruling requires.

## Scope and evidence

Spec-only; no code written. The architecture ruling and the author's report are lead-supplied local files and were treated as the session's routed decision record, not as upstream normative text.

- FR-1044 Description, Decisions bullets 4-5, Behavior bullet 5: examined.
- FR-1044-AC-4: examined.
- FSpec FR-348, FR-491, FR-492, FR-290, FR-331 at origin/main f73bd9f3: context_only.
- FSL FR-530 lines 36-42, 66-75, 88-98, AC-2/AC-6: context_only.

## Dispositions

Round 1, fix commit a0d6cd85503d2d32235981d82d262ac51bfb166e over ae565a5dd5ece5f080d7e6625b7a32b2242a8d82; verified against the committed text, not the author's report.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
