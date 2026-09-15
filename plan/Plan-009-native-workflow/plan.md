---
id: Plan-009
title: "Native source integration and usable workflow"
type: Plan
status: complete
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-022
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-009
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
- [Task-025](tasks/Task-025-standalone-command.md): bounded standalone file
  intake, native execution output and a runnable example while earlier PRs await review.
- [Task-026](tasks/Task-026-compiler-export.md): source-only command exporting the
  actual native package bytes for existing consumers.
- [Task-027](tasks/Task-027-selected-package-run.md): verify and execute selected
  package artifacts with the existing source/model authority and reader.
- [Task-028](tasks/Task-028-lowering-export.md): export the existing Boolean
  projection through the standalone compiler for existing IR/backend consumers.
- [Task-029](tasks/Task-029-quire-consumer.md): join the current real Quire Rust
  extractor to mapped native compilation and runtime, preserving producer metadata.
- [Task-030](tasks/Task-030-extracted-command.md): run selected Markdown clauses
  through the standalone binary while earlier PRs await review.
- [Task-031](tasks/Task-031-config-version.md): execute the concrete ConfigVersion
  parent, graph and update examples with real native and Markdown file requests.
- [Task-035](tasks/Task-035-runtime-owned-wire.md): move native runtime decoding
  constraints into their owning types, following PR #16's review under #27.
- [Task-036](tasks/Task-036-runtime-input-schema.md): publish and exercise the
  native runtime input schema, preserving the exact reader's authority.
- [Task-037](tasks/Task-037-published-v2-handoff-addresses.md): publish the
  producer-owned `/2` handoff member and format addresses required by the
  independent Rust consumer.
- [Task-040](tasks/Task-040-v2-state-evaluation.md): evaluate strictly admitted
  `/2` compensation expressions through the existing bounded state evaluator.
- [Task-041](tasks/Task-041-initialized-capture-evaluation.md): resolve admitted
  compensation captures through their authored initializer and anchor.
- [Task-042](tasks/Task-042-forward-effect-capture-admission.md): admit the
  exact forward-effect inputs required by registration-capture initializers.

Use Rust, serial local Cargo phases and existing caches. No hosted CI or agents.
QUOIN specification and actual code/Rust reviews occur once at PR readiness;
optional semantic review remains declined.

## Completion

All seventeen tasks are done. Task-037 completed issue #78's bounded public
handoff-address correction without reopening LC05 behavior or the v0.2.0
release. Task-040 completes issue #101's exact v2 state-evaluation seam without
adding a second evaluator or changing Protocol ownership. Task-041 tracks issue
#105's initialized-capture correction without reopening that architecture.
Task-042 completes the exact forward-effect admission gap found by the
independent Protocol consumer after #105. The setup effort,
artifact lineage, reproduction path and remaining support gaps are recorded in
`docs/lc05-technical-acceptance.md` at LC05 closeout.
