---
id: SR-11706
title: "Spec review of quire-spec-language PR #901: clauses and attempts on a process operation"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@078869f0fd223d9c2e3d1ddebcaa469b3a11ae63; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-105-emit-state-nodes.md, spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md, spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md, spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md, spec/test-cases/TC-462-s4-emits-state-nodes.md, spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md"
review_set: subset
---
# Spec review of quire-spec-language PR #901

## Summary

Ticket: QSL-808. One pass of spec-review over the PR diff, with the
contradiction, completeness, vector-soundness (reachability, loci, check
order) and EARS/AC-consistency checks. Spec-only PR, so no code review. Only
major issues block. Mediums and lows are recorded in the Verdict.

Oracles, read-only on the reviewed head (equal to main outside `spec/`):
qsl-eval `tests/it/tc235_business_meaning_runtime.rs` (M16 process clauses,
state-machine clauses), `tests/it/state_model_family.rs`; qsl-semantics
`tests/it/model_operations.rs` (process assembles as an identified run type),
`tests/it/tc235_process_runs.rs` (M07), `tests/it/state_clauses.rs` (TC-460
rows), `src/check/protocol_clause.rs` tests and role resolution,
`src/check/assemble.rs` (ObjectType/StateMachine/Process declared through
one path), `src/check/lowering/state.rs` `operation_anchor`/`frame_node`,
`src/check/lowering/model.rs` `record_form`/`object_node`; qsl-package
`src/emit/tests.rs` (TC-513); TC-1043 rows I13 to I17 and FR-154-AC-8;
QSpec FR-208 and FR-342 by id (read-only). Open PRs checked for id
collisions.

Examined:
- FR-103 operation bullet (examined)
- FR-104 Resolution bullets, FR-104-AC-10, FR-104-AC-11 (examined)
- FR-105 Behavior anchor bullet, FR-105-AC-8, FR-105 node table (examined)
- FR-114 process bullet, FR-114-AC-5 (examined)
- FR-154 process receiver sentence (examined)
- TC-459 step 7, TC-460 row 13, TC-462 step 5, TC-513 step 5 (examined)
- TC-1043 Scope and rows I13 to I17; FR-154-AC-8 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-105 now contradicts itself. The new Behavior bullet says the anchor's context, and the frame's `semantic_type`, is the declaring type's own node (`model`/`process`, `model`/`state_machine` or `model`/`object_type`), and that an invariant on a process is anchored at the process node. The unchanged normative node table and the bullet under it still say: `operation_anchor` `semantic_type` is "the declaring `model`/`object_type` node" and its `context` binding is `reference(declaring object_type)`; `frame` `semantic_type` is "the declaring `model`/`object_type` node"; and "The anchor is the context's `model`/`object_type` node for an invariant". The "one `operation_anchor` per (declaring object type, operation name)" Behavior bullet has the same wording. Code (`record_form` gives `model`/`process` and `model`/`state_machine`; `object_node` serves both the anchor context and the invariant anchor) and the merged M16 test agree with the new bullet, so the table is the stale side. Fix: change the table rows and the invariant-anchor bullet to "the declaring type's model node (`object_type`, `state_machine` or `process`)". | spec/functional/FR-105-emit-state-nodes.md:66-67, :69-71, :96 |
| FND-002 | medium | FR-104-AC-11 is traced "Test (TC-1043)", but TC-1043's Scope (FR-105-AC-7, FR-154-AC-6 to FR-154-AC-10, FR-115-AC-11) does not list it, so no TC row covers it. It also restates FR-154-AC-8 rows I13, I14 and I17 (intake refusal at the member), which already covers the behaviour. Its "so no clause names it" adds nothing to test: the whole package is refused at intake. Fix: delete FR-104-AC-11 and keep the sentence in FR-104 Resolution pointing at FR-154-AC-8. Or add FR-104-AC-11 to TC-1043's Scope. | spec/functional/FR-104-check-state-clauses.md:279 |

