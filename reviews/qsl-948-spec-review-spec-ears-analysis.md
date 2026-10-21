---
id: SR-13721
title: "EARS review of PR #1169 (FR-397 IEEE profile selection)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@d8477bd126f11395bd4ad28120bcdcd063bd5908; git diff origin/main...HEAD (PR #1169): spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-holds-an-ieee-operation.md, the edited sentences of FR-091 and FR-093; ticket QSL-948"
review_set: subset
date: "2026-10-21"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: reviews
---
# SR-13721: EARS review of PR #1169

## Summary

Ticket: QSL-948. EARS conformance of FR-397's ten SHALL statements, its
declarative Behavior sentences and its six ACs, plus the two edited FR-091 and
FR-093 sentences. The repository grammar linter (`make check-spec-validation`,
quire 0.36.2) reports nothing on any changed file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "A header `profile` naming its identity SHALL still refuse `unknown_profile`/`wrong-selection-role` (FR-110-AC-3), and no other unit declaration supplies the row." has a source construct, not the system, as its subject. "still" describes this change rather than the behaviour, and the trailing clause is a second, unmarked obligation. Restate it as "The checker SHALL refuse a header `profile` naming the `ieee_profile` identity as ... (FR-110-AC-3)", and give "no other unit declaration supplies the row" its own SHALL or make it a plain reference. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-holds-an-ieee-operation.md:62-64 |
| FND-002 | low | AC-4's "still refuses" uses the same change-relative wording. "refuses" alone states the behaviour. | FR-397-AC-4 |

## Verdict

Two low findings. Examined and clean:

- The trigger, selection, profile-in-use, comparison and absent-profile SHALLs
  are ubiquitous statements, each with a named agent (the checker, the
  package's definition selection, the evaluator) and an observable outcome.
- The declarative sentences on rounding mode and on float-to-rational lowering
  are definitions, each citing its owning FR.
- The FR-091 and FR-093 edits state ownership, not a changelog.

## Dispositions

Re-checked at head 99d30b1cc9a4c10138f6abbcc6520035e258687c.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 99d30b1cc: "The checker SHALL refuse a header `profile` that names the row's identity as `unknown_profile`/`wrong-selection-role` (FR-110-AC-3), and no other unit declaration supplies the row." The checker is the subject and "still" is gone. The trailing clause stays as a statement of fact, not an obligation, which is acceptable. |
| FND-002 | fixed | 99d30b1cc: AC-5 reads "... refuses `unknown_profile`/`wrong-selection-role` in a unit that holds a float function and in one that holds none." |
