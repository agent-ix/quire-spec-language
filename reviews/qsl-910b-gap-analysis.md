---
id: SR-12244
title: "Gap analysis of quire-spec-language PR #1127 (QSL-910 follow-up) against FR-225, FR-227, FR-229"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@06cd62ca9a993b81c362280e1e3aff886b6410f7; PR #1127 diff origin/main...HEAD; FR-225 (TC-670), FR-227 (TC-672), FR-229 (TC-674), ADR-025 MB-3, MB-4, MS-2, MS-4, MK-1, MK-2, MF-1, ADR-027 PB-2, PB-3, FT-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
---
# Gap analysis of quire-spec-language PR #1127

## Summary

Ticket: QSL-910. Ticket text was read as data. The analysis is planless and
scoped to the PR diff. Plan completion: not assessed.

**Tags.**

- `a_flush_is_a_position_with_a_memory_anchor_and_atoms_read_memory` is
  tagged `TC-674`, `FR-229-AC-2`. Every clause of the AC is asserted:
  - `x.v` is false after `Sx` and true after `flush(left)`;
  - the flush carries the `memory` anchor naming `tso`/`flush`/`left`;
  - the register reads 0 after `Ly`;
  - the guarded load is enabled for `left` after `Sx`, not for `right` until
    the flush, and not before `Sx`, so it is non-vacuous.

  See FND-003 for how the atoms are read.
- `a_tso_counterexample_replays_only_under_tso` keeps only `TC-672`. That is
  honest: FR-227-AC-1's "four positions as in §10's table" waits on the
  SR-12240 FND-008 step-order question.
- `a_refuted_run_that_cut_a_store_states_its_bound_reached` is tagged
  `TC-670`. It backs FR-225's Outputs clause ("whether it was reached"), not
  a numbered AC.
- No tag overclaims.

**Blocked remainders.** Each reason was checked against the code and the spec:

- **Request wire / obligation identity.** True. `ProtocolCheckRequest` has no
  wire or identity digest. The send record is likewise in-memory only.
- **`ra` (QSL-566).** True. `SubjectMemory` has only `Sc` and `Tso`.
- **FR-225-AC-4, first half.** True as written. MS-4 enables `flush(b)`
  whenever a buffer is non-empty, and a store is cut only at a non-empty
  full buffer, so with bound 2 no state has only a cut store enabled. This
  is a spec question.
- **FR-225-AC-4, second half.** True. `ProtocolCheckRequest` carries no
  ADR-021 state constraint.
- **MF-1, FR-228, FR-227-AC-4.** True. `check_protocol` refuses liveness
  clauses (`FormNotDecided`), so no protocol lasso exists.
- **PB-3 uncounted steps.** True. No counted-step interval distance exists
  over protocol subjects.
- **FR-229-AC-4.** The `tso` half is covered by
  `tso_steps_carry_store_buffer_footprints` (TC-666). The `ra` half waits on
  QSL-566.

Nothing unblocked was skipped.

No stub, no unowned code, and no inflated coverage.

## Verdict

Mergeable. Three low findings, none blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The MK-2 case of an `sc` step that carries a memory component (recorded Some, replayed None) is untested. The `sc` replay of the `tso` trace refuses `PostState` at step 0 before the component check runs. The code covers the case through `Option` `!=`, but no test reaches that arm. | qsl-replay/src/model_check/replay.rs:875-881; qsl-analyze/tests/it/weak_memory.rs:1428-1447 |
| FND-002 | low | "Names `SB::Both` resolved to `tso`" is asserted on the request's `ProtocolSubject` (`case.subject(&tso)`), which holds by construction of the selection. It is not asserted on the trace subject that replay reads, so it cannot fail for a counterexample that drops the model. | qsl-analyze/tests/it/weak_memory.rs:1335-1338 |
| FND-003 | low | FR-229-AC-2's `holds(x.v = 1)` and `holds(c1.r = 0)` are checked by reading fields of `atom_state`, not by evaluating a `holds` atom as TC-674 step 2 says. A defect between `atom_state` and the clause evaluator would pass. | qsl-analyze/tests/it/weak_memory.rs:1248-1272 |
