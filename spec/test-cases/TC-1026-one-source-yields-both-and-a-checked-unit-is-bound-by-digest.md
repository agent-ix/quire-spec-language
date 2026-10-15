---
id: TC-1026
title: "One source yields the graph and the type report, and a checked unit is bound by digest"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: verifies
---
# TC-1026: One source yields the graph and the type report, and a checked unit is bound by digest

## Description

Verify FR-362-AC-1 to FR-362-AC-4 and FR-362-AC-7. Catches an implementation that compares only labels, one that reads the source twice with two grammars, and one that accepts a type report built from other bytes.

## Test Procedure

1. Read one source with a predicate and an invariant under one authority and identity. Run the producer. Compare the graph's source reference, the type report unit's reference and the formal source's reference with the `quire.source.bytes/v1` digest of the input bytes.
2. Run `qsl_cst::parse`, `build_unit` and the producer on the same source with its profile line rewritten to carry `version` and `digest` after the identity.
3. Build the graph from the text of step 1 and the type report from the same text with one character of a comment changed, same authority and identity. Call the checked-unit constructor. Repeat with the graph from the changed text and the type report from the original.
4. Build the checked unit from a graph and a type report of the same bytes and take the inventory. Compare its rows with the rows of the producer's unit. Offer a type report that covers two units.
5. Read a unit holding `predicate Positive` and an `invariant` (moved families) and one `temporal` clause (unmoved: S2 has no dispatch entry). Read it again without the `temporal` clause, and once more with a `relation` clause in its place.

## Expected Results

- Step 1: the three references are equal and carry the digest of the input bytes.
- Step 2: `qsl_cst::parse` returns a recovered `ParsedSource` with an `invalid_syntax` diagnostic at `version`; `build_unit` and the producer refuse `invalid_syntax`.
- Step 3: both calls refuse with the cause naming the differing side, the selected reference and the differing reference.
- Step 4: the rows are equal; the two-unit report is refused as not one unit.
- Step 5: the first and third reads are refused `unsupported_construct` at the `temporal` or `relation` clause with no graph and no type report; the second yields both; no second parser runs. When the `temporal` family moves, its read is retired with the family.
- Assertions compare typed causes and references, never message text.
