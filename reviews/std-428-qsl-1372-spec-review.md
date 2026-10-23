---
id: SR-16269
title: "Spec-review of the ADR-012 rule for contract parts with no construction (quire-spec-language PR 1372)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ce0bc491c4763c3c34164f34db195ade4898ca57; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: references
---

# Spec-review of the ADR-012 rule for contract parts with no construction (quire-spec-language PR 1372)

## Summary

Ticket: STD-428. Head ce0bc491. origin/main is 6 commits ahead and is not an ancestor. The
merge is clean. Gate `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation`: exit 0, no warnings on the touched files.

The new rule was checked against the code. `StageFailure` in `qsl-foundation/src/diagnostic/stage.rs` holds
`Limit`, `Refused`, `Cancelled` and `Fault`. Families take `type Cause = CheckRefusal`. `requirements()` is a
required trait method (`contract.rs:340`). FR-065-CON-2 exists. The build log is gone, and so is the
"deferred part comes back" wording.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [soundness] The new rule says there is 'no stage-participation table'. ADR-012 §5.1's S1 row (line 496) still lists 'the stage-participation table that says which hook each family has at each stage' as a seam that must fail to compile. §13.5 (line 1037) says 'the S1 stage-participation table has one entry per family (§5.1)', and ADR-011:1372 repeats it. §2 is therefore not the one statement of the contract that STD-428 asks for. Fix: drop the table from the S1 row, §13.5 and ADR-011, so the S1 seam is the `catalog_code()` prefix match. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:345-347; spec/decisions/ADR-012-semantic-family-extension-contracts.md:496; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1037; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1372 | wrong-requirement |
| FND-002 | low | [other] The bullet cites '("Packaging" below)', but the Packaging paragraph is at line 288, above the new section (line 326). Fix: say 'above'. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:345; spec/decisions/ADR-012-semantic-family-extension-contracts.md:288 | missing-requirement |
| FND-003 | low | [soundness] The new section names docs/family-migration-recipe.md as 'the procedure a family migration follows to meet these rules'. The recipe (lines 180-193) still records `Requirements`, `CapabilityKind`, `Extent`, `Bound` and `Stage::Requirements` as 'real deferrals'. That contradicts the rule's 'every family implements requirements()'. Fix: update the recipe in this PR, or file a new STD ticket for it. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:357-358; docs/family-migration-recipe.md:180-193 | wrong-requirement |

## Verdict

Not mergeable until FND-001 is fixed. The new normative rule is correct against the code. The same ADR and ADR-011, though, still describe the stage-participation table the rule says does not exist. The two low findings are wording.
