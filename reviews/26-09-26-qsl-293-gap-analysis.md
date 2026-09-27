---
id: SR-743
title: "QSL-293 gap analysis of PR 493 (FR-096-AC-1 embedded body)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@09e9d18585d4aed678ce681ef824a24b19a24b27; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/test-cases/TC-426-a-check-location-resolves-to-its-unit-region.md; qsl-semantics/src/check/region.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/mod.rs; qsl-source/src/lib.rs; qsl-foundation/src/source_map.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-426
    type: reviews
---
## Summary

Ticket: QSL-293 (blocks QSL-160). PR: quire-spec-language#493 at 09e9d185.

TC-426 step 3 is backed by
`an_embedded_body_resolves_its_locations_under_the_document_shifted`,
tagged `#[trace("TC-426", "FR-096-AC-1")]`. The test resolves the four
locations under `d`, shifted by `k`, from the declarations, their regions and
the checked package. It checks the source reference, the start and end
positions, and the document bytes. Steps 1, 2 and 4 are backed by the
QSL-239 tests, so all four steps are covered.

Mutation check: a detached worktree at 09e9d185 with its own
`CARGO_TARGET_DIR`, running `cargo test -p qsl-semantics --lib
check::region::tests::an_embedded`. The baseline passed (4/4 region tests).
Eight mutants were run. Five were killed: the shift is dropped (the embedding
is ignored), and `None` is passed in `CheckedGraph::region`,
`DeclarationRegions::region`, `PackageDeclarations::region` or
`PackageDeclarations::declaration_region`. Three survived. See FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No production path sets `embedding`. `PackageDeclarations::new` always sets `None`, and nothing outside the test assigns the field. `qsl-source` builds a document `SourceMap` for an embedded body, but that map never reaches the assembler or `PackageDeclarations`. As a result, a real Markdown-embedded unit still resolves under the body reference, and the FR-096 behaviour "when the unit is a body embedded in a document" is only reachable from a test. This meets the ticket's done-when (a test that fails when the shift is dropped), but FR-096-AC-1 is not delivered end to end. Wire it in this PR, or defer to a follow-up ticket that states the gap in FR-096's Status. | qsl-semantics/src/check/check.rs:357; qsl-source/src/lib.rs:304-344 |
| FND-002 | medium | Three mutants survive. (a) `DeclarationRegions::declaration_region` passes `None` (region.rs:115). (b) `CheckedGraph::declaration_region` passes `None` (region.rs:181). (c) The split rule is replaced by "take the first region" (region.rs:59). The test checks the declaration span through `PackageDeclarations::declaration_region` only, and only its start (region.rs:520-525). The FR-096 text "a declaration as a whole ... mapped the same way", together with "the checked package SHALL resolve ... by the same rule", is untested for the other two views. The split disposition has no test at all. Assert the declaration region from `regions` and `checked` too, with start and end. Add a test for the split case once SR-744 FND-001 settles what it should do. | qsl-semantics/src/check/region.rs:115; qsl-semantics/src/check/region.rs:181; qsl-semantics/src/check/region.rs:59; qsl-semantics/src/check/region.rs:520-525 |

## Verdict

Approve with changes. Fix FND-002 in this PR. FND-001 is either fixed here or
deferred with a ticket and an honest FR-096 Status line.
