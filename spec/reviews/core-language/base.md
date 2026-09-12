---
id: SR-390
title: "Base review of the reconciled core language and producer handoff"
type: SpecReview
analysis: base
scope: "Issues #36/#37/#39/#40/#66: FR-046–050, NFR-009, TC-126–138, IT-009, docs/compiled-protocol-v2.md, spec/spec.md and TM-003; D Producer interface 1.2.0 at 6259d3a5b99088740df9bcc8e8d60f3720aaa603; merged L5 baseline 72507f856457ba0922719bd5d9f5cadcce4058cd; immutable native-v1 baseline 4d6230eb8aa9766ff3017360962f2d6368d74cb3"
review_set: subset
review_date: "2026-09-11"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-046, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/IT-009, type: reviews }
---

## Summary

The expanded retrospective packet now encodes the ticketed L3/L4/L6 language
semantics, the shared composed-evaluation boundary, and compiler #40's narrow
strict `quire.compiled-protocol/2` temporal producer extension. Recheck resolves
all four original findings: the compiler contribution is separated from the
external campaign, verification uses the declared `Test` method, boundary
permutations are explicit, and FR-047 links its owning standard graph rule.

The final base verdict is **PASS AFTER RECHECK**. The #28 reconciliation now
documents the matrix's two intentional status headers, the corrected producer
configuration, the exact pinned verification stack and its negative controls.
That pinned run executed status classification and found no lie. The installed
ambient stack still skips classification and therefore its own
`status_lies: []` is not cited as status evidence. All FR-046–050/NFR-009 rows
remain visibly Planned and unbacked, so implementation and qualification remain
open without becoming a specification-review defect.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | TC-135 lists nine materially different adverse mutations but does not map each one to its authoritative B/F outcome kind, result field and success prohibition, nor identify the selected consumer contract revisions that define those oracles. “Its owning typed disposition” permits incompatible implementations to pass the same prose. | FR-048-AC-10; TC-135 procedure and expected results; IT-009 preconditions/SC-06 | missing-requirement |
| FND-002 | medium | `Demonstration and Test` is not a declared verification method or class, so Quire reports FR-048-AC-10 as an uncatalogued verification method. Use declared methods and retain TC-135 as the linked case. | FR-048-AC-10; Quire coverage diagnostic `uncatalogued-verification-method` | wrong-requirement |
| FND-003 | medium | The choreography cases do not exercise the stated control-boundary permutations: repeat maximum zero versus positive/exhausted behavior, compensation-attempt minimum/maximum, and delivery lower/upper-bound edges. Preservation of one authored value and zero/exact/one-short proof work do not satisfy the constraint-boundary rule. | FR-048 behavior (delivery, repeat and positive attempt bounds); FR-048-AC-2/3/6; TC-132..134 | correct-requirement-no-evidence |
| FND-004 | medium | The graph requirement omits the immutable baseline's directly owning graph rule from its formal relationships, while listing adjacent standard rules; the packet therefore lacks an explicit machine-readable FR-047 → standard FR-043 semantic-authority edge. | FR-047 relationships/dependencies; resources/native-v1/spec/functional/FR-043; issue #37 baseline | missing-requirement |
| FND-005 | medium | TM-003's `Functional Requirement Coverage` table names its status column `Coverage Status`, while the installed traceability model selects `Status`. Quire skips status classification, so an empty `status_lies` list is ambiguous between honest rows and an unexecuted check. Align the column/configuration under #28 and rerun coverage before accepting the base review. | spec/model-linking/tests.md Functional Requirement Coverage; Quire `status-column-matches-nothing`; issue #28 | correct-requirement-no-evidence |

## Resolution recheck

- **FND-001 — resolved.** FR-048-AC-10 and TC-135 now stop A's local
  obligation at source-derived, byte-identical compiler output accepted by B's
  intake. D owns the separately version-locked campaign; B #6/#12 and F own the
  result and observation vocabularies. The former downstream mutations are
  explicitly campaign cases rather than underspecified A outcomes.
- **FND-002 — resolved.** FR-048-AC-10 now uses the catalogued `Test (TC-135)`
  method. Current coverage emits no scoped uncatalogued-method diagnostic for
  FR-046–050.
- **FND-003 — resolved.** TC-132 covers zero/equal/ordered delivery intervals
  and independently invalid lower/order/representation cases. TC-133 covers
  repeat maximum zero, one, the largest finite maximum, normal exit, exhaustion
  and invalid values. TC-134 covers compensation-attempt one, the largest
  representable positive value, zero and overflow, plus unordered concurrent,
  ordered, duplicate-delivery and later-distinct trigger cases.
- **FND-004 — resolved.** FR-047 has a formal `depends_on` relationship to
  `ix://agent-ix/quire-specification/FR-043` and names that immutable rule as the
  graph semantic authority in Dependencies.
- **FND-005 — resolved with an explicit tool-boundary caveat.** The #28 repair
  retains `Coverage Status` for functional rows and `Status` for test summaries,
  records the corrected producer schema and exact pinned CLI/engine/process/ISO
  revisions in `docs/matrix-status.md`, and records a pinned rerun with no
  `status-column-matches-nothing` and no status lie plus positive negative
  controls. The current installed CLI recheck reports 414/472 backed, 336
  criteria and 58 unbacked, but its `status_lies: []` is explicitly discarded
  because the same output reports skipped classification. The newly reviewed
  FR-046–050/NFR-009 rows are all Planned, so none makes a completion assertion
  that the ambient skip could conceal.

## Base checklist and coverage

- FR-046–050, NFR-009 and TC-126–138 are unique and use valid identifiers; the
  merged L5 baseline supplies the previously reserved FR-043–045 and TC-122–125.
  The master frontmatter and Requirements table contain the expanded packet.
- Each of the 40 FR acceptance criteria maps to at least one planned TC in
  TM-003. TC-137 verifies NFR-009's fourteen independently measured dimensions.
  IT-009 verifies only FR-036's real static producer boundary and references,
  rather than overclaiming, the new evaluation requirements.
- Inputs, outputs and error paths cover authority/type/identity substitution,
  invalid and unavailable inputs, unsupported definitions, incomplete closure,
  overflow, exhausted work, version crossing and internally resealed tampering.
  No non-complete outcome is accepted as a value, Boolean, artifact or campaign
  result.
- Query cases cover all eight forms, empty/maximal and nested inputs, duplicate
  occurrences, before/after-decision unavailable values, invalid-input
  precedence, exact prefix arithmetic and partial-output discard. Graph cases
  cover exact storage keys, pre/post identity, absent closure, cycles, self-loop,
  diamond, duplicate/ordered edges and exact traversal limits.
- Choreography cases cover role/channel/control/recovery identities, guard
  visibility, repeat progress, delivery/retry bounds, commit and activation
  transitions. TC-138 covers all three canonical clock alternatives, definition
  bytes/digests, `/1`↔`/2` refusal, exact/one-short limits and v2-specific L5
  binding comparison without assigning clock settlement to the compiler.
- No option set is introduced. State transitions are explicit where applicable;
  retries start with fresh accounting and immutable admitted inputs.

## Verdict and claim limits

**PASS AFTER RECHECK.** All five recorded findings are resolved and no additional
requirement-content blocker was found. Targeted and full Quire validation pass.
The pinned-stack status result and ambient coverage census are deliberately
separate evidence. This verdict does not qualify the Rust implementation: all
new rows remain Planned/unbacked, prior compiler tests are candidate evidence
only after assertion-level reconciliation, and the composed ecosystem campaign
remains separately owned and gated.
