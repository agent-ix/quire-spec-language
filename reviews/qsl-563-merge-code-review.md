---
id: SR-13730
title: "Merge-delta code review of quire-spec-language PR #1137 (QSL-563): the reconciliation in merge 3ddbcdae4 of origin/main 2f0227a5e"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3ddbcdae4; merge-only delta (git show --cc 3ddbcdae4; git diff d2ca261a0 3ddbcdae4 and git diff 2f0227a5e 3ddbcdae4 restricted to lines in neither parent): qsl-semantics/src/check/{mod,refinement,step_simulation}.rs, qsl-semantics/src/check/refinement/{protocol,step_label}.rs, qsl-semantics/src/check/refinement/protocol/tests.rs, qsl-replay/src/refinement/subject.rs, qsl-replay/src/certificate/closure.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-214
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-13730: Merge-delta code review of quire-spec-language PR #1137 (QSL-563): the reconciliation in merge 3ddbcdae4 of origin/main 2f0227a5e

## Summary

Ticket: QSL-563. This is the code review of the merge commit only, with the rust-review lane folded in. The PR content was reviewed mergeable at d2ca261a0 (SR-13657, SR-13658) and is not re-reviewed. The review covers the lines `git show --cc 3ddbcdae4` shows as differing from both parents: `check/mod.rs` (two-pass refinement checking), `check/refinement.rs` (`check_refinement` result, `CheckedRefinement` fields, the `clause_refused` rule, `Typing`), `refinement/protocol.rs` (node right side on a protocol row), `refinement/step_label.rs` (`LabelStep`), `step_simulation.rs` (`StepRowRefusal` with occurrence-keyed entries), `qsl-replay` `open_systems` and `ClosureRefusal`.

