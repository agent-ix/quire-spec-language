---
id: SR-914
title: "QSL-341 spec review of PR 543 (FR-106 check 6.5 and AC-8 conformance sentences)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d0a4b6efe5007b0892ff8a01ba85ba2f22adf815; spec/functional/FR-106-admit-snapshots-and-invocations.md; quire-specification spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md (read, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
## Summary

Ticket: QSL-341. PR: quire-spec-language#543 at d0a4b6ef. Two sentences were
added to FR-106.

What was checked:

- **Check 6.5's sentence.** It quotes QSpec FR-151's conformance definition
  faithfully: "the same effective type or a chain of declared supertypes leads
  from `S` to `T`". Check 10 applies "check 6's value rule" to parameters and
  results, so the one sentence covers both object fields and parameters, as
  the ruling requires. It states what is, names no unsupported alternative and
  adds no ticket id.
- **AC-8's sentence.** It names the three cases the ruling requires.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-8's new sentence starts "In that variant", meaning the Sub/archive package variant. But two of its three cases need fixtures that variant does not have. "An object of a type unrelated to `ConfigVersion`" needs a further type and a population of it; the test adds `Other`/`others` to the base package, without `archive`. "A parameter declared `Sub`" needs an operation `probe` does not have; the test adds `probeSub` and a clause on it. A reader cannot rebuild these cases from the AC. Name the extra fixtures (an unrelated object type and its population; an operation `probeSub(target: Sub)`) or say they extend the variant. | spec/functional/FR-106-admit-snapshots-and-invocations.md:313 |

## Verdict

The spec change is correct and states FR-151 by reference to QSpec. One low
wording finding on AC-8's fixtures.

## Dispositions

Round 1, reviewed at 7ba3a212430669eca995281d090a319420cbc4fe (fix commit 7ba3a212 over the rebased 877d9701; content diff d0a4b6ef..7ba3a212 restricted to qsl-semantics and spec, minus main's #542 changes). Coder's make ci on 7ba3a212: exit 0 (not re-run). Reviewer focused runs: qsl-semantics --features test-support --test it, 446 tests pass; mutants re-run (tuple exact, resolve always-admit, resolve exact, admits exact) each killed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7ba3a212: AC-8 names the extra fixtures: the base `probe` unit plus object type `Other` and population `others` for the unrelated case, and the variant plus `probeSub(target: Sub)` with a precondition for the supertype case |
