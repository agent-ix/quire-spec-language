---
id: SR-60021
title: "Spec review of QSL PR #1592 and QSpec PR #445: release and delay as model-level declarations (QSL-1004)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8b97d4816473c92a229589e91342c12b547c15eb (FR-249, FR-253, ADR-026 RT-8, TC-704); agent-ix/quire-specification@2e81f694c32c9f78e8d9401ae48d08ae306e6e2f (FR-405, FR-408, FR-420, FR-421, TC-350, TC-374)"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-249
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-253
    type: reviews
---
## Summary

Ticket: QSL-1004. Two spec changes: (A) the spec edits of QSL PR #1592 and (B)
QSpec PR #445, a wording-only change. Verdicts: A PASS with findings, B PASS
with findings; no blocker in either.

(A) FR-249 reads coherently end to end: Description, Inputs, Behavior and the
References paragraph all say `release S.τ on M::o;` is a model-level declaration of
QSpec's shared grammar (`release-decl`, shared-grammar.md:971 and the prose at 1012),
and AC-1's `release Ctl.a on M::sample;` is the same spelling as AC-3. A grep for
`releases` across the QSL repo (spec/, ADRs, traces, source) finds no remaining
`releases τ` operation-member wording; the hits are the memory-model `releases()`
helpers in qsl-eval/qsl-analyze and unrelated review text. ADR-026 RT-8 matches
FR-249. The new AC-3 is testable: each clause is a concrete input with a stated code
and span, and TC-704 scope ("FR-249-AC-1 to FR-249-AC-3"), fixtures and step 3 say the
same thing as AC-3 (modulo FND-001 and the gap-analysis SR-60020 findings).
AC-1/AC-2 stay with QSL-585 and the edit does not claim them.

(B) The diff is six files (FR-405, FR-408, FR-420, FR-421, TC-350, TC-374); nothing
under proposals/ changed, so the shared grammar is untouched, and `delay-decl`
(`'delay' qualified-name '~' delay-dist ';'`) is already model-level. `quire validate`
over the six changed files exits 0 with only the repository-wide archetype
warnings that also appear on origin/main. FR-405's new sentence and AC-6 are
consistent with QSL FR-253 and with the implementation (`DelayFault::RepeatedDeclaration`
refuses `invalid_model_binding`/`malformed-declaration` at the second declaration).

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | FR-249 Behavior says `ambiguous_declaration`/`ambiguous-name` when `M::o` "names operations of several declaring types". The resolver (`ProbabilisticInput::operation`) is ambiguous when two selected models expose the same object type (or inheritance leaves two candidates), not when "several declaring types" are named. Reword to "when `M::o` names an operation in more than one model or is ambiguous by inheritance". | FR-249 Behavior |
| FND-002 | low | Operation-member `delay` wording remains in the QSL spec that PR A's FR-253 sentence now contradicts: ADR-026 SD-1 ("may declare `delay ~ D`", "no `delay` member", line 232), OV-7 (449) and the line at 551; ADR-028 lines 60, 744 and 805; TC-642 (14), TC-708 (19) and TC-709 (19) fixtures ("no `delay` member on `timeout`"). Same fix as B: "declaration". | spec/decisions, spec/test-cases |
| FND-003 | low | FR-249 and QSpec FR-419 now carry one overlong first line in the Description (`..., each by a` / `` `release S.τ on M::o;` declaration, is a task automaton. For the decidable class, S3 SHALL lower the model and its``); rewrap. Cosmetic. | FR-249:21-22; FR-419:137-138 |
| FND-004 | low | (B) FR-420-AC-1 and AC-6 still say "`reply` with `delay ~ discrete { ... }`" and "`reply` under `delay ~ uniform`", the operation-member shorthand PR B removes everywhere else in FR-420 (the TC-374 vectors write the full `delay Call::reply ~ ...;`). Write `delay Call::reply ~ discrete { ... }`. | FR-420-AC-1, AC-6 |
| FND-005 | low | (B) The error classes disagree between siblings: a second `random` or `reward` refuses `invalid_probabilistic_model`/`duplicate-distribution` or `duplicate-reward` (FR-405-AC-8, AC-9), a second `delay` refuses `invalid_model_binding`/`malformed-declaration`. The PR matches the shipped QSL-1003 code, so it is correct as documentation; a `duplicate-delay` under `invalid_probabilistic_model` would be the symmetric design. Decide deliberately, not by accident; no change required here. | FR-405 AC-6/8/9 |

The FR-253 SHALL with no AC or test is in gap-analysis SR-60020 FND-001.

## Verdict

(A) PASS with findings (see also SR-60019, SR-60020). (B) PASS with findings;
FND-004 is the only edit worth making in #445.
