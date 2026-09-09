---
id: Plan-006
title: "Native runtime validation and independent reference execution"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-003
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/IT-006
    type: references
---
# Plan-006: Native runtime validation and independent reference execution

## Requirements Summary

This plan executes the reviewed LC03 slice owned by private language issue #4.
The exact specification is 045025f843346001a91919b8d0816a519e2df337. The eight
actual QUOIN reviews are SR-088–095, committed at 1603f97 with the EARS document
format repair and passing validation at f7ed193. No runtime implementation was
written before that gate. Plan-005's checker work is done and remains intact.

- [ ] StR-001: advance trustworthy concrete assessment; full backend/Quire acceptance remains.
- [x] FR-018-AC-1..7: bounded immutable input artifacts with exact byte correspondence.
- [x] FR-007-AC-1..15: complete valid finite snapshots and recorded invocations before evaluation.
- [ ] FR-008-AC-1..20: source-AST truth, captured values, actual events and exact reference work.
- [ ] NFR-006-M-1..17: explicit construction, validation and evaluation ceilings.
- [ ] NFR-003: no Boolean from failed/incomplete runtime stages in this slice.
- [ ] NFR-005: new production and qualification paths remain Rust.
- [ ] IT-006-SC-01..07: actual source/model/input/reference-result pipeline.

The detailed criterion-to-test mapping remains in
[TM-004](../../spec/native-runtime/tests.md). TC-055–057 are qualified in SR-096;
the validation portions of TC-058–066 are qualified at 45ed1b4 by SR-097.
TC-061's evaluation portion and TC-067–077 remain planned.
A native API milestone does not replace IT-002's compiled ConfigVersion and
existing backend, remaining LC02 strict package/projection, FS03 acceptance
or LC05 Quire integration.

## Dependency Graph

- FR-018 → FR-007: structural artifacts are required for model-aware validation.
- Landed FR-015/016 → FR-007: exact admitted model roles and checked clause
  obligations are prerequisites to validating any runtime input.
- FR-007 → FR-008: execution accepts only a constructor-private validated context.
- FR-008 → IT-006 qualification: the complete reference result must exist before
  the integration can establish its observations.

NFR-006 constrains each stage; NFR-003 constrains outcome classification and
NFR-005 all executable work. The DAG is acyclic. Task frontmatter owns the
execution edges; the source-derived model/IR setup is already qualified.
Shared flat value views and stage-specific budget primitives remain internal
compiler modules. No new crate, shared result authority or external reader is
an implementation prerequisite.

The implementation attaches to src/checking.rs and the existing source,
diagnostic and native_model boundaries. It evaluates the retained native AST,
not the proof graph. Public artifact constructors, validation and execution
remain separate phases with different success types.

## Test Plan

| Cases | Component | Entrance and exit criteria |
| --- | --- | --- |
| TC-055–057 | Input construction | Admitted Rust drafts; exact independent bytes/roles, invalid-index paths and bounded work |
| TC-058–066 | Runtime validation | Real checked model and constructed artifacts; exact bindings, all value shapes, closure/frame/capture diagnostics and immutable retries |
| TC-067–076 | Reference execution | Successful checked/validated context; independent truth, graph, sequence, arithmetic, capture, event and cost expectations |
| TC-077 | Complete native API integration | Actual IT-006 setup; all seven integration steps with source/model/input correspondence |

Tests precede implementation and record a meaningful initial missing-API/failing
assertion. A setup failure cannot count as the requested runtime refusal.
Generated properties and independent exact vectors are explicit in the TC
artifacts. NFR boundary evidence includes empty/zero, exact, one-below and hard
clamping, while reporting coupled ceilings honestly. No metric gets a passing
status from a proposed command or a trace attribute alone.

## Remaining Work

### Track A: Critical path, serial

- Task-012 input construction — Medium, estimated one session; exit: exact immutable artifacts and all structural/budget controls pass.
- Task-013 runtime validation — Hard, estimated two to three sessions; gate: known-invalid/unavailable/frame/closure cases retain the specified outcomes before a context is admitted.
- Task-014 reference execution — Hard, estimated two to three sessions; gate: independent graph and exact cost/event controls pass before the result API is qualified.
- Task-015 qualification and handoff — Medium, estimated one session; gate: full local results, actual code/Rust review and plan-gap evidence agree at the PR source revision.

Effort estimates describe relative work, not a delivery-time promise. This
longest serial chain controls completion of this slice. No parallel track is
authorized: all work is Agent A, with shared desktop resources and one writer.

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-012 | A | FR-018, NFR-006, NFR-005 | TC-055–057 | done |
| Task-013 | A | FR-007, NFR-006, NFR-003, NFR-005 | TC-058–066 | done |
| Task-014 | A | FR-008, NFR-006, NFR-003, NFR-005 | TC-067–076 | in_progress |
| Task-015 | A | FR-007, FR-008, FR-018, NFR-006, NFR-003, NFR-005, IT-006 | TC-077 | blocked |

## Coordination Rules

The sole owning issue is agent-ix/quire-spec-language#4 under the existing
compiler epic; these are local decompositions of LC03, not unowned new campaign
tasks. The claim records the branch agent-a/lc03-native-runtime and isolated
formalization-a-language worktree. No change to B/C/TL/Filament ownership occurs.

Use nice -n 10, one Cargo job, one test thread, --locked --offline and explicit
existing target caches. Inspect competing builds before heavy work and await
each actual exit. Do not edit source/tests while a build reads them. Hosted
workflows stay workflow_dispatch-only and are not dispatched.

Changed interfaces/meaning/acceptance reopen QUOIN specify/review. The selected
all-review choice persists; the optional semantic gap comparison remains declined.
The actual local rust-review skill is mandatory in qualification. A ready
private PR can be pushed/reviewed/merged under existing authorization; no public
posting, licensing change or wider issue closure follows from it.
