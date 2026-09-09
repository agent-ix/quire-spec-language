---
id: TC-039
title: "Round-trip a bounded generated source family"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: verifies
---
# TC-039: Round-trip a bounded generated source family

## Description

Property, P1; verifies FR-014-AC-5 using deterministic bounded generation in Rust.

## Test Procedure

Generate every string of zero through three scalars over {a, CR, LF, é, 😀}.
Enumerate every ordered pair of byte offsets from zero through length plus one.
Use UTF-8 boundary membership and pair ordering as the independent validity
predicate. For valid spans, calculate expected coordinates by scanning the
prefix's Unicode scalars and counting LF; compare forward coordinates and
reverse round-trip. Repeat requests in reverse order. Never derive the oracle
through Source::position, Source::locate or either bridge direction.

## Expected Results

All 156 generated sources are exercised. Every valid span agrees with the
independent oracle and returns the identical span after reverse mapping.
Reversed, split-scalar and out-of-range spans refuse invalid_source_map. Results
are independent of request order. This finite family is bounded property
evidence, not an exhaustive claim over every 1 MiB document or a fuzz campaign.
