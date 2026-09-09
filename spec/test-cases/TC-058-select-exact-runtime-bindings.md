---
id: TC-058
title: "Select exact runtime bindings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-058: Select exact runtime bindings

## Description

Integration, priority P1. Verifies FR-007-AC-3, FR-007-AC-6. Qualified at 45ed1b4 by SR-097 through the public runtime validation tests. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Use a successfully checked source with two authored clauses and exact native model artifacts. Alter selected RequirementRef/ClauseId, model owner/digest, operation context/name/anchor, snapshot digest/labels and observation role one at a time. Add duplicate/conflicting artifact inventory entries, foreign model bindings, unselected inventory and same labels with snapshot/invocation kinds.

## Expected Results

Valid selection retains exact bindings. Foreign model/clause bindings receive invalid_model_binding, stale bytes receive stale_dependency, inventory conflicts receive invalid_runtime_input and invocation/role mismatches receive wrong_snapshot. No first-match or latest-revision selection and no predicate evaluation occurs.