Gates run in a scratch copy of 3ddbcdae4 (`git archive`) with its own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`. I did not run `make ci`.

- `cargo test -p qsl-semantics -p qsl-replay -p qsl-analyze -p qsl-eval -p qsl-package --no-fail-fast`: 4,521 passed, 0 failed.
- `make check-spec-validation`: exit 0 (warnings only, none in FR-136, FR-147 or FR-214).
- `tools/check-index-completeness.sh`: exit 0.
- `rustfmt --check` on the merged files: fails on merge-only lines (FND-001). Both parents are fmt-clean.

origin/main moved one commit since the merge (c16d4be02, QSL-945, replay facade union keys). It touches none of the merged files, and `git merge-tree` of HEAD and origin/main is clean.

What the merge got right, checked line by line against both parents:

- Two-pass checking (`check/mod.rs:1706-1791`, `:2146-2163`). Every refinement gets one index-addressed slot. Pass 1 checks the model-abstract-side refinements before the protocol clauses, pass 2 the protocol-abstract-side refinements after them. One `check_one_refinement!` macro carries the PR's stop handling (resource exhaustion and cancellation return with every pending protocol refinement's refusals). A refinement is checked exactly once: the two loops partition `self.refinements` on `is_protocol_subject`. A pass-2 refinement whose rows select a concrete protocol is pushed to `protocol_refinements` and completed by the PR's completion loop, which runs after pass 2. No refinement is lost or checked twice. `refinement_slots` plus `selected_protocols` keep declaration order in `refinements`.
- `clause_refused` (`refinement.rs:1166`). An abstract protocol clause refused by its own checks gives `Refused` even when the rows select a concrete protocol. This is FR-147's 'When the abstract clause was refused, S3 SHALL report the clause's own refusals and SHALL give no `CheckedRefinement`' (FR-147 lines 88-90), and it drops the pending concrete-clause rows, so no coverage refusal is reported over a refinement that has none to give.
- `StepRowRefusal` (`step_simulation.rs`). It merges main's `StepSimulationEntry` with the PR's `Overflow`/`ProtocolSubject` causes. `operation_rows` runs first, so a protocol subject refuses before any record is built. `StepRowOverflow` has no remaining user.
- `open_systems` (`subject.rs:60-65`). The concrete-protocol refusal comes before the abstract-protocol one. Both are not-built refusals, so the order only picks which reason a combined refinement reports (`ProtocolRefinement`), and nothing downstream depends on it.

The combined case (concrete protocol subject with a protocol abstract side) is the coder's reconciliation. It is specified, not a free design call: FR-214 'Inputs' takes 'for an abstract protocol, the refinement's internal list of abstract steps' (lines 42-44), its Behavior gives `Visible{label}` 'for a row whose target is an abstract operation or node' (lines 61-65) and 'an abstract step SHALL be internal exactly when the refinement's internal list names it' (lines 66-67), and FR-214-AC-4's second sentence is that case. FR-147 lines 104-110 allow a node right side on any step row and type its expressions over the concrete step. FR-136 'Order of the checks' (lines 169-175) orders the abstract header and the concrete coverage after the clauses. The merged code is consistent with all three. A scratch test of the case (7 tests, kept at `/private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/rv1137m/combined_tests.rs`, appended to `protocol/tests.rs`) gives:

- `step M::Fill::A -> node A::Seq::x(self)` checks as `StepTarget::Node`, with `AbstractSide::Protocol{Seq}` and observation `Visible{LabelStep::Node{Seq, x}}`.
- An internal list `["A::Seq::F"]` gives `abstract_internal = [Seq::F]`. The list naming `A::Seq::x` beside the node row refuses `ListEntryConflict` at the row.
- `step M::Fill::Done -> node A::Seq::F` refuses `ProtocolStepApplication` at `node A::Seq::F`.
- `Seq` refused by its own checks (`role r on A::Nope`) reports `Seq`'s refusal alone.
- A missing `join` row refuses `MissingRow`.

Replay refuses the case before it explores anything (`open_systems` returns `ProtocolSubject` first), so the engine half cannot be observed. The S3 half can be observed (a `CheckedRefinement`, its node targets, `abstract_internal` and refusals), and no test of it is committed (FND-002).

Mutation testing of the merge-specific logic: 11 mutants, run against qsl-semantics `check::refinement` (lib) and `--test it refinement` with `--features test-support`, plus qsl-replay and the qsl-analyze `refinement` tests for M7. With the committed tests, 2 are killed and 9 survive. Killed: M1 (swap the pass order, so every refinement is checked before the clauses) and M9 (resolve an abstract side against a refused clause). Survived:

- M2: drop the completion of pass-2 concrete-protocol refinements.
- M3: drop the `clause_refused` arm.
- M4: a node right side on a protocol row is always malformed (the PR's pre-merge behavior).
- M5: `observation` maps `StepTarget::Node` to `Either`.
- M6: `Typing.protocol_subject` is false.
- M7: the two `open_systems` refusals are swapped.
- M8: `Typing.internal` is empty.
- M10: the macro's stop path drops the pending refinements' refusals.
- M11: `Typing.abstract_classes` is `None`.

With the scratch combined tests added, M2, M4, M5, M6, M8 and M11 are killed. M3 is killed by one more scratch test: `Seq` refused and the concrete `join` row missing; without the arm, a `MissingRow` leaks beside the clause's refusal. Three survive:

- M7 cannot be observed, since replay refuses a combined refinement either way.
- M10 is the PR's own stop handling moved into the macro unchanged. It needs a resource-exhaustion fixture, and it is out of this merge's scope.
- M3 stays alive for a model concrete side: the unit is refused by the clause anyway, so the extra `CheckedRefinement` cannot be observed there.

## Verdict

**Not mergeable as it stands.** FND-001 breaks `make ci`'s `cargo fmt --all -- --check` gate. FND-002 leaves a specified, compile-observable path the merge built with no test. Both are small fixes. The reconciliation itself is sound: no refinement is lost or checked twice, the refused-abstract-clause rule matches FR-147, and the combined case behaves as FR-214, FR-147 and FR-136 state.

- **FND-001 (medium).** rustfmt drift on merge-only lines.
- **FND-002 (medium).** The combined case has no committed test.
- **FND-003 (low).** A combined refinement's own refusals are lost when the unit stops before the protocol clauses.
- **FND-004 (low).** The same malformed node right side refuses at two different spans.
- **FND-005 (low).** Doc comments left stale by `LabelStep::Node`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Merge-only lines are not rustfmt-formatted, so `cargo fmt --all -- --check` (Makefile:47, `make ci`) fails. The `pub use refinement::{...}` list has a 108-column line (`CheckedRefinement, CheckedRow, ..., LabelStep, ListDefect, ListDefectKind,`), the `refinement_slots` initializer is wrapped differently from rustfmt's output, and the test import adds `LabelStep` to a line over 100 columns. Both parents (d2ca261a0, 2f0227a5e) are fmt-clean. Fix: `cargo fmt -p qsl-semantics`. | qsl-semantics/src/check/mod.rs:161, qsl-semantics/src/check/mod.rs:1706-1709, qsl-semantics/src/check/refinement/protocol/tests.rs:19 |
| FND-002 | medium | The combined case (rows that select a concrete protocol, abstract side `abstract A::<protocol>`) is reachable in the default build (`RefinementSupport::BUILD` holds both FR-147 features) and gives an S3-observable result. The merge built it, and no committed test runs it. FR-214 specifies it (Inputs lines 42-44, Behavior lines 61-67, FR-214-AC-4's second sentence), so it is not unspecified. Untested merge-only code: the node right side on a protocol row (`protocol.rs:821-866`), `Typing.{protocol_subject, abstract_classes, internal}` and the pass-2 push into the completion loop (`mod.rs:2158-2162`), and `observation`'s `StepTarget::Node` arm (`step_label.rs:70-83`), which no test reaches at all. Mutants M2 to M6, M8 and M11 on these lines survive the committed tests (see Summary). Replay refuses the case (`ProtocolSubject` first), but the compile output does not, so a test is needed. Fix: add a test such as `a_concrete_protocol_refines_an_abstract_protocol` to `protocol/tests.rs`, with an abstract protocol `Seq` over `A::Acc` (attempt `x` of `A::Acc::add`, `finish F`) and the refinement `abstract A::Seq concrete M` with the rows of FR-214-AC-1, the `step` row replaced by `step M::Fill::A -> node A::Seq::x(self)` and `undo -> stutter`. Assert: (a) it checks with `AbstractSide::Protocol{Seq}`, `StepTarget::Node{Seq, x}` and observation `Visible{LabelStep::Node{Seq, x}}`; (b) with `refinement_internals` `[{"refinement": ..., "internal": ["A::Seq::F"]}]`, `abstract_internal == [Seq::F]` (FR-214-AC-4); (c) the list naming `A::Seq::x` refuses `ListEntryConflict` at the row; (d) `step M::Fill::Done -> node A::Seq::F` refuses `ProtocolStepApplication` at `node A::Seq::F`; (e) with `role r on A::Nope`, `Seq`'s refusals alone and no `MissingRow`; (f) with the `join` row removed, `MissingRow`; (g) with `role r on A::Nope` and the `join` row removed, no `MissingRow` (kills M3). These pass at 3ddbcdae4 in the scratch files `combined_tests.rs` and `combined_m3.rs` beside the file named in the Summary, and together they kill M2 to M6, M8 and M11. | qsl-semantics/src/check/refinement/protocol.rs:821-866, qsl-semantics/src/check/mod.rs:2158-2162, qsl-semantics/src/check/refinement/step_label.rs:70-83, qsl-semantics/src/check/refinement.rs:1179-1186 |
| FND-003 | low | A combined refinement's own refusals are dropped when the unit stops at the refusal gate before the protocol clauses (`mod.rs:1808`). The PR's FND-001 fix keeps a concrete protocol subject's refusals through `deferred`, but a combined refinement is checked only in pass 2, so it has nothing deferred. Reproduced at 3ddbcdae4: the combined refinement with `step M::Node::undo -> A::Acc::add(self, 1)` beside `OverModel` (a model subject missing `undo`) reports only `OverModel`'s `missing-name`, not the combined refinement's `ill_typed`/`operator-ineligible`. FR-136 'Order of the checks' asks for the header, population, object, fairness and history rows to be checked 'wherever the refinement is reached', with the abstract protocol header 'the one header check that needs a clause' (lines 164-168), and FR-136-AC-8 says 'a refusal of a row not drawn from the clause is still reported'. main has the same behavior for a model-concrete refinement over an abstract protocol (reproduced too), so this comes from main's whole-refinement deferral, which the merge extends to concrete protocol subjects. The unit is refused either way. Only one diagnostic is lost. Fix: a follow-up that checks the model-only parts of a protocol-abstract-side refinement in pass 1 and resolves only its header in pass 2, for both subject kinds. Or record it as remaining work on QSL-563. | qsl-semantics/src/check/mod.rs:1787-1810, qsl-semantics/src/check/mod.rs:2158-2162 |
| FND-004 | low | One refusal at two spans. A node right side under a model subject refuses `invalid_model_binding`/`malformed-declaration` at the whole row on a protocol row (`self.findings.malformed(&node.name, row.span)`, kept from the PR), but at the node name on an operation row (`node_target`'s `self.findings.malformed(&node.name, node.span)`, from main). Reproduced: `step M::Fill::Done -> node A::Seq::F;` refuses at the row text, and `step M::Node::undo -> node A::Seq::F(self);` at `A::Seq::F`. Before the merge each side was self-consistent. Fix: use `node.span` in `protocol.rs` (and update `a_protocol_row_refuses_a_node_target_and_an_application_without_an_operation`), or route the model-subject check through one helper. | qsl-semantics/src/check/refinement/protocol.rs:832, qsl-semantics/src/check/refinement.rs:2141 |
| FND-005 | low | Doc comments left stale by `LabelStep::Node`. `StepLabel`'s doc says the label is 'the abstract operation and the binder record' (step_label.rs:27). `RowObservation::Visible` says 'A row to an abstract operation' (step_label.rs:43). `a_row_target_gives_its_observation` says 'the abstract-protocol half (the internal list, FR-147) is not built' (protocol/tests.rs:758-759), which is no longer true after the merge. Fix: say 'an abstract operation or node', and drop the not-built clause, or point it at FND-002's test. | qsl-semantics/src/check/refinement/step_label.rs:27, qsl-semantics/src/check/refinement/step_label.rs:43, qsl-semantics/src/check/refinement/protocol/tests.rs:758-759 |
