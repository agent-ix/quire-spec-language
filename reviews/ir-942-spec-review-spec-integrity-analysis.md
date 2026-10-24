---
id: SR-13683
title: "integrity review of IR-942 ADR-013 O-07/O-09 per-step record identity (PR #1157)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@da13209b734d509e800eda6951e70c4619d9e9d2; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
---
# SR-13683: integrity review of IR-942 ADR-013 O-07/O-09 per-step record identity (PR #1157)

## Summary

Ticket: IR-942. Consistency of the O-07 and O-09 amendments with QSpec
FR-322 (source_map, `generated` role, FR-322-AC-87), FR-375 (`steps`
total order, FR-375-AC-19), FR-468 (per-step queries and certificate parts),
QSV FR-123 (`StepSimulationRecord` and its canonical encoding), QSL FR-146
and the owner rulings for this ticket.

The owner rulings hold. The O-09 paragraph's preimage is an RFC 8785 object
with exactly `subject_kind` = `protocol-refinement-step` and `record` =
QSV's canonical `StepSimulationRecord`. The O-07 occurrence key is excluded
from that digest, in both paragraphs. The record members (`refinement`,
`obligation`, `hypotheses`), the unsorted hypotheses and the zero-based
row position agree with QSV FR-123. One step query and one certificate step
part per row, in `steps` order, agrees with FR-468. Using the FR-375 total
order (not source order) for ordinals agrees with FR-375-AC-19 and FR-146.
The companion CG PR (quire-contract-codegen#491, FR-015-AC-120 to AC-122)
states the same preimage, exclusions and refusals.

Examined: the O-07 amendment paragraph, the O-09 amendment paragraph, the
O-07 table Conversions row, the O-09 table Public type row, FR-322 source_map
(`generated` role definition) and FR-322-AC-87, FR-375 checked-package node
(`steps` order) and FR-375-AC-19, FR-468 queries and certificate, QSV FR-123
Behavior 1-3, FR-146 Outputs, Row order and "Record identity, pending".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The O-07 amendment puts `generated` occurrences on the source-declared refinement node itself, with ordinal 0 for `Initial` and `i + 1` for row `i`. QSpec FR-322 defines `generated` as the role of every occurrence of a compiler-derived node (a node no source declares), with the ordinal equal to that node's position among its owner's derived nodes (FR-322-AC-87: the node from row `R1` has ordinal 1, so row `i` gets `i`, not `i + 1`). The refinement node is not compiler-derived, and `i + 1` disagrees with FR-322's ordinal rule. The paragraph acknowledges the departure ("specifying an owner-key mapping") but a QSL ADR cannot redefine a QSpec FR-322 role, and FR-146 says this identity waits on "a QSpec FR-322 occurrence-role ruling". As written, QSL would emit `source_map` keys that FR-322 does not define. Land a QSpec FR-322 change that admits owner-node `generated` occurrences for non-node derived records (with this ordinal rule), and cite it, or cite a recorded owner ruling that settles it. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:254-266 |
| FND-002 | medium | The O-07 table still says "The O-09 obligation identity includes the clause occurrence key, never its regions". The O-09 table still says an obligation is the digest of node id, occurrence key, kind and arguments, "its identity includes the occurrence key", and "The three subject kinds need no separate tag". The new O-09 paragraph adds a fourth, tagged subject kind whose digest excludes both the occurrence key and the obligation kind. A reader of either table gets the opposite rule for per-step records. Qualify both table rows (or name the per-step kind in them) so the tables and the paragraph agree. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:250, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:290 |
| FND-003 | low | FR-146 "Record identity, pending" still says the occurrence key and obligation identity are pending this ADR-013 amendment and that FR-146 "states no role, ordinal or identity preimage". Once this PR merges that text is stale. Replace it with a reference to the O-07 and O-09 per-step paragraphs. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:136-145 |
| FND-004 | low | "The CG acceptance and refusal obligations are specified in CG's own spec under IR-942" points at a ticket, which is status data, not system behaviour. Cite the CG requirement (quire-contract-codegen FR-015, AC-120 to AC-122) instead. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:318-319 |

## Verdict

Not mergeable until FND-001 is settled: the occurrence key depends on a
`generated` role use that QSpec FR-322 does not define. FND-002 should be
fixed in the same PR. The preimage itself matches the owner rulings, QSV
FR-123, FR-375 and FR-468.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | Blocked on QSpec #322 (FR-322-AC-88/89, open at dcc70c07d). FR-322 on main still defines `generated` only for compiler-derived nodes. The 6dc640fa4 text matches #322's rule (Initial ordinal 0 with the declaration region, row `i` ordinal `i + 1` with its source-row region, reader refusal `invalid_package`/`invalid-value`), so it resolves when #322 merges and the ADR cites it. |
| FND-002 | fixed | 6dc640fa4: the O-07 Conversions row says the clause, application and function forms include their occurrence key and the per-step form excludes it; the O-09 Public type row says "These three subject kinds need no separate tag ... The per-step record form below is a fourth subject kind and has an explicit `subject_kind` tag"; the Serialized authority row says per-step content identity excludes both occurrence key and regions. |
| FND-003 | fixed | 6dc640fa4: FR-146 "Record identity and occurrence dependency" says O-09 defines the content identity and CG FR-015-AC-120 to AC-122 govern digest, carrier and refusals; only the O-07 occurrence key awaits the QSpec FR-322 ruling. |
| FND-004 | fixed | 6dc640fa4: the ticket pointer is replaced by "CG FR-015-AC-120 through AC-122 specify its acceptance and refusal obligations." |
| FND-001 | fixed | 2bd8d702b: O-07 now rests on QSpec FR-322-AC-88 through AC-91 (merged as fc98b5e): "QSpec FR-322-AC-88 through AC-91 authorize and validate these owner-node occurrences. FR-322-AC-87 separately covers a node derived from a step row; its ordinal counts derived nodes". Only proof-eligible refinements carry generated occurrences, and an ineligible one carries only its `declaration` occurrence (FR-322-AC-91). Ordinals (Initial 0 with the declaration region, row `i` at `i + 1` with its row region) and the `invalid_package`/`invalid-value` refusal match merged FR-322. |
