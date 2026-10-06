---
id: SR-1355
title: "Spec review of quire-spec-language PR #650: FR-357, TC-904 and ADR-013 O-09 (QSL-641)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4c2cf5dab9e9f1629d188b9f72001435b4a2d793; spec/functional/FR-357-replay-scalar-parity-claims.md, spec/test-cases/TC-904-replay-scalar-parity-claims.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (O-09 Public type row), spec/spec.md, spec/tests.md; FR-358 step 6 on PR #645 head read as context"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-904
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# Spec review of quire-spec-language PR #650

## Summary

Ticket: QSL-641 (follow-up). This review judges whether the spec conforms
to the four rulings. It does not reopen them.

Checked and correct:

- FR-357 Outputs and Behavior: each report holds the full claim identity
  and exposes it as `claim()`. The consumer bullet makes
  `report.claim() == sent` a SHALL. The observation digest bullet says
  carry-and-bind, and assigns authentication to the driver.
- The recompute bullet names the O-09 preimage, the operand-identity enum
  with the `InlineLiteral` triple and the singleton range, the order (after
  the node checks), and the refusal (`ScalarIdentity`, cause `Obligation`,
  both digests).
- ADR-013 O-09 now limits "never recomputes" to the non-parity paths. It
  defines the operator-parity preimage's member names, tags, number forms
  (ordinal and position as integers, range bounds as decimal strings) and
  lowercase-hex node ids. The code encodes exactly this.
- TC-904 steps 12 to 15 and expected results cover AC-13 to AC-16.
  `tests.md` and `spec.md` are updated to match.

FR-357's Inputs present the occurrence key and the operand identities as
caller inputs, with no check against the recompiled package. The plan
lead's ruling now adds that check. FR-357 (and AC-16's list of refusals)
will need the matching text when the check lands. The defect itself is
recorded once, as SR-1353 FND-001, and is not repeated here.

## Verdict

Request changes, for FND-001 and FND-002. The rulings are otherwise
stated clearly and testably.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-013 O-09 says QSL recomputes the digest "from this preimage" for an operator-parity claim (FR-357) and for a composite parity claim (FR-358). But "this preimage" is the operator-parity one: per-position operand identity and range. FR-358 step 6 (PR #645) recomputes from the clause-form preimage instead: parameter node ids with their declared domains from the harness bounds, ascending by `DomainKey`, with the occurrence key taken from the recompiled package. As written, the two specs contradict each other on what the composite preimage is. Fix: say that each parity claim is recomputed from its own preimage: the operator-parity one defined here for FR-357, and the clause rule with FR-358 step 6's arguments for FR-358. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:280 |
| FND-002 | medium | FR-357 contradicts itself on the operator claim identity. The carriage bullet lists the operator identity as the obligation identity, node, operator, operands with ranges, result range, limits and observation digest, with no native outcome. The consumer bullet says a report with another "outcome" is a report for another claim. For the operator arm the outcome is not in `claim()`, so the consumer check the FR mandates cannot detect it (the code side is SR-1353 FND-002). Ruling 1 asks for the full identity. Fix: add the native outcome to the operator identity list. | spec/functional/FR-357-replay-scalar-parity-claims.md:81-104 |
| FND-003 | low | FR-357-AC-16 and TC-904 step 15 list an operand's "position" among the preimage members whose change refuses. Position is not a claim field: it is the operand's index in `ScalarOperation`, so a caller cannot change it alone, and the test leaves it out (commit 4c2cf5da drops it). AC-16 claims a case that cannot be tested. Fix: remove "position" from AC-16 and step 15. The literal-position distinction is already covered by AC-15. | spec/functional/FR-357-replay-scalar-parity-claims.md:161, spec/test-cases/TC-904-replay-scalar-parity-claims.md:60-61, spec/test-cases/TC-904-replay-scalar-parity-claims.md:89 |
| FND-004 | low | The FR-357 acceptance-criteria table is out of order: AC-15 and AC-16 sit between AC-11 and AC-12, and AC-13 and AC-14 come after AC-12. Fix: order the rows AC-1 to AC-16. | spec/functional/FR-357-replay-scalar-parity-claims.md:158-164 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The rewritten O-09 "Public type" cell breaks the O-09 table. The cell runs on into blank-line-separated paragraphs (*Encoding*, *Operand identity*, *Domain*, *Key*, *Bound*, *Worked example*, *Who computes it*), and a blank line ends a GFM table. The rows after it (`Serialized authority`, `Conversions`, `Validation and diagnostics`, `Equality`) follow the last paragraph with no header or delimiter row, so they render as paragraph text, not table rows. The "Public type" cell is also left without its closing pipe. Fix: end the cell after "each `{operand, position, domain}`." with a pointer such as "(encoding below the table)", and move the seven paragraphs, unchanged, under their own heading after the `Equality` row. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:280-298 |

## Dispositions

Round 1, reviewed at 0679c3acb1a4315d8bf15c850a9d44fbcdab799f (fix commits 4c2cf5da..0679c3ac).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | af7ec0717 |
| FND-002 | fixed | dfab20868 |
| FND-003 | fixed | dfab20868 |
| FND-004 | fixed | dfab20868 |
