---
id: TC-139
title: "Publish and read checked native handoffs"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-051, type: verifies }
---
# TC-139: Publish and read checked native handoffs

## Description

Owner-boundary checks for the implemented FR-051 public APIs.

## Test Procedure

Integration, property and compile-fail tests derive both documents from real
parse/link/check/compiled-protocol-v2 admission, read them against the same and
foreign admitted packages, recompute schema/content digests independently, and
mutate every field and boundary described by FR-051. Exact and one-over limits
cover bytes, depth, strings, populations, expressions and visited fields.

The compile-fail surface attempts public construction and evaluator/callback
injection. Runtime vectors attempt a textual AST, self-asserted trust/total
flags, false/numeric leaves, foreign spans, clock/profile/capture substitutions,
unknown history kinds and noncanonical JSON.

## Expected Results

Only owner-derived canonical bytes bound to the exact independently admitted
package and selection produce a validated view. Every mutation returns the
stable typed invalid/unsupported/resource-incomplete report with no partial
value. The views expose static checked structure only and cannot report truth,
availability, progress, closure or completeness.

## Status

Passing for `quire-spec-language#90`.
