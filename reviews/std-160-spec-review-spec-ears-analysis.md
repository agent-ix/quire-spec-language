---
id: SR-13781
title: 'Spec review of PR #1183: type of self in terminal when P (STD-160), EARS analysis'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@788935b87; git diff origin/main...788935b87 (PR #1183, branch std-160-spec): spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md (new behaviour bullet); ticket STD-160'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-124
  type: reviews
---

## Summary

Ticket: STD-160. PR: quire-spec-language#1183, head 788935b87. Method: spec-review with the EARS sub-analysis. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

The one new requirement statement is FR-124's bullet "The checker SHALL give `self` in `P` a type from the model alone". The repository's EARS lint (`make check-spec-validation`) gives no warning on FR-124. The findings below come from a manual reading against the EARS patterns.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-124 behaviour bullet "The checker SHALL give `self` in `P` a type from the model alone" | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:90-101 | examined |
| neighbouring FR-124 behaviour bullets (P checking, None default) | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:84-89,102-103 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The bullet is non-singular. It holds five SHALLs (`give`, `be`, `hold`, `be refused`, `check`) under three different conditions: always, one object type, and none or several. Split it into one statement per condition: the one-type typing and reading, the none-or-several refusal, and the self-free `P` rule. Each split statement can then trace to its own AC (AC-5, AC-7, AC-6/AC-7). | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:90-101 |
| FND-002 | low | "When the model declares exactly one object type" and "When the model declares no object type or several" use the EARS event keyword for a static property of the model. The state-driven or optional-feature form ("Where the model declares exactly one object type, ...") fits the condition. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:92,96 |
| FND-003 | low | "and `P` names its objects through a quantifier over a population of the model (...), which types each bound variable by its population" is a descriptive clause inside a requirement and carries no SHALL. Whether the checker SHALL admit and type that form in a terminal predicate is the testable obligation AC-6 relies on. State it as one, for example "the checker SHALL admit a quantifier over a population `M::P` in `P` and type its bound variable by that population". | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:97-100 |

## Verdict

Mergeable on the EARS axis after the three lows. The statement names its subject (the checker), its conditions are explicit and its outcomes are observable. The defects are compounding and keyword choice, plus one descriptive clause that hides an obligation.

## New findings (disposition pass 1)

Disposition pass 1 at head ce7fddd31. `make check-spec-validation` gives no EARS warning on FR-124. Findings below are from a manual reading of the split bullets.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The two new classifier bullets open "When `P` reads `self`, ..." and "When `P` does not read `self`, ...". Whether `P` reads `self` is a static property of the declaration, not an event, which is the FND-002 defect the fix removed from the typing bullets. Use the optional-feature form ("Where `P` reads `self`, ..."). | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:103,110 |
| FND-005 | low | The quantifier bullet holds three SHALLs: admit the population form, type each bound variable, and "A `P` that does not read `self` SHALL check for every model". The last is a separate obligation under its own condition, with `P` rather than the checker as subject. Make it its own statement with the checker as subject, for example "Where `P` does not read `self`, the checker SHALL check `P` whatever the model's population member types are." | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:98-102 |

## New findings (disposition pass 2)

Disposition pass 2 at head f8de46a6e. `make check-spec-validation` gives no EARS warning on FR-124 or FR-391. The finding below is from a manual reading of the bullets the round-2 fix added.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The missing-name bullet opens "Where those populations cover no object type or several". It used to follow the typing bullet, where "those populations" meant the model's populations. It now follows the new ambiguous-name bullet, whose last populations are "two or more populations" covering `T`. Name the antecedent ("Where the model's populations cover no object type or several"). | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:99 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: the bullet is split into five statements by condition: one-type typing ("Where the member types ... are exactly one object type `T`, the checker SHALL type `self` in `P` as `Reference<M::T>`."), the none-or-several refusal, the quantifier admission, and the two classifier readings. The quantifier bullet still bundles a third obligation (new FND-005). |
| FND-002 | fixed | Round 1: "Where the member types of the model's populations ... are exactly one object type `T`" and "Where those populations cover no object type or several". The new classifier bullets reintroduce "When" for a static condition (new FND-004). |
| FND-003 | fixed | Round 1: "The checker SHALL admit in `P`, for a model with any number of object types, a quantifier or population form over a population `M::P` of the model (`forall(o in M::P : ...)`, `exists(o in M::P : ...)`, FR-391), and SHALL type each bound variable by that population." |
| FND-004 | fixed | Round 2: "Where `P` reads `self`, the deadlock classifier SHALL evaluate `P` once for each object of `T`'s population ..." and "Where `P` does not read `self`, the deadlock classifier SHALL evaluate `P` once at the terminal state (QSpec FR-366)." |
| FND-005 | fixed | Round 2: the quantifier bullet ends "and SHALL type each bound variable by that population."; a separate statement reads "Where `P` does not read `self`, the checker SHALL check `P` whatever the model's population member types are." |
