---
id: SR-1578103
title: "Spec review of PR #2240: FR-480 repeated model aliases and FR-480-AC-9"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2b4bbc1a5; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md (Selections and imports bullet, FR-480-AC-9); ticket QSL-1578"
review_set: subset
date: '2026-11-04'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1578. The PR adds one FR-480 "Selections and imports" bullet and FR-480-AC-9. The
bullet says a package selected under several aliases is projected once, under the first alias.
Each further alias `N::T` names every `M::T` in every type position and resolves to the same
declaration, and identity stays alias-independent. The bullet and the AC were checked against the
ticket's acceptance text, QSpec FR-322-AC-71, AC-132 and AC-142, and FR-480-AC-3. `quire validate`
on FR-480 is clean.

The coder chose not to mint alias-qualified type identities, and that choice is correct. QSpec
FR-322-AC-71 says two units that select the package under different aliases derive one `Tier` node.
AC-132 says the `model_root` `node_id` is never determined by a model alias. AC-142 says one package
under two aliases derives one `Pair` node. Alias-qualified identities would break all three. The
ticket's literal text asks for them, so the ticket needs a recorded deviation (FND-001).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-480 Selections and imports bullet (repeated aliases) | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:120-133 | examined |
| FR-480-AC-9 | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:242 | examined |
| FR-480-AC-3 | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:236 | context_only |
| QSpec FR-322-AC-71, AC-132, AC-142 | QSpec (by id) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ticket's Scope ("define and implement alias-qualified effective identities ... rekey fields, supertypes, operations ...") and its second acceptance bullet ("retaining alias-qualified type identity in checked artifacts") contradict QSpec FR-322-AC-71, AC-132 and AC-142, which the PR follows instead. The PR body reinterprets "identity" as "names", but the ticket records no deviation, so the ticket's literal acceptance text is not met as written. Fix: before closing, add a QSL-1578 comment that records the deviation (identity stays alias-independent per the QSpec ids above, and the aliases are name spellings only), or amend the acceptance bullet. | QSL-1578 Acceptance bullet 2 |
| FND-002 | low | FR-480-AC-9 is compound: about twelve independent checks (positions, one type, one declaration, graph parity, relationship root, FR-360 variant, record, object and operation positions, inheritance, an alias collision refusal, unit-name separation) share one AC row, so one failure cannot be traced to one criterion. Fix: split it into 3-4 ACs (resolution positions; no duplicated projection; FR-360 and inheritance; adverse collisions). | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:242 |

## Verdict

Conditional. The spec content is correct and agrees with QSpec. FND-001 needs only a recorded
ticket deviation, with no code or spec change. FND-002 is optional.
