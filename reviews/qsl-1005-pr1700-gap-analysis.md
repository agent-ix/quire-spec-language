---
id: SR-1700002
title: "gap analysis of PR #1700 (QSL-1005c) against FR-430-AC-8, FR-244-AC-5 (TC-699), FR-245-AC-8 (TC-700), TC-1170 steps 6-8"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1700 git diff origin/qsl-1005b-timed-refinement...2fd0cedfb; FR-430-AC-8, FR-244-AC-5, FR-245-AC-8; TC-699, TC-700, TC-1170"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
---
# SR-1700002: gap analysis of PR #1700 (QSL-1005c)

## Summary

Ticket: QSL-1005. Head reviewed: 2fd0cedfb. Method: each AC checked against its tests in qsl-analyze/tests/it/timed_refinement_certificate.rs, plus forgery probes and mutants. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-430-AC-8 | Met. TReq `Holds` → `Accepted` → `Proved{ZoneCertified, Certified}`. AC-6's `undo` post-state is the `locked` node at the reserved index, entered with `taken = None`, and is `Accepted`. With the mark removed it is `StepFails` at that edge. |
| FR-244-AC-5 | Met, except byte equality, which is checked as value equality. That is a spec question (below). |
| FR-245-AC-8 | Met, except the second meter fixture, `HeavyEnq` (FND-002). All of these are asserted: `Malformed(positions)` and `Malformed(refinement)`; `PostDiffers` for a replaced position and for `taken = Stutter`; `InitialMismatch`; `stale_dependency`; `Stopped(EvaluationMeter)`. AC-7's `Undetermined` variants are `Undecided` in timed_refinement.rs. |

**The PR body's gap (2), byte equality, is a spec question, not a gap to build.** FR-244 says equal runs give byte-equal certificates, and its refinement form says `taken` "is serialized ... so equal runs give byte-equal bytes". Yet no byte encoding of a zone certificate is specified anywhere:
- not in FR-244 or FR-245;
- not in QSpec FR-418 "Zone certificate", which at e3d48d76 lists the members but gives no wire;
- not in ~/dev/quire-specification (bd73a062).

ADR-026 OV-12 leaves "the zone certificate wire" open to QSpec FR-418. Until FR-418 defines the wire, FR-244-AC-3 and AC-5's "byte-equal" cannot be tested as written. Value equality is the right stand-in. This needs an STD ticket on FR-418, or a rewording of the ACs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **No test fails when a divergent node can carry the `locked` mark.** The one rule that makes leaving locked-region steps unjudged sound is that a `locked` node with a time-divergent continuation is `BadState` (`Checker::node`). With that rule switched off for the refinement form, all 32 `timed_` tests still pass. My probe then got a forged certificate `Accepted`: AC-6's certificate with its zones widened, checked against the same model without the `Dead` invariant, whose safety half is `Violated`. Unmutated, it is `BadState` at the locked node. Add that forgery as a test, asserting `BadState` at the entry node. | qsl-replay/src/certificate/zone_certificate.rs:1835-1838; qsl-analyze/tests/it/timed_refinement_certificate.rs |
| FND-002 | low | **Two FR-245 cases have no test.** The rule "an edge leaves the locked region for a positioned node" → `PostDiffers` is untested; no fixture's locked region has an edge. FR-245-AC-8's second meter fixture, FR-141-AC-9's `HeavyEnq` shape, is also not run. | qsl-analyze/tests/it/timed_refinement_certificate.rs |

## Verdict

**Not mergeable until FND-001 is fixed**, together with SR-1700001 FND-001. FND-002 may be deferred with a ticket.
