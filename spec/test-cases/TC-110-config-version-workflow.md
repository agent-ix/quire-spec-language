---
id: TC-110
title: "Execute concrete ConfigVersion cases"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-032
    type: verifies
---
## Description

Integration, P1: generate explicit ConfigVersion model and input artifacts, then
run the actual standalone binary against independent expected judgments.

## Test Procedure

Inspect the admitted model's source-corresponded roles. Execute all named state,
identity/graph, pre/post and adverse cases from native files and actual Markdown.
Check truth only after completed evaluation, original identities and refusal
stages; retry the healthy case after resource exhaustion.

## Expected Results

Parent/graph/identity and update semantics execute on the actual named model.
Malformed or incomplete inputs do not become false. Original source and each
case's immutable runtime references remain observable.