## Verdict

FAIL, one blocking finding (FND-001) and one AC with no covering TC
(FND-002, which the brief counts as major). Both are small text fixes.

The main question: no merged test contradicts the new text.
- QSpec TC-235 M16 (`tc235_m16_process_invariant_and_operation_clauses_evaluate_per_run`)
  checks and evaluates `invariant ... on M::Fulfilment` and `pre`/`post ... on
  M::Fulfilment::restart` per run, roots the violation at the run's object
  identity with no steps, and frames `restart` with an every-object
  `modifies [Fulfilment/attempts]`. This matches FR-104 Resolution, AC-10 and
  TC-459 step 7.
- State-machine clauses (`M::OrderLifecycle`) check and evaluate. This matches
  the new state-machine anchor sentence.
- `process_model_members_assemble_as_an_identified_run_type` (FR-322-AC-25),
  M07 and the TC-1043 I13/I14/I17 rows agree with "receiver-scoped members are
  refused at intake".
- No merged test asserts the old refusal. `state_clauses.rs` TC-460 tests cover
  rows 1 to 12 only, and qsl-package TC-513 tests cover steps 1 to 4 only.
- Code: `assemble.rs` declares ObjectType/StateMachine/Process records through
  one `ObjectTypeDeclaration` path. `lowering/state.rs::operation_anchor`
  binds `context` to `frame_node`'s `object_node(anchor.declaring)`, which
  `record_form` gives as `model`/`process` for a process. So QSL already emits
  `model`/`process` anchors today, which is consistent with the merged tests
  and with the new FR-105 bullet. `protocol_clause.rs` resolves `role R on
  Config::Flow` through the same object-type index, so TC-513 step 5 is
  reachable.
- Vector walks: TC-460 row 13 resolves `Flow` as a context and then refuses
  `missing-name` at `missing`. TC-513 step 5 gives `[Q]` bound, `[Absent]` a
  missing-name refusal at the entry, and `[I]` a wrong-anchor refusal, in the
  same order as the AC-3 rows. TC-462 step 5 has two `anchor` occurrences and
  no anchor when only `I` is present. All are consistent.
- QSpec: FR-208 agrees (process clauses have object-type meaning, `self` is
  the run). QSpec FR-342 check 1 still requires the anchor context to be a
  `model`/`object_type` node. The QSL text does not claim QSpec agrees; it
  states the target design. QSL's own reader does not check the context kind,
  so nothing observable in QSL conflicts. The QSpec widening the PR body lists
  is still needed for a QSpec-conformant reader to accept the package.

Not a #901 defect, but it blocks a clean merge order: open PR #898 ("Refuse
process-named clauses ...") implements the reverted refusal. Its tests are
tagged `FR-104-AC-10`, `FR-114-AC-5` and TC-460 rows 13 to 16, all with the
old refusal meaning. It must drop that half before or with #901. Otherwise
main gets tests whose AC ids now mean the opposite, and the M16 test breaks.

Recorded, not blocking:
- medium: TC-460 row 13's fixture omits the process's required step and
  `go`'s frame. A process with empty `steps` is refused at intake (M07 run 2),
  so as written the row can stop before the clause. Cite "TC-459 step 7's
  package" instead.
- medium: TC-459 step 7's expected result asserts evaluator behaviour (FR-107
  binds `self` to the run and roots the path), but the step only checks. That
  sentence cannot be observed by this TC. Move it, or point at the M16 test.
- low: FR-104 line 235 still names a population domain by "the
  `model`/`object_type` node". For a process population it is the process node.
- low: FR-114's process bullet ("SHALL bind ... as FR-104 gives a clause")
  restates the general rule. It could be one clause in the existing bullet.

Clean: no roadmap language, no ticket ids, no ceremony, and no QSpec text
quoted in the diff. AC ids FR-104-AC-10/11, FR-105-AC-8 and FR-114-AC-5 collide
with no other open spec PR. #674 touches FR-103 ACs only and does not overlap.
