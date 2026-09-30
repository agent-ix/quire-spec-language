---
id: SR-905
title: "QSL-337 spec review of PR 540 (FR-121, TC-516, spec.md and tests.md rows)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a44e9c9a5ebae435347134a03ddc9d6da1631f1e; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/test-cases/TC-516-locate-a-function-call-site.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-516
    type: reviews
---
## Summary

Ticket: QSL-337. PR: quire-spec-language#540 at a44e9c9a. Base spec review
(structure, EARS SHALL statements, AC testability, consistency with code) of
the FR-121 rewrite, TC-516's new steps 3 to 5, and the index rows.

Checked and clean: FR-121's Inputs and Outputs match the code's signature
and types exactly; each Behavior SHALL is observable; AC-1 to AC-5 are each
testable and each is verified by a TC-516 step with matching Expected
Results; the new `depends_on` edges (FR-105, FR-115, FR-116) are real
dependencies; the spec.md and tests.md rows list AC-1 to AC-5 and agree with
TC-516. Removing QSL-317 ticket ids from the rows and comments is right:
ticket ids there were ceremony.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Description says `qsl_replay` re-exports "every type a `call_site` input or answer names", then lists five. `SourceHolder::Library` and `DependencyInputRefusal::EmptyVersion` name `LibraryName`, and the refusal's `code()`/`host_cause()` return `Code`/`HostCause`; none is re-exported. Narrow the sentence to the listed types, or re-export `LibraryName`. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:55-59 |
| FND-002 | low | Two new sentences state what is not: "An invariant names no operation and is never returned." and "no other type selects". The positive statements ("every `pre` and `post` clause ..." and "the sealed trait both implement") already fix the set. AC-3's "Neither invariant ... is returned" is a test assertion and is fine. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:77-78,121-122 |

## Verdict

The spec edits are sound and match the code. Two low wording findings.
