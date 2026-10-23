---
id: SR-16337
title: "spec-review of FR-354 admits, emits and reads extension forms by QSpec FR-467/FR-469 (quire-spec-language PR 1305)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0f5332a2; spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md; spec/test-cases/TC-882-an-extension-form-parses-checks-and-packages-as-a-typed-node.md; spec/test-cases/TC-883-malformed-colliding-and-unselected-extensions-refuse.md; spec/test-cases/TC-884-extension-admission-leaves-other-units-and-backends-alone.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-882
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-883
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-884
    type: references
---

# Spec review of FR-354 admits, emits and reads extension forms by QSpec FR-467/FR-469 (quire-spec-language PR 1305)

## Summary

Ticket: STD-347. One spec-review pass over `git diff origin/main...0f5332a2`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- FR-354-AC-2 vectors: the three source-level refusals (`}` expected `days`; `days` where `subject` expected; `}` after a trailing `,`) match QSpec FR-467 "Production" and FR-467-AC-9, and none is `invalid_package`.
- FR-354-AC-6: node id changes with declared name, node kind or clause value and not with inserted blank lines, as FR-469 states (source span in neither key).
- Names FR-467, FR-469, FR-322, FR-133-AC-3, `unknown_required_capability`, `ExtensionFormNode`, `definition_selections`, `CheckedPackageEvidence` (FR-367) all exist on main.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable: no defect in the PR delta. Needs a merge of origin/main (49 commits behind); merge-tree against main is clean.
