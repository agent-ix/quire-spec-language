---
id: SR-12115
title: "Spec review of quire-spec-language PR 949: FR-382, FR-124 restated to the shared grammar, and the terminal declaration wording sweep"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7e2480dc5d66f1a03473dafe3979455f774158fd; spec/functional/FR-382-build-the-terminal-declaration-form-at-s2.md, spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md, spec/test-cases/TC-1059-s2-builds-the-terminal-declaration-form.md, spec/test-cases/TC-1060-s1-parses-terminal-declarations.md, spec/spec.md, spec/decisions/ADR-018, ADR-020, ADR-021, ADR-022, ADR-027, spec/functional/FR-125, FR-126, FR-128, FR-143, FR-144, FR-167, FR-230, FR-338, spec/test-cases/TC-519, TC-520, TC-521, TC-525, TC-537, TC-548, TC-592, spec/usecase/US-015; context FR-165, FR-211"
review_set: subset
---
# Spec review of quire-spec-language PR 949

## Summary

Ticket: QSL-857. One spec-review pass over the spec files of
`git diff origin/main...HEAD`. Integrity, test-intent and cross-file
consistency are folded into this one artifact. QSpec was read read-only and is
cited by id only.

- Grammar authority. FR-124's S1 statement and FR-382's Inputs give QSpec's
  shared-grammar `terminal-decl` exactly. QSpec FR-366 states the package-level,
  per-model-alias declaration, the `when { P }` / `any` forms and the reserved
  word. The restated ADR-018 DL-1, DL-3 and DL-5 match it.
- FR-124 restatement. The per-state-model rule "at most one `terminal` member"
  becomes "at most one per model alias". The `ambiguous_declaration` refusal,
  the FR-104 predicate check, `TerminalDeclaration::None`, and package identity
  survive. No requirement is deleted. FR-124-AC-1 to AC-3 are kept, and AC-4 is
  added for S1.
- FR-382 and its TCs. AC-1 to AC-6 are each atomic and testable, and TC-1059
  rows 1 to 6 map to them one for one. TC-1060 rows 1 to 5 map to FR-124-AC-4.
  The interim assembly refusal (AC-6) is stated as current behaviour, by
  planner ruling, with the existing catalog pair
  `unsupported_construct`/`declaration-form`.
- Wording sweep. Every changed sentence in the 21 ADR/FR/TC/US files is still
  true under the package-level declaration. ADR-027 PD-1 and FR-211 rightly keep
  the protocol clause's own `terminal` member. The sweep missed three sentences
  (FND-001).
- Ids. FR-382, TC-1059 and TC-1060 exist only on this branch, on main and every
  remote branch. `spec.md` gains FR-382 in both the relationship list and the
  index table.
- `quire validate` (quire 0.36.2) on the touched files: clean.
  `make check-spec-validation`: EXIT=0, with no warning on a touched file.
  `tools/check-index-completeness.sh`: EXIT=0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The sweep missed three sentences that still make the state model the declarer, which contradicts FR-124's own restated first sentence and ADR-018 DL-1/DL-3 as reworded. FR-124's Description says "unless that subject's state model declares `terminal any`". FR-124's Behavior says "When a subject's state model declares `terminal any`, the request writer SHALL add no deadlock-freedom item". FR-165's Behavior says "add none for a subject whose state model declares `terminal any`". Under the package-level `terminal-decl`, a state model declares nothing, so read literally the opt-out never fires. Each should read as ADR-018 DL-3 now does: "unless the package declares `terminal any` for the subject's state model". | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:29,118; spec/functional/FR-165-check-state-graph-claims-at-s3.md:125 |

## Verdict

PASS with one medium finding recorded. FR-124 and FR-382 state the shared
grammar's `terminal-decl` exactly. The new criteria are atomic and map one for
one to TC-1059 and TC-1060. No requirement is lost, and the validators are
clean. Three request-writer sentences still name the state model as the
declarer (FND-001). That is a three-sentence wording fix that falls within the
sweep's own purpose.
