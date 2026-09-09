---
id: Task-024
title: "Promote the rule-model frontend for standalone callers"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-101
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-102
    type: verifies
---
## Scope

Move the existing fallible, located rule-model lowering into the library and
replace test semantic helpers with thin public-API callers. Add explicit source
profile selection, retained error provenance and caller-lowered limits. Verify
existing model/package/runtime cases and submit a PR; a standalone CLI follows.
