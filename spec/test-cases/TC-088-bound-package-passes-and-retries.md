---
id: TC-088
title: "Bound package passes and retries"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: verifies
---
# TC-088: Bound package passes and retries

## Description

Property, priority P1. Verifies FR-019-AC-10, FR-020-AC-9, FR-021-AC-6 and NFR-007's five metrics. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Generate package byte/string/member/array/depth boundaries and independently count the contract's charged units. Exercise zero, exact, one-below and hard/elevated limits separately for recognition, decode, derive, canonical, encode and compare. Include escaped delimiters inside strings and retries after refusal/exhaustion/success. Record coupled upstream limits honestly, including passes unreachable because an earlier pass exceeds the same selected limit; private focused controls may isolate those counters but do not qualify a public successful path.

Include nested recognition-only content in an unknown-version object at the
selected depth boundaries; assert that Serde recursion refusal is classified
as resource_exhausted at the contract's ceiling. Count token scratch separately
from package retention, and ensure no raw-value probe bypasses traversal limits.

## Expected Results

Each pass stops before excess retention/traversal/append with distinct actual usage, no package and unchanged source/model bytes. Serde grammar controls depth; delimiter text in strings adds no container. Retries use fresh accounting and retain upstream stage failures.
