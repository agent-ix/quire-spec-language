---
id: Plan-009
title: "Native source integration and usable workflow"
type: Plan
status: in_progress
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-022
    type: references
---
## Scope

LC05 compiler-side mapped intake and an executable parent/aggregate workflow.
C owns the Quire producer adapter; A consumes its verified source correspondence.
Subsequent work exposes native results to standalone callers. Full extraction
adoption and assurance remain explicit acceptance work.

## Delivery

- [Task-021](tasks/Task-021-mapped-compiler-intake.md): mapped compiler API and
  actual runtime integration, then a PR ready for review.
- [Task-022](tasks/Task-022-native-execution.md): native execution and retained
  reports while Task-021's PR is reviewed. Standalone file intake remains later work.
- [Task-023](tasks/Task-023-native-input-reader.md): selected snapshot/invocation
  byte intake while earlier PRs await review.
- [Task-024](tasks/Task-024-model-source-frontend.md): production rule-model
  source frontend and thin fixture callers, enabling the standalone CLI.

Use Rust, serial local Cargo phases and existing caches. No hosted CI or agents.
QUOIN specification and actual code/Rust reviews occur once at PR readiness;
optional semantic review remains declined.
