---
id: SR-17012
title: "Gap analysis of quire-spec-language PR #1448: QSL-985 refinement remainder"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8d8060bdf43c3742e6f4c3aacb94fe3cfbd2bc1d; PR #1448 (QSL-985) diff origin/main...HEAD; criteria FR-135-AC-6, FR-138-AC-9, FR-139-AC-7, FR-140-AC-15, FR-139-AC-6, FR-136-AC-5, FR-140-AC-14, FR-140-AC-11"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1448

## Summary

Ticket: QSL-985. This is a planless gap analysis against the computed matrix
(`quire matrix --scope . --format json` at the reviewed head) and the source.

Matrix status at the head:

- Tagged: FR-135-AC-6, FR-138-AC-9, FR-139-AC-7 and FR-140-AC-15.
- Untagged: FR-139-AC-6, FR-136-AC-5, FR-140-AC-14 and FR-140-AC-11.

The PR's tests for the untagged criteria carry no AC id in a `#[trace]` or a
doc comment, which is correct for partial evidence. The four untagged
criteria are filed as QSL-1056 ("QSL-985 remainder"), which also records
FR-139-AC-6's S3-versus-admission conflict, FR-140-AC-11's missing
`Some`/`none` constructors, FR-140-AC-14's unreachable path and FR-136-AC-5's
need for S4 emission.

Clause by clause:

| Criterion | Clause | Status in the test |
| --- | --- | --- |
| FR-135-AC-6 | `Sequence<Reference<Queue>>[0, 2]` field, row of type `Sequence<Reference<Ring>>[0, 2]`, refuses `unsupported_construct`/`not-yet-implemented` naming `nested-reference-row` | asserted: code and cause, `feature.name()`, the only refusal |
| FR-135-AC-6 | at the row's expression | asserted (`at == "self.chain"`, the root span of the row expression) |
| FR-135-AC-6 | `cells = R::slots` checks | asserted (the refinement checks, with two object rows) |
| FR-135-AC-6 | the same field as a plain `Reference<Queue>` mapped by `self` checks | asserted (`head = self` in the same checked refinement) |
| FR-135-AC-6 | beyond the AC | the option case is asserted, along with the profile identity and that `subject` is absent |
| FR-138-AC-9 | Decimal row refuses once at the type, `history-row`, `subject` `decimal`, no other diagnostic | asserted (`codes` has exactly one entry, `at` is the declared type) |
| FR-138-AC-9 | enum and `Option` of the enum refuse with `subject` `enum` | asserted |
| FR-138-AC-9 | `Int[0, 1]`, Boolean and `Option<Int[0, 1]>` rows check | asserted, each with its declared type |
| FR-139-AC-7 | the undefined `only` for two objects and for none carries an `UndefinedRecord` with reason `only-not-one` | asserted for both the `LONE` (found=2) and `GHOST` (found=0) rows, at two states. The tag was added to an existing test whose assertions already cover the AC. |
| FR-140-AC-15 | two populations whose digest order is opposite to their `DeclarationKey` byte order | asserted as a precondition (`cells < queues` by key, `queues < cells` by node bytes). It uses the ring-queue `queues`/`cells` pair, not a fixture named `TwoPopulations`. |
| FR-140-AC-15 | both undefined: the population first in digest order is reported | asserted (`items` of `r`) |
| FR-140-AC-15 | only the other population undefined: that one is reported | asserted both ways (only `queues`, and only `cells` giving `value` of `s0`) |

All four tagged criteria are complete. The coder's mutations on each tagged
path (digest-order revert; the Option arm removed; the exemption removed; the
collection element not walked) were re-run by the reviewer and all were
killed.

Reverse gap:

- `nests_reference`, `holds_reference` and `PopulationRow::abstract_node`
  each have a production caller and an owning requirement (FR-135, FR-140).
- The composite arms of `holds_reference` have no test, and no well-typed row
  can reach them (SR-17011 FND-001).

No stubs were found.

## Verdict

**CONDITIONAL.** The four tagged criteria are complete and mutation-backed.
The four untagged criteria are named in QSL-985's own work list and are
deferred to QSL-1056 with recorded reasons. The strict FAIL rule ("any
untagged criterion") is therefore met only by criteria the ticket explicitly
defers. FND-001 records that, so the lead can confirm the deferral. FND-002
is the untested composite path behind SR-17011 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Four criteria in QSL-985's work list stay untagged at this head: FR-139-AC-6, FR-136-AC-5, FR-140-AC-14 and FR-140-AC-11. Each is deferred to QSL-1056, whose title records why (S3 versus admission refusal, the S4 emission dependency, an unreachable fault path, missing `Some`/`none` constructors). The PR body still says "Remaining work: QSL-985", which is the ticket this PR closes, not QSL-1056. Fix: point the PR body's remaining-work line at QSL-1056, and have the lead confirm the deferral. | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md:120; spec/functional/FR-136-check-a-refinement-s-step-rows.md:272; spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:213; spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:216 |
| FND-002 | low | FR-135's nested-reference rule names record and union nesting, but no test covers either, and the composite arms of `holds_reference` are unreachable for a well-typed row (SR-17011 FND-001). The FR-135-AC-6 test covers sequence and option only. Fix: with the SR-17011 FND-001 fix, add a record- or union-nested row to `a_row_nesting_a_reference_other_than_a_whole_population_is_unsupported` (it carries TC-540; it needs no new AC tag unless the spec adds one). | qsl-semantics/tests/it/refinement_declarations.rs:950-994; qsl-semantics/src/check/refinement.rs:1044-1062 |

## Coverage

- Criteria examined: 8. Tagged and complete: 4 (FR-135-AC-6, FR-138-AC-9,
  FR-139-AC-7, FR-140-AC-15). Untagged and deferred to QSL-1056: 4
  (FR-139-AC-6, FR-136-AC-5, FR-140-AC-14, FR-140-AC-11).
- No AC id appears in a doc comment for any untagged criterion.
- Mutations: 4 coder mutations re-run by the reviewer, all killed. 2 reviewer
  probes (see SR-17011).
- Semantic review: the clause-by-clause test-to-AC check is in the Summary
  above.
- Plan completion: not assessed

## Dispositions

Round 1. Reviewed at `c2abdd278b12e1156321da44d0a0f3ec6759ab51`. `quire matrix --scope . --format json` was rerun. FR-135-AC-6, FR-138-AC-9, FR-139-AC-7 and FR-140-AC-15 are tagged. FR-139-AC-6, FR-136-AC-5, FR-140-AC-14 and FR-140-AC-11 are untagged.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | The four untagged criteria are deferred to QSL-1056, and the PR body now reads "Remaining work: QSL-1056 (FR-139-AC-6, FR-136-AC-5, FR-140-AC-14, FR-140-AC-11)". |
| FND-002 | fixed | - |
