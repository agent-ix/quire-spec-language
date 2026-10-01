---
id: SR-928
title: "QSL-241 spec review of PR 547: QSpec- prefix amendments to FR-087, TC-246, TC-491 and tests.md"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@35c74a26273504a59ff21cd54be903b727544189; spec/functional/FR-087-typestate-and-cross-package-node-key.md (Behavior paragraph at 571-578, AC-7, Remaining-work paragraph at 796-800); spec/test-cases/TC-246-resolved-source-package-retired.md (step 7); spec/test-cases/TC-491-link-a-complete-v1-definition-bundle.md (Description, Implemented); spec/tests.md (Requirements Traceability convention paragraph, TC-491 row, TC-242 backing line)."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-246
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-491
    type: reviews
---
## Summary

Ticket: QSL-241. PR: quire-spec-language#547 at 35c74a26.

Each amendment was checked against the ruling: a `QSpec-` prefix for QSpec
requirement and TC tags, a bare id for a QSL artifact, no local TC rows, and
amended FR-087 wording. Checked:

- FR-087-AC-7: "keeping its QSpec FR-131/FR-339 tags, written
  `QSpec-FR-131-AC-n` and `QSpec-FR-339-AC-n`". The AC is testable, and the
  code matches it (SR-927).
- FR-087 Remaining-work paragraph: "(written with the `QSpec-` prefix since
  QSL-241)". Consistent.
- TC-246 step 7 and TC-491 Description / Implemented: these now spell the
  carried tags `QSpec-FR-131`/`QSpec-FR-339` and
  `QSpec-FR-131-AC-1`..`AC-3`, `QSpec-FR-339-AC-3`. They are consistent with
  the code.
- tests.md convention paragraph: it states the rule once, with an example of
  each form and the reason (independent numbering). It is clear.
- tests.md TC-491 row and TC-242 line: consistent spelling.

No amendment creates a local TC row. No QSpec text was copied in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-087 Behavior restates the repo-wide trace-tag convention, which now lives in tests.md. This gives it two normative homes inside a requirement about retiring `ResolvedSourcePackage` | spec/functional/FR-087-typestate-and-cross-package-node-key.md:574-577 |
| FND-002 | low | The inserted FR-087 line breaks the file's wrap at about 76 columns: line 577 is 101 columns and runs on into the next sentence | spec/functional/FR-087-typestate-and-cross-package-node-key.md:577 |

## Verdict

The amendments are correct and consistent with the code. Two low nits:

- **FND-001:** in FR-087 Behavior, keep only the FR-131/FR-339 spelling
  (which AC-7 already carries) and point to `spec/tests.md` Requirements
  Traceability for the general rule, so the rule has one home.
- **FND-002:** rewrap lines 574-578.

The convention paragraph's gap (61 tags that name nothing) is recorded as
SR-927 FND-001 and FND-002, not repeated here.
