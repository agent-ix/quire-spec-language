---
id: Task-033
title: "Deliver state-scalar projection and validated backend inputs"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-034
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-112
    type: verifies
---
## Scope

Compiler-only LC04 bridge from concrete self fields and pre/post observations
to existing primitive IR and validated runtime inputs. Reuse model/checker/
runtime APIs; C retains backend producers. Specify now and run the selected
QUOIN all-set and code/Rust reviews at PR readiness. Task-020 is complete separately.
