---
id: SR-1635002
title: "Gap analysis of quire-spec-language PR #1635: dimension equivalences (580g)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d2934f0b83ac4e6548acfbde998d8227ba8c7ba8; PR #1635 against FR-091-AC-59, AC-60, AC-61, AC-62, FR-230-AC-27, AC-28 and TC-482/483/685; QSpec FR-142 Dimension equivalence and FR-154 package dimensions, checked-package-v2 schema (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1635

## Summary

Ticket: QSL-580, slice 580g. Plan completion: not assessed. This review used
the trace attributes on the PR head, the AC text and the code.

Backed by tests:
- FR-091-AC-59: the forms and the nine malformed spellings.
- FR-091-AC-61: the Q12 and Q13 keys, order and spelling independence, the
  three QSpec package-time vectors, and `nominal_spans`.
- FR-230-AC-27 and AC-28, check halves: the converted bound, reset and period,
  and the refusals that remain.
- FR-091-AC-60 and AC-62: the base-pair decisions and the `Time` class.

Deferrals checked:
- **Identity halves of AC-27 and AC-28: real blockers.** They need the timed
  model in S4 (STD-493 item 9, a real spec gap). They also need IR to emit
  `dimension_equivalence`. QSpec's checked-package-v2 `schema.json` lists
  `dimension_equivalence` among the `scalar_type` forms, but the IR commit QSL
  locks (quire-contract-ir eb007b2) has no `dimension_equivalence` in its Rust
  sources. That is cross-repo IR work, not a spec gap.
- **Derived package dimensions and FR-091's turn-list phases: not blocked**
  (FND-001).
- **FR-142's other effects** (`convert<…>` across a class, products and
  quotients over class roots) and FR-398-AC-6 are outside the ACs this PR
  claims. Noted, not flagged.

## Verdict

**CONDITIONAL.** FR-091-AC-60 and AC-62 are tagged complete but leave out
vectors that can be built now.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-091-AC-60 is tagged in full, but the test leaves out several vectors: `R::Time ~ R::Area`, which refuses `unsupported-feature` instead of `incompatible-definition`; the `Dur = R::Time` admit and refuse pair; the package `Speed` pairs; and the two turn-order vectors (`L ~ L` refused `definition-cycle` before row 1 at `z`, and `Dur ~ L2` refused at row 1, not `incompatible-definition`). These can be built now. Package dimension `terms` are in the FCD document, but `PackageDimensionRecord` keeps only `base`, which is why a derived package dimension gets no key or map. FR-091's two-phase turn order is fixed by the spec and needs no new spec text. | qsl-semantics/tests/it/dimension_equivalence.rs:84-275; qsl-semantics/src/model/domain_package.rs:752-759; qsl-semantics/src/check/assemble/equivalence.rs:330-340 |
| FND-002 | medium | FR-091-AC-62 is tagged in full, but the test covers only "its class half". The `Mass` vectors are not tested: `missing-name` at `F::kg`, and the base-dimension rule naming `ix://acme/flow/dimension/Mass` and `n`, with or without an equivalence. Neither is Q12's `Distance`/`Length` class. | qsl-semantics/tests/it/dimension_equivalence.rs:330-348 |
| FND-003 | low | The tests tagged FR-230-AC-27 and FR-230-AC-28 do not cover the identity halves, which are blocked by STD-493 item 9 and the IR vocabulary. The tags claim the whole ACs, and only the doc comments say what is missing. | qsl-semantics/tests/it/timed_declarations.rs (`an_equivalence_of_equal_roots_converts_the_other_owners_time`, AC-28 test) |
