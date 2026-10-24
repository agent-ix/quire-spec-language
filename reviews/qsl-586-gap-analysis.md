---
id: SR-12192
title: "Gap analysis of quire-spec-language PR #1031: FR-250 hybrid solver verdict map against TC-705 and QSpec FR-193"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5395e3fdda8a5e2bed7dfd11421e9114146316df; PR #1031: qsl-replay/src/hybrid.rs and its four TC-705 tests, ProofBasis::BoundedSolver, InconclusiveCause::SolverInconclusive, ModelCheckLimit::{SolverTime,SolverSteps}; against QSL FR-250 (AC-1, AC-2), TC-705, and QSpec FR-193, FR-331"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1031

## Summary

Ticket: QSL-586. This review is planless, so plan completion is not
assessed. It was read-only. `quoin matrix` (quoin 0.28.3) shows FR-250-AC-2
`tagged` with the four `hybrid.rs` tests as binders, and FR-250-AC-1
`untagged`. Neither has run evidence, because nothing was run in this review.
QSpec was read at its current main and is cited by id.

- FR-250-AC-2 is implemented and covered. Four tests are tagged
  `TC-705`/`FR-250-AC-2`, and every binding is correct. Each test asserts a
  clause of AC-2: the δ-complete `Unsat` with horizon 10 and jumps 3 settles
  `Proved{BoundedSolver, Uncertified}`; `DeltaSat` settles `inconclusive`;
  `EnclosureDisjoint` from a flowpipe without outward rounding settles
  `inconclusive`; `Stopped` settles `incomplete`/`LimitReached`; and no result
  settles `refuted`. TC-705 Step 2 is exercised. The grid's partial cell
  assertion is SR-12191 FND-003.
- FR-250 Behavior: the map, the never-`refuted` rule, `uncertified` on every
  `BoundedSolver` proof, and the horizon and jump bound carried on the record
  are all implemented. Soundness is read from the method kind alone.
- QSpec FR-331: `bounded-solver{method, horizon, jumps}` is the proved basis
  and `solver-inconclusive` the inconclusive cause, as the vocabulary table
  lists them. `bounded-solver` carries `uncertified`, as the certification
  table requires.
- FR-250-AC-1 is the declared remainder: classifying clocks against hybrid
  variables, routing through FR-075 negotiation, and `unsupported` with no
  provider. It has no code and no test here, and none is tagged. It needs
  QSL-490 and STD-139. The ticket text "delete `sound: true`" had nothing to
  delete, which was verified on origin/main.
- Production code without an owning requirement: the solver budget
  vocabulary, which is SR-12191 FND-002. Everything else traces to FR-250.

## Verdict

PASS, with no blocking gap. AC-2 is fully tagged and honestly bound. AC-1 is
declared out of scope and must stay open (FND-001). FR-250 also drops QSpec
FR-193's retained-trace evidence (FND-002), which is a spec alignment gap to
resolve with the AC-1 provider contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-250-AC-1 (classify `v` as hybrid and `t` as a clock, route to the fixture provider, and settle `unsupported`/`unsupported-requested-capability` with none) has no implementation and no test, so TC-705 Step 1 is not exercised. This is declared as the remainder. QSL-586 must not close on this PR: the remainder needs its own ticket, blocked by QSL-490 and STD-139. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:89; spec/test-cases/TC-705-hybrid-solver-results-map-to-verdicts.md:21 |
| FND-002 | medium | QSpec FR-193's verdict map and FR-193-AC-2 require an `inconclusive` solver result to retain the reported trace (a δ-sat point, or a flowpipe meeting the unsafe set) as unverified evidence. FR-250 does not mention this, and `SolverOutcome::DeltaSat`/`EnclosureMeetsUnsafe` carry no trace payload, so the evidence is dropped. The fix is to amend FR-250 Inputs and the table to carry the trace, and implement it with the AC-1 provider contract. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:47-50; qsl-replay/src/hybrid.rs:62-76 |
