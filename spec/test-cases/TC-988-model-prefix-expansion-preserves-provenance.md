---
id: TC-988
title: "Canonical shared-prefix runs expand to exact model provenance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-988: Canonical shared-prefix runs expand to exact model provenance

## Description

Verify FR-081-AC-9 and FR-081-AC-10: compact prefix runs preserve every
logical fact, input path, rule, ordinal, hidden entry and eager identity, and
the strict reader rejects noncanonical or incomplete representations.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_prefix_expansion.rs::canonical_runs_match_independent_expansion`.

1. Construct a chain, a diamond with two distinct paths to one ancestor, and a
   hidden redefinition fixture. Admit each through the real model-normalization
   boundary under the replacement selection.
2. Independently enumerate expected logical facts from the original declaration
   graph and selected rule manifest, in phase/input/rule order. Include duplicate
   fact multiplicity and the exact zero-based decimal ordinal.
3. Expand the retained prefix representation in the test oracle, without
   calling the production expansion routine. Compare every fact, ordered input
   key, rule, path, multiplicity and ordinal with the independent sequence.
4. Submit mutations that change a prefix boundary, overlap, represented count,
   tail key, ordinal, hidden entry or reachable key/block record to the strict
   reader. Separately remove an eager declaration identity and the complete
   universe/view record.

## Expected Results

For each positive fixture, oracle expansion equals the complete logical fact
sequence and all effective entries, including hidden entries, retain their
preimage and eager identity. The diamond's two paths remain distinct. Every
mutation refuses complete-view admission and returns no partial effective view.
