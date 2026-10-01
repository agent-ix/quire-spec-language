---
id: SR-951
title: "QSL-360 spec review of PR 558: ADR-011 OQ-6 and §2.4, FR-087, FR-093 and AC-17, FR-110, TC-416 step 9"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@13f0b2a6cf2a50af6768366cb754b0c3995ad71a; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/functional/FR-087-typestate-and-cross-package-node-key.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-360. Base review of the spec diff.

- **ADR-011 §2.4 table row and 2026-09-24 amendment.** Accurate: `DefinitionLock`
  reads `complete-value-lock.json` from the `quire-specification` crate and holds no
  copy. The diagnostics sentence matches `native_diagnostics_catalog`.
- **ADR-011 2026-09-26 amendment, the native-compile bullet, FR-087 (two places),
  FR-110 Behavior and Inputs.** "QSL's `DefinitionLock`" becomes "the
  `DefinitionLock`", and "no QSpec accessor" is removed, since there now is one.
  Accurate.
- **FR-093 diagnostics paragraph, FR-093-AC-17 and TC-416 step 9.** They state the
  value-by-reference rule (header revision, SHA-256 of the bytes) instead of a
  literal `1-draft.8`. Testable, and the emit test plus the semantics test back it.
- **No ticket ids in new prose.** The added and changed lines carry none. The
  `STD-129`, `QSL-247`, `IR-450` in FR-093-AC-19 are on unchanged lines.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | OQ-6 now says "QSpec checks each digest against the bytes of the file it names (§2.4)". §2.4 says nothing about QSpec checking digests. That check is QSpec's own (its crate doc names TC-233). Cite QSpec's check, or drop the parenthetical. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1362-1367 |

## Verdict

The spec edits are accurate and match the code. One low citation fix.
