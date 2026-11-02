---
id: SR-202611020002
title: "gap-analysis of QSL-836 retire native 0-draft compile and runtime (quire-spec-language PR 2068)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR 2068 diff at fd92175a8f5456ef7443988f0719ca64fe1feddc; FR-493, FR-027, FR-007, FR-008, FR-018, FR-024, FR-025, FR-036"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
---

# Gap analysis of QSL-836 (PR 2068)

## Summary

Ticket: QSL-836. Of 94 acceptance criteria whose trace tags sat on deleted lines, 86 have no tag left in the tree; most belong to requirements already gone from spec/ (FR-019 to FR-023). Listed below are the ones whose requirement is still in spec/.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-027-AC-2 and FR-027-AC-3 are live 1-draft criteria and now have no tagged test; the tests deleted from compile_command.rs also covered retained behavior (compile usage arity, exit 30 on failed output, unknown_wire for a wrong format, request-member refusal, invalid_syntax, per-category file-count details) with no successor. | tests/it/compile_command.rs | correct-requirement-no-evidence |
| FND-002 | medium | FR-493 says the retired requirements (FR-007, FR-008, FR-018, FR-024, FR-028, NFR-006, TC-055 to TC-077, TC-099, TC-100, TC-106, TC-431, IT-006) are removed with the path, but spec/ is untouched: FR-007, FR-008, FR-018 and FR-024 alone leave 49 criteria with no tag; FR-036-AC-8 and FR-025-AC-5 are likewise untagged. | spec/functional/FR-493 | correct-requirement-no-evidence |
| FND-003 | low | The native-package vectors left the formatter identity corpus and the historical model-artifact assertion left native_model_profiles while the 0-draft parser and model_source stay until QSL-837; the retained code lost coverage. | tests/it/formatter_identity.rs | correct-requirement-no-evidence |
| FND-004 | low | The ConfigVersion runnable walkthrough (README section, examples/config-version/README.md, requests.sh, fixtures) is deleted with no spine-based replacement; the 17-case corpus survives only as tests. | README.md | correct-requirement-no-evidence |

## Verdict

Resolved: all findings were fixed by PR 2080 (see Dispositions); the original not-mergeable status no longer applies.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Coverage restored in PR 2080 (QSL-836 follow-up); confirmed by SR-202611020004 gap-analysis, Linear QSL-836 comment 12031d29 |
| FND-002 | fixed | Retired-spec cleanup landed in PR 2080; confirmed by SR-202611020004 |
| FND-003 | fixed | Fixed in PR 2080; confirmed by SR-202611020004 |
| FND-004 | fixed | Fixed in PR 2080; confirmed by SR-202611020004 |
