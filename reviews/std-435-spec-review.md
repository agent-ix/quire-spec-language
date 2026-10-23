---
id: SR-16262
title: "spec-review of FR-150 aligned to QSpec FR-381 forms, scope and precedence (quire-spec-language PR 1371)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@296e1662; spec/functional/FR-150-check-the-symmetric-population-annotation.md; spec/test-cases/TC-565-s3-accepts-and-refuses-the-symmetric-annotation.md; spec/test-cases/TC-566-every-identity-observing-form-is-refused-in-every-clause-kind.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: references
---

# Spec review of FR-150 aligned to QSpec FR-381 (quire-spec-language PR 1371)

## Summary

Ticket: STD-435. One spec-review pass over `git diff origin/main...296e1662`.
The oracle was QSpec FR-381 at QSpec main d2fab1b0: the breaking forms
(including the new `convert` form), "Scope of the check", "Precedence over
FR-145", the transparent forms, and FR-381-AC-2 to AC-6.

Examined, clean:

- **Form 4** (`convert` of a `Set`/`Bag` of such references to a `Sequence`
  or an ordered set) matches FR-381 word for word.
- **The single-report rule** ("SHALL NOT also report `operator-ineligible`")
  and the per-fold split match "Precedence over FR-145".
- **FR-150-AC-5**, derived by hand: the `Sequence` and ordered-set conversions
  refuse at `convert`. The folded `Sequence` conversion gives only the
  `convert` refusal, because FR-145 admits any step over a sequence. The
  unfolded `Bag` conversion checks, and a non-catalog fold over it is form 1
  at the `fold` because its source still contains the reference. The
  unannotated `Sequence` conversion checks. This matches FR-381-AC-6.
- **FR-150-AC-6** gives `identity-observing-form` for the `peers` fold and
  `operator-ineligible` for the `drafts` fold, each once, as FR-381-AC-5
  requires.
- **FR-150-AC-7** puts the function-body fold under FR-145 alone, as
  "Precedence over FR-145" states.
- The "Remaining work" section is deleted, and no deferral language is left.
  The lead default that the clause scope covers every breaking form is
  FR-381's own text ("refuses a breaking form in every clause"), not a
  narrowing.

Gates: `make check-spec-validation` at 296e1662 exits 0, with no warning on a
touched file. The PR merges cleanly with origin/main and with #1332, #1310,
#1311, #1343, #1346, #1327 and #1356. origin/main is not an ancestor of the
head. FR-150-AC-5 to AC-7 are unique, and no other open PR edits FR-150,
TC-565 or TC-566.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new scope bullet lists seven clause kinds, but no AC refuses a form in an operation precondition or in a refinement row. QSpec FR-381-AC-3 requires the reference-literal and text-conversion refusals in each of the seven kinds. FR-150-AC-3 covers an invariant, a temporal atom, a `terminal when` predicate and a state constraint, and FR-150-AC-2 and AC-5 cover a postcondition. Fix: add a precondition vector and a refinement-row vector (for example a reference literal naming key `a` in a `pre` clause, and a text conversion of a reference in a refinement field row), each refusing at its node, with TC-566 steps. | spec/functional/FR-150-check-the-symmetric-population-annotation.md:86-91,127 | missing-requirement |
| FND-002 | medium | "A form written outside them, such as in a function body, SHALL be checked by the rules that apply without the annotation" covers every breaking form, but FR-150-AC-7 tests only a fold. Nothing shows a reference literal, a text conversion or a `convert` to a `Sequence` in a function body of the annotated unit checking with no `identity-observing-form`. An implementation that scoped only the fold split would pass every AC. Fix: extend FR-150-AC-7 and TC-566 step 4 with a function body that holds a key literal, a text conversion and a `Sequence` conversion of a `Set<Reference<ConfigVersion>>`, each checking. | spec/functional/FR-150-check-the-symmetric-population-annotation.md:86-91,131 | missing-requirement |
| FND-003 | low | The transparent-forms bullet names `sum`, but neither FR-150-AC-4 nor FR-150-AC-7 has a `sum` vector, so `sum` over the population is the only listed transparent form with no test. Fix: add `sum` to FR-150-AC-7's post-clause vector and to TC-566 step 4, for example `sum(p in self.peers: 1)`. | spec/functional/FR-150-check-the-symmetric-population-annotation.md:82-85,131 | missing-requirement |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed. The forms, the precedence
and the new vectors match QSpec FR-381. The scope of the check, which is
STD-435's own scope item, has vectors for only some of its clause kinds and
forms.
