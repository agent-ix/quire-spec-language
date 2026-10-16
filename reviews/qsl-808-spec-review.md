---
id: SR-11698
title: "Spec review of quire-spec-language PR #894: process-operation anchor refusal and single-valued cardinality kind"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c7ec0af415dd9ceafe4af70d9ffebf4133c5ad59; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-105-emit-state-nodes.md, spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md, spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md, spec/test-cases/TC-1034-navigation-reads-an-extent-with-charges-dangling-first-and-cardinality-refusals.md, spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md, spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md"
review_set: subset
---
# Spec review of quire-spec-language PR #894

## Summary

Tickets: QSL-808, QSL-810. One pass of spec-review with the contradiction,
completeness, vector-soundness and EARS/AC-consistency sub-analyses over the
PR diff (`origin/main...HEAD`). Spec-only PR, so no code review. Only major
issues block; mediums and lows are recorded in the Verdict.

Oracles: QSL on main: FR-056-AC-22/32/33, FR-094 node table, FR-103 (kept
`requires`/`ensures`), FR-107 (receiver path, AC-7, AC-11), TC-1040 steps
6-8, `qsl-eval/tests/it/tc235_business_meaning_runtime.rs`
(`tc235_m15_lifecycle_invariant_keeps_receiver_context`). QSpec on main by
id: FR-152, FR-208 (state-machine context anchors, operation contracts,
AC-10, AC-11), FR-322, FR-340, FR-341, FR-342.

Examined:
- FR-104 Resolution (new process/state-machine bullet), FR-104-AC-10 (examined)
- FR-114 Behavior (new bullet), FR-114-AC-5 (examined)
- FR-103 new bullet, FR-105 anchor bullet, FR-154 receiver rule (examined)
- TC-460 rows 13-15, TC-513 step 5 (examined)
- FR-364 runtime navigation paragraph, FR-364-AC-6 (examined)
- TC-1034 rows 6, 7, 13 (examined)
- FR-107 receiver path, TC-1040 state-machine fixture, FR-103 kept clauses (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-104's new Resolution bullet (and FR-114's) refuses `invalid_model_binding`/`malformed-declaration` at `T` when `M::T` names a state machine, for an invariant as for a `pre`/`post` clause. Merged QSL already checks and evaluates exactly these clauses: FR-107's receiver path and FR-107-AC-7/AC-11 with TC-1040 steps 6-8 use `invariant Consistent ... on M::OrderLifecycle`, `pre PlaceRequires ... on M::OrderLifecycle::place` and `post FlagsPlaced ...` over the state machine `OrderLifecycle`, and `tc235_m15_lifecycle_invariant_keeps_receiver_context` (QSpec TC-235 M15, FR-208 state-machine context anchors) checks four such clauses. With this PR those rows become unreachable: every one refuses at S2. Fix: drop state machines from the refusal (processes only), or amend FR-107/TC-1040 and state how QSpec FR-208's state-machine clause anchors are carried. | spec/functional/FR-104-check-state-clauses.md:77-90; spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md:72-76; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:126-140,240,244; spec/test-cases/TC-1040-a-clause-evaluation-reports-its-receiver-and-read-paths.md:49-66 |
| FND-002 | high | FR-154's new sentence "The receiver is never read as the run elsewhere either: no clause names a process operation (FR-104), so `self` of a state clause is always an object" contradicts FR-103's kept `requires`/`ensures` clauses, which are checked for every domain-package operation with "the operation's receiver (`self`) ... bind as FR-104's state contract states", and QSpec FR-208's Operation contracts, where for a process operation `self` is the run. FR-104's new "The `self` of a state clause is a `Reference` to an object, never a run" is the rule FR-103 points kept clauses at, so a process operation's own `ensures self.x` now has no defined `self`. Fix: limit both sentences to unit state clauses and state how a process operation's kept clause binds `self` (the run), or state that it refuses. | spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md:123-126; spec/functional/FR-104-check-state-clauses.md:86-87; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:151-160 |
| FND-003 | medium | FR-104-AC-10's second sentence ("The package without those clauses holds no anchor, frame or frame record for `go`") has no row that compiles that package. TC-460 row 13 and TC-513 step 5 instead expect "the package holds no anchor ..." on a unit that refuses, where no package exists ("No row yields a checked clause"), so that half of each expected result can never be observed. Fix: add a row that compiles the TC-458 package plus `Flow` with no clause naming `go` and asserts no anchor, frame node or frame record for `go`; drop the package clause from the refusing rows. | spec/functional/FR-104-check-state-clauses.md:269; spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md:62-63; spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md:54-55 |

## Verdict

FAIL, three blocking findings (FND-001 to FND-003).

Traced and correct:
- Refusal order (QSL-808): `T` is resolved against the package's
  declarations before its kind is known, so the kind check sits after the
  missing-name check for `M`/`T` and before `op`; row 15 (`Flow::missing`)
  refusing at `Flow`, not `missing-name`, follows. No existing TC-460 or
  TC-513 row names a process `T`, so the new rows collide with no
  missing-name row.
- Processes: QSpec FR-341/FR-342 require a `model`/`object_type` anchor
  (invariant) or operation-anchor context, and a process is a
  `model`/`process` node (FR-322, FR-340), so refusing unit clauses and
  attempts on a process is consistent with QSpec and with FR-105.
- QSL-807 (#889) consistency: FR-154 refuses receiver-scoped members on a
  process operation at the member, at intake (I1), before any clause is
  checked; the new refusal is at the clause's `T` at S2. Different
  constructs and stages, no collision; TC-460/TC-513 use an empty frame
  for `go`, so intake admits it.
- QSL-810: FR-364's new text agrees with its multi-valued rules, step 4
  (single-valued: no charge after `model.navigate`) and FR-152's result
  types. Rows 6, 7 and 13 charge only `model.navigate`, matching step 4;
  row 13's count 2 against `[1,1]` and `[0,1]` is `above-maximum` in both.
  No `collection.bound` is charged for a single-valued end, as before.
  QSpec FR-152 does not fix the collection type a single-valued refusal
  retains, so QSL's choice contradicts nothing. The merged `single_target`
  using `Set` always is a code change this PR asks for, not a spec defect.
- No ticket ids, roadmap language, QSpec quotation or tracking ceremony
  added in spec text.

Recorded (medium/low, not blocking):
- FR-364-AC-6's new clause "`Bag` for a `unique: false` one ... as for a
  multi-valued end" has no row: no TC-1034 row refuses a non-unique
  multi-valued end, and rows 3 and 7 (min) do not name the kind.
- FR-104 now refuses a process or state machine `T` with
  `malformed-declaration` but keeps a record value type, event type or other
  wrong-meaning `T` at `missing-name`, while FR-056-AC-22/32 and FR-103 give
  a wrong-meaning name `malformed-declaration`.
- FR-104 says the assembler refuses, FR-114 says the checker; same locus.
- FR-154 (merged) still says "QSL has no state-machine operations", which
  TC-1040's `OrderLifecycle::place` contradicts.
