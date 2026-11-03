---
id: SR-20261103230159
title: "Spec review of QSL-1557 PR #2220 FR-517 AC-15 and TC-1312 row 15"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@d5b2e550570cc3f2a2369b8ef7ea96e8e61e8b72; PR #2220 diff: spec/functional/FR-517-*.md, spec/test-cases/TC-1312-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1312
    type: reviews
---

## Summary

Ticket: QSL-1557. Spec review of the new FR-517 Inputs paragraph, FR-517-AC-15, and TC-1312 row 15. It covers EARS and integrity, scoped to the changed statements. `quire validate` passes on both files (exit 0).

## Verdict

**PASS with one low finding.** AC-15 is atomic and testable, and its oracle is the bytes that `emit_checked_v2::emit` returns for an equal `Request`. TC-1312's scope line, row 15, and trace-tag line are updated consistently. FR-517 is the only spec that names `emit_checked_v2`, so putting the command-line form here does not conflict with any other owner.

## Scope

- `FR-517` Inputs, the `--clock-selections` paragraph: examined.
- `FR-517-AC-15`: examined.
- `TC-1312` Description scope line, row 15, and trace-tag line: examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-15 is narrower than the Inputs paragraph it verifies. Inputs refuses any "value that is not a well-formed clock configuration", and it states that an unknown name is an `UnknownClause` refusal. AC-15 lists only "an unknown `kind` and a member the configuration does not define". The missing-member and wrong-type cases, and the `UnknownClause` outcome, have no acceptance criterion. Fix: widen AC-15 to "a value that is not a well-formed clock configuration (unknown `kind`, missing or undefined member)", and add "a name matching no clause is `UnknownClause`". Alternatively, narrow the Inputs wording to match. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:198 |
