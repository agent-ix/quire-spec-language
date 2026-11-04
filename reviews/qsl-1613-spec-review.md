---
id: SR-20261104115610
title: "QSL-1613 spec review of PR #2287 (FR-480 Selections prose, FR-480-AC-12)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2287 head 170e02f8b; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md (Selections and imports; FR-480-AC-12)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1613. The PR replaces the FR-480 "one namespace, ambiguous in every unit" bullet with per-unit resolution: `N::T` resolves through the writing unit's alias `N`, and a unit that binds no `N` refuses it. The stricter rule is what FR-480 already required. Its first Selections bullet says a `model` or `import` alias "SHALL resolve only within the unit that declares it", citing QSpec FR-151-AC-16. It is also consistent with QSpec FR-322-AC-71, AC-132 and AC-142 (node identity independent of alias) and QSpec FR-480-AC-3 (a second alias of the same package shares the term). The old bullet contradicted the first bullet, and the new one removes that contradiction. No QSpec requirement depends on cross-unit alias visibility.

Units examined: FR-480 "Selections and imports" (all five bullets) and FR-480-AC-12. QSpec FR-151-AC-16, FR-322-AC-71, FR-322-AC-132, FR-322-AC-142 and FR-480-AC-3 were read as context only.

## Verdict

**MINOR GAPS.** The direction is correct and the prose is coherent. AC-12 is compound. The new "every position" SHALL says nothing about the refusal for a non-type-form position, and AC-12 does not exercise one.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new bullet says `N::T` resolves through the writing unit's alias "in every position a type name is written". The refusal half covers only "a state clause's context or a type form". An expression position (variant case `N::T::Case`, `allInstances<N::T>`, `lookup<N::T>`) in a unit that binds no `N` has no specified outcome. AC-12 tests only the context and parameter-type positions. Fix: name the refusal for expression positions, or say "in every position", and add one expression-position case to AC-12 or to a new AC. | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:134-142; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:254 |
| FND-002 | low | FR-480-AC-12 is compound. It covers a profile-alias ambiguity, a unit record's bare name, the further-alias versus first-alias cross-unit case, the first-alias-of-each case and the unbound-alias refusal in one row. Fix: split the per-unit alias cases (the last three sentences) into their own AC. | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:254 |
