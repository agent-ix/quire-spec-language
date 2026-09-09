---
id: TC-102
title: "Retain rule-model source refusals and limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: verifies
---
## Description

Refuse malformed source and exhausted lowering/admission with exact provenance.

## Test Procedure

Mutate source profile, fields, declaration identities and semantic roles. Lower
source bytes, entry/type-depth limits and native admission ceilings, then retry.
Inspect the retained document and original JSON/IR/native causes.

## Expected Results

No failed read returns a draft; no failed admission returns a model. Limits
remain incomplete, defects remain refusals, and fresh requests are independent.
