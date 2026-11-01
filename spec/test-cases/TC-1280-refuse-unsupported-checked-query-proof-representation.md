---
id: TC-1280
title: "Refuse unsupported checked query proof representations explicitly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: verifies
---
# TC-1280: Refuse unsupported checked query proof representations explicitly

## Description

Checked-route successor for FR-046-AC-5. The control compiles real native
source through S1-S4, selects a checked state clause, and asks the proof-value
boundary to classify its checked collection query. TC-127 remains the legacy
predicate and query execution procedure; this test neither promotes an
unsupported representation to success nor removes the outstanding proof
representation work.

The executable control is
`qsl-replay/tests/it/query_proof.rs`. The public result is derived entirely
from `CheckedPackage`; no caller-created query inventory, value, result or
charge is accepted.

## Test Procedure

1. Compile a clause containing two distinct checked Map nodes of the same
   collection result type, plus a checked Filter clause of that result type.
2. Select each clause with
   `qsl_replay::spine::query_proof::select_query_proof`. Require
   `Unsupported(ValueRepresentation)` and assert the exact containing checked
   clause `NodeKey`, selected query `NodeId`, query form and result type. For the
   two Map candidates, require the returned node identity to match the exact
   checked node selected by the route, not merely either node's common type.
3. Inspect the result shape and require no successful value, truth, or partial
   evaluation charge to be present. Do not treat this unsupported disposition
   as support for Map/Filter proof transfer.

## Expected Results

The unsupported proof representation remains an explicit typed outcome whose
identity and type come from checked nodes. Other checked representations not
implemented by the proof route remain outstanding under FR-046-AC-5.
