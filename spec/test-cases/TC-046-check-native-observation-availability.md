---
id: TC-046
title: "Check observation and operation value availability"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-046: Check observation and operation value availability

## Description

Integration, priority P1. Verifies FR-016-AC-2. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Use real step pre/post/current source contexts. Check result in post, result in current/pre, pre(self.n) in current/pre/post, and attempts to read an operation result using its underlying IR value name. Exercise captured parameter/reference values and pre(alias). Record the actual native frontend phase where availability is rejected.

## Expected Results

Post result and admitted pre reads retain exact selected operation/observation identities. Unavailable result/pre use and result-name bypass refuse wrong_snapshot. Immutable invocation parameters remain pre-captured; post result remains post-captured. Alias reads are not retagged. Valid model construction is required, and an availability refusal intentionally raised during native linkage is distinguished from failed model setup.

