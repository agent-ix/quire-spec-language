---
id: SR-11743
title: "Spec review of PR 904: FR-154 frame decision over decide_frame and persistence-interface intake"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9f24d128f566e9f399c8e9c44ecb00b1c5877736; spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md, spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md, spec/test-cases/TC-1043-receiver-scoped-frame-entries-and-scoped-deletes-in-the-frame-node-and-decision.md, spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md, spec/test-cases/TC-572-receiver-scoped-frame-in-simulation-and-check-frame.md, spec/test-cases/TC-575-footprints-are-enforced-on-every-reduced-step.md, spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md, spec/decisions/ADR-021-state-space-reduction.md, spec/decisions/ADR-027-protocol-transition-system-for-parallel.md"
review_set: subset
---

## Summary

Ticket: QSL-522. PR 904 (spec only) removes the invented FR-120
`StateModel::check_frame` and `StateDelta`, rewrites FR-154 Outputs, Decision,
Arguments and One decision over the real `population::decide_frame`, and adds
persistence-interface intake rows I18 to I22 to TC-1043 and FR-154-AC-8.
Sub-analyses run in one pass: contradiction against merged FRs, TCs, tests and
code; completeness (every AC covered by a TC row); vector soundness (TC-1043
step 3 and step 5 expected outcomes against the code); EARS/AC consistency.

## Verdict

PASS. No major finding. The rewritten FR-154 text matches `decide_frame`,
`FrameDecision`, `FrameObject`, `FrameDelta`, `FrameStop`, `FrameCharge`,
`OperationEffect` (`receiver_modifies`, `deletes_receiver`,
`deletes_arguments`), `enforce_frame` and `observation::frame::enforce`
(`qsl-semantics/src/model/population.rs:1857-2245`,
`qsl-semantics/src/model/observation/frame.rs:250-375`,
`qsl-semantics/src/model/domain_package.rs:454-470`), including finding order
(create/retype, delete, field write, then declared delta), the
no-receiver rule and the meter charges. TC-1043 D1 to D5 expected outcomes are
what `decide_frame` returns given the stated declared lists; the author's
coverage claim (D2, D5 covered by
`a_receiver_scoped_modifies_entry_grants_the_receiver_only` and
`a_scoped_deletes_entry_grants_the_receiver_or_the_argument_only`; D1, D3, D4
have no test combining both scopes) is accurate. Rows I18 to I22 match the
open implementation PR 903: `read_persistence_interface` passes
`OperationReceiver::NoObject`, and the rule refuses `modifies_receiver` before
`deletes_receiver` at the member, before any entry is resolved. X6 and
FR-154-AC-10 agree on the frame-node locus. No merged test contradicts the new
text; no test parses the edited files. `quire validate` over the 13 changed
files at the head passes (warnings only, all module-level), so
`make check-spec-validation` is plausible at the head. No roadmap language, no
new ticket id and no QSpec text quoted in added lines (ticket ids in the edited
ADR-016 rows were already there).

Residue judgement: FR-154-AC-1, the Candidates and Domains bullets, TC-572
step 1 and FR-120 "The frame decision" describe FR-120's `ModelSystem`
expansion, which does not exist yet (ADR-016 G-4 is its open gate). The spec
states target design, the PR does not make that worse, and no merged test
contradicts it. Recorded follow-up, not blocking for this PR.

## Examined Scope

