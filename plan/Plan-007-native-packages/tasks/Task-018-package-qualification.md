---
id: Task-018
title: "Qualify reconstructed package execution and review the delivery"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/IT-007
    type: references
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-089
    type: verifies
---
# Task-018: Qualify reconstructed package execution and review the delivery

## Scope

Qualify every IT-007 step through actual package readback and native runtime
execution, complete the required local gates and reviews, and hand off the
reviewable compiler delivery under LC02. Preserve all broader assignment gates.

## Subtasks

- [x] Write the integration assertions first, including dropping the original package before readback and retaining independent truth/event/cost expectations.
- [x] Execute healthy, violating, refused and incomplete current/pre/post cases, operation capture/frame controls and fresh-budget retries through reconstructed CheckedPackage.
- [x] Reconcile every matrix criterion, all five metrics and all eight integration steps against actual Rust evidence; shared case portions left by earlier tasks must be complete before this task closes.
- [x] Run README's required local tests, selected private audit tests, formatting, strict Clippy, cached minimal build, strict rustdoc and documented CLI/audit commands. Record exact source revisions, commands and terminal results.
- [x] Apply /home/peter/dev/agent-skills/code-review/SKILL.md and /home/peter/dev/agent-skills/rust-review/SKILL.md, including applicable Rust-style/implementation-gap guidance, and resolve findings.
- [x] Run QUOIN gap-analysis on this plan, actual trace attributes and full criterion mapping; retain the owner's declined optional semantic comparison.
- [x] Update the private owning issue and PR with reviewed specification/implementation/evidence pins and accurate remaining work. A ready merge follows actual gates; no hosted run is dispatched.
- [x] Share concrete native payload/canonical fixtures for independent B/C FS05 qualification and retain actual lowering/backend/Quire work in its existing tickets. Posting a handoff does not prove acceptance.

## Deliverables

Actual reconstructed native workflow observations, full local command evidence,
validated code/Rust and gap reviews, reconciled plan/matrix statuses and a
reviewable private PR plus ownership handoff. Merge readiness is separate from
closing full LC02/FS05 or the original assignment.

## Notes

Use imported bare single-line #[trace(...)] attributes for minted TC/FR IDs.
IT procedure labels and NFR metric ordinals are not invented code trace IDs.
Run one Cargo phase at a time with nice 10, -j 1, --locked --offline,
--target-dir target and one test thread; inspect competing builds first.
No additional agents, hosted dispatch, public publication or overlapping
B/C/TL/Filament edits. Review meaning changes through specify/spec-review again.

Qualification: implementation eb97a87; PR code/Rust review SR-112 and final
Plan-007 gap audit SR-113. Broader LC02/FS05 and LC04/05 acceptance remains open.
