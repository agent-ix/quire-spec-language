---
id: SR-15531
title: "EARS review of STD-299 NFR-015 statement (QSL PR #1223)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@44f497b8ced379b560d10c2707c97b643ca609e4; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: reviews
---
# SR-15531: EARS review of STD-299 NFR-015 statement (QSL PR #1223)

## Summary

Ticket: STD-299. NFR-015 is the only new requirement statement. FR-262, AD-001 and ADR-011 gain
references and rationale but no requirement statement. The NFR-015 statement uses the state-driven
pattern ("While its `std` feature is disabled, each QSL crate ... shall build ..."), and its
subject is each crate in the per-crate table. `make check-spec-validation` passes, with no warning
on NFR-015, TC-1110 or TC-1111.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: the statement has two `shall` clauses under two different states: "While its `std` feature is disabled ... shall build ..." and "while `std` is enabled (the default) it shall build and behave exactly as it does with the feature absent". The second is a separate state-driven requirement. Split it into its own sentence or turn it into an AC (AC-3 and AC-6 already verify it). The validator's `ears:non-singular` rule did not flag it. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:25-28 |

## Verdict

One minor finding. Otherwise the statement conforms to EARS.

## Dispositions

Round 1 reviewed QSL #1223 at c6e3c941e.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