```yaml
scope:
  - {id: FR-154, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "Intake, Decision, Arguments, One decision, Outputs over decide_frame"}
  - {id: FR-154-AC-2, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "decide_frame with effect modifies self.value, receiver c1 ... refused frame_violation/unauthorized-change naming c2 and value"}
  - {id: FR-154-AC-5, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "drop() framed deletes self on receiver c1: decide_frame returns a FrameDelta for a post without c1 (declared deleted c1)"}
  - {id: FR-154-AC-8, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "intake reads the frame members of rows I1 to I22 ... persistence interface Store"}
  - {id: FR-154-AC-9, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "decide_frame over a population holding c1, c2 and c3 ..."}
  - {id: FR-154-AC-10, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "located at the frame node, QSpec FR-342 check 5"}
  - {id: FR-154-AC-1, path: spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, role: examined, excerpt: "increment on receiver c1 has three successors"}
  - {id: FR-120, path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, role: examined, excerpt: "The frame decision. ModelSystem decides whether a candidate stays inside its operation's frame ..."}
  - {id: FR-120-AC-3, path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, role: examined, excerpt: "Each successor's delta is the one the frame decision returns."}
  - {id: FR-141, path: spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md, role: examined, excerpt: "the frame decision (FR-120 The frame decision) over (a, a')"}
  - {id: FR-155, path: spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, role: examined, excerpt: "writes through decide_frame at entry scope"}
  - {id: FR-215, path: spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md, role: examined, excerpt: "the frame decision's delta"}
  - {id: TC-1043, path: spec/test-cases/TC-1043-receiver-scoped-frame-entries-and-scoped-deletes-in-the-frame-node-and-decision.md, role: examined, excerpt: "rows I18 to I22, X6 locus, step 5 D1 to D5"}
  - {id: TC-471, path: spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md, role: examined, excerpt: "equal the delta the frame decision returns"}
  - {id: TC-572, path: spec/test-cases/TC-572-receiver-scoped-frame-in-simulation-and-check-frame.md, role: examined, excerpt: "call decide_frame with receiver c1 on posts without c1, without c2, and without either"}
  - {id: TC-575, path: spec/test-cases/TC-575-footprints-are-enforced-on-every-reduced-step.md, role: examined, excerpt: "write enforcement through decide_frame"}
  - {id: ADR-016, path: spec/decisions/ADR-016-state-model-finite-execution-mapping.md, role: examined, excerpt: "FE-1 One frame decision; G-4"}
  - {id: ADR-020, path: spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md, role: examined, excerpt: "RS-4"}
  - {id: ADR-021, path: spec/decisions/ADR-021-state-space-reduction.md, role: examined, excerpt: "POR-2, DS-1, RU-1"}
  - {id: ADR-027, path: spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, role: examined, excerpt: "TS-3, FT-2"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-154-AC-5 and TC-572 step 3 do not state the declared created/deleted lists for the `deletes Counter` posts; `decide_frame` refuses `population_delta_mismatch` when they differ from the computed sets, so "return a FrameDelta" holds only with the lists TC-1043 step 5 states. | spec/test-cases/TC-572-receiver-scoped-frame-in-simulation-and-check-frame.md:26 |
| FND-002 | medium | ADR-016 FE-1 ("Verified by inspection") and ADR-021 POR-2 say FR-120's expansion calls `decide_frame`; no FR-120 `ModelSystem` code exists (G-4 open). Same residue as FR-154-AC-1, Candidates and TC-572 step 1. | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:296 |
| FND-003 | medium | Open PR 903 also gives record value-type operations `OperationReceiver::NoObject`; FR-154 Intake names only processes and persistence interfaces, so the spec is silent on that refusal. | spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md:151 |
| FND-004 | low | FR-154 Decision says "A `Receiver` entry ... grant nothing"; the `Receiver` entry type no longer exists in the text (a `receiver_modifies` member). | spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md:232 |
| FND-005 | low | FR-154 Decision does not state the within-step order (ascending identity, fields in ascending `DeclarationKey`) that picks the first finding; TC-572 file slug still says check-frame. | spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md:207 |

## Evidence and Limits

Read-only. Code read at the PR head: `population.rs` `decide_frame`,
`delete_granted`, `check_declared_delta`, `enforce_frame`;
`observation/frame.rs` `run`/`enforce`; `domain_package.rs`
`OperationEffect`; tests `model_population.rs:4438-4641`. PR 903 diff read
for intake order and locus. `quire validate --scope .` over the changed files
with quire 0.36.2 (engine 0.50.2): no errors. No build run.
