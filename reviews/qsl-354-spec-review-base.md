---
id: SR-932
title: "Base review of PR #550 (driver owns Kani run, replay and terminal record)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6d60826c4fb5e9e1f7acebcaf349965cd4cdf43d; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/functional/FR-069-implement-typed-proof-result-envelope.md"
review_set: subset
---

## Summary

Ticket: QSL-354. Base checklist over the PR diff (`git diff origin/main...HEAD`):
ADR-011 T-13, E9 row, §6.1 driver row, qsl-replay crate row, §7.1, OBS-031,
OBS-036, OBS-040, #215 row; ADR-012 §5.3; ADR-013 O-16, C-09, O-23, O-24
conversions, QC-8, §5, §7, §8 Q209, §9 OBS-022/031/034; ADR-016 §11 PI-4;
FR-069 scope line. IDs are not renumbered; Q209-7 is deleted with no
remaining spec reference. `quire validate` on the five files exits 0. No new
ticket IDs are added in spec text. Code names verified at the reviewed sha:
`TerminalValue::Failed` (qsl-replay/src/proof_result.rs:126),
`ReplayRefusal::Fault` and `ReplayRefusal::Admission` (execute.rs:222-225),
`ReplayRefusal::code` (execute.rs:232), `AdmissionFailure::Fault`,
`WitnessArmResult` (result.rs:155), `DisagreementCause` (result.rs:50),
`InconclusiveCause::KaniVacuousProof` (proof_result.rs:94).
`TerminalValue::Inconclusive`, `InconclusiveCause::ReplayParity` and
`ReplayRefused` do not exist yet and are stated as targets ("gains").

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-013 says `InconclusiveCause` gains `ReplayParity` and `ReplayRefused` "beside `KaniVacuousProof`", and `TerminalValue` gains `Inconclusive(InconclusiveCause)`. That makes `TerminalValue::Inconclusive(InconclusiveCause::KaniVacuousProof)` a second representation of a vacuous proof, while C-09 says a vacuous proof is `Proved { success_checks: 0 }` and "`TerminalValue` has no separate vacuous variant". Fix: say which values C-09 never produces (or give the new variant a cause type without the vacuity case). | ADR-013:629, ADR-013:967 |
| FND-002 | low | C-09's replay input is "the `qsl-replay` `WitnessArmResult`, or the `ReplayRefusal`", but `qsl_replay::replay` returns `ReplayResult { Witness, Input }` (result.rs:325). No row covers an `Input`-arm result for a Kani `Counterexample`, so "Every Kani counterexample therefore settles exactly one terminal record" rests on an unstated premise. Fix: state that a Kani counterexample's packet is `Witness`-sourced (C-10), so its replay returns the `Witness` arm. | ADR-013:629, ADR-011:1401, ADR-013:967 |

## Verdict

The settled decisions are carried faithfully: T-13 gives quire-driver the S6b
run, the E9 replay through `qsl_replay::replay` and the terminal record; CG
keeps C-09 and the O-25 packet; qsl-replay keeps the replay and terminal
types; reproduced → refuted, parity → `replay_parity`, non-fault refusal →
`replay_refused` with `ReplayRefusal::code`, fault → `TerminalValue::Failed`;
the heads/ lane, pin-equality tests and #215 pin duty are gone while
one-revision-per-lock, release pins and `RevisionPin` stay; the crossing
test sits in quire-integration; FR-069 names CG for C-09. The two findings
above are completeness gaps in the new C-09 shape.

## Dispositions

Round 1, reviewed at a00ace78 (fix commit a00ace78 on top of 6d60826c).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a00ace78 |
| FND-002 | fixed | a00ace78 |
