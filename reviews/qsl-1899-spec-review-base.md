---
id: SR-2690001
title: "base review of FR-576 operation-body carrier (PR 2690)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Base checklist over the one added file, FR-576, read
against FR-094, FR-102, FR-103, FR-302, FR-494, FR-496 and QSpec FR-322 on
`main`, and against the dispatch-link code that FR-094/FR-302 describe.
Review set `subset`: base, ears-conformance, integrity, dependency,
scope-boundary. criterion-strength (Jev) was not run; AC strength was judged
manually here. No AssuranceProfile exists under `spec/`. `quire validate`
on the file reports no errors (two EARS warnings, see the ears-conformance
review).

IDs, sections, US link and Verification cells conform. No hash, digest, pin
or tracking-record antipattern: AC-6's byte-identity is a determinism check
on emitted output, not a recorded digest.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-576 requires a body record naming the FR-094 body function node for every operation with `has_body`, and lists the body as FR-302-checked. Both exist only for operations reached by a dispatched call: FR-094 builds body functions only for a dispatch-eligible operation in `checked_dispatch`, FR-302 checks a body only "when dispatch linking first reads" it, and `link_dispatch` returns early when the unit makes no dispatched call. For an operation that has a body but is never called, the node FR-576 must reference does not exist and the body is unchecked. FR-576 neither requires building and checking it nor defines the record for that case. AC-1 does not state a dispatched call, so as written it requires a node that no requirement builds. | FR-576 Behavior bullet 1; FR-576-AC-1; FR-576-AC-7; FR-094 "Clause function nodes"; FR-302; qsl-semantics/src/check/dispatch_link.rs:144-150 |
| FND-002 | medium | AC-4 compares the package with the package "with the body records removed". Read as removing the records after emission, it is tautological: removing sidecar rows cannot change graph nodes. It can fail only if emission runs with the record writer disabled, and the AC does not say that. Once FND-001 is fixed by building body nodes for uncalled operations, AC-4 must also say whether those nodes count as record-caused graph changes. | FR-576-AC-4 |
| FND-003 | medium | Two Behavior rules have no criterion. The alias rule ("one operation reached through two model aliases keep separate records") is not tested. Inheritance and redefinition are not covered: whether a subtype that inherits `A.size` without redefining it gets its own record, and whether a redefining `Sub.size` with a body is keyed apart from `A.size`. FR-496 states the inherited case explicitly for effects. | FR-576 Behavior bullet 3; FR-576-AC-3 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | AC-8 and the omit-the-member rule depend on "the producer cannot supply the body or absence of one operation". Since FR-576 now requires `check` to build and check every body, no requirement says when that state can occur, so AC-8's precondition cannot be set up. Name the concrete producer condition, for example a family that does not link under FR-302, or remove the rule and AC-8. | FR-576 Behavior bullet 6 (lines 95-98); FR-576-AC-8 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Behavior lines 83-88 require `check` to build and check the FR-094 body function for every operation with `has_body`, whether or not a call reaches it; AC-2 tests an uncalled body, and AC-1 now states the dispatched call. |
| FND-002 | fixed | The post-hoc-removal AC is gone. AC-8 compares against a separately emitted package without the member, and Behavior lines 103-104 scope graph equality to "the same bodies". |
| FND-003 | fixed | AC-5 covers an inherited operation (no row) and a redefined one (own row), and AC-6 covers two aliases of one document (one row). |
| FND-004 | fixed | Round 2: the omit-the-member rule and its AC-8 are removed. Behavior now states that every body is built and checked or the compile refuses, so the emitter always has every row; Outputs no longer allows omitting the member. |
