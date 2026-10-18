---
id: SR-12122
title: "Gap analysis of quire-spec-language PR #952: requirement and trace coverage after deleting native lowering"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@283f23cc045b70b1d889972e0f372d333f45630a; PR #952: deleted FR-009, FR-029, FR-033, FR-034, TC-092, TC-093, TC-107, TC-111, TC-112, TC-912, Plan-008, Plan-009 Task-028; edited FR-010 (AC-3, AC-9), FR-019, FR-075, FR-273, IT-002, NFR-003, NFR-004, StR-001, US-004, US-011, spec/spec.md; trace tags in tests/it/compile_command.rs, tests/it/cli.rs and the five deleted test modules"
review_set: subset
---
# Gap analysis of quire-spec-language PR #952

## Summary

Ticket: QSL-833. Planless. Plan completion: not assessed. Read-only; the
computed Test Matrix (`quoin matrix`) did not finish within two minutes on
this repository, so the criterion-to-test check below was done by hand over
the `#[trace]` tags.

- Deleted requirements: FR-009, FR-029, FR-033 and FR-034 specify the native
  `lower` projection export. ADR-011 section 6.2 (`lowering` row, owner
  ruling 2026-09-24) and ADR-012 state that this capability is removed and
  that backends are chosen by `BackendId` over `quire.checked-package/v2`.
  Deleting them follows the target design; no requirement for a capability
  the target design keeps was removed.
- Deleted tests: `lower_command`, `native_lowering`, `integer_lowering`,
  `config_version_refusals` and `state_scalar_lowering` were tagged only with
  the deleted TC/FR ids, except one `integer_lowering` case tagged
  FR-010-AC-9 for the unknown-target exit 21. This PR removes that clause
  from FR-010-AC-9, and the remaining clause is still tagged by
  `tests/it/cli.rs:229` and `:266`.
- FR-010-AC-3 ("An invalid command invocation exits 20") is tagged by
  `tests/it/cli.rs:15`, `:75` and the new
  `lower_is_an_unknown_command_and_the_lowering_module_is_gone`, whose CLI
  half asserts exit 20 and the exact stderr line.
- TC-015 and TC-017 still list FR-010-AC-3 and AC-9; both ACs exist.
- No `relationships` edge in spec or review records targets a deleted
  `quire-spec-language` id; StR-001, US-004, IT-002, NFR-003 and NFR-004
  each keep at least one other live edge.
- No new production code without an owning requirement: the PR only removes
  code.

## Verdict

FIX. Coverage of the surviving criteria is intact. One test still carries
trace tags to criteria this PR deletes (FND-001); the same test also fails at
runtime, recorded as SR-12121 FND-003.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `lower_refuses_a_complete_v1_program_as_unknown_edition` is tagged `#[trace("TC-107", "FR-029-AC-3")]`, and its doc comment cites FR-029. This PR deletes TC-107 and FR-029, so the tag binds a test to criteria that no longer exist and the matrix carries a dangling row. The test should be deleted with them (see SR-12121 FND-003). | tests/it/compile_command.rs:422-436 |
