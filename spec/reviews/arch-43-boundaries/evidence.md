---
id: SR-804
title: "Evidence review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/tests.md, spec/test-cases/, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-804: Evidence review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping` (QSL
`main` at `99e9b6c7`) against QSpec `origin/main` at `4634f5f`. For each
decision (PF-1 to PF-8, RF-1 to RF-7, AR-1 to AR-6, TK-1 to TK-4), the review
checked three things. Does it have a verification method and a named
evidence artifact? Can its oracle be computed from the types the code
returns? Do the TCs it cites as landed exist, and are they recorded as
covered in `spec/tests.md`?

What holds:

- RF-6 is the right shape for a gate oracle. It has a seeded regression,
  literal expected verdicts (ADR-011 §2.3 rule 8), four mutation controls
  and a determinism check.
- The #192 fault-injection seed follows a working precedent. FR-105-AC-6
  injects a fault at the frame node, and its test exists.
- TC-514 and TC-515 are `✅ Passed locally` in `spec/tests.md:266-267`, and
  their `#[trace]` tags are in `qsl-replay/src/spine/clause/tests/`.
- TC-462 is `✅ Covered` (`spec/tests.md:244`).
- The PF-1 identities and the PF-6 staleness rule ("identity equality only")
  are exercised by the landed TC-515 identity-mismatch path
  (`qsl-replay/src/execute/frame.rs:370-395`).

What does not hold:

- Three findings are high. The "landed" TCs are not all landed. RF-4
  classifies a type that the gate never receives. #191 and #192 have no
  procedure that produces their recorded reference outcomes.
- The #198 decisions, the TK tickets and the TK-3 deletion have no
  evidence, or they remove evidence that exists.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The §4 Tests cell cites "TC-462, TC-463, TC-510 to TC-515 (landed)". `spec/tests.md:245` records TC-463 as `🚧 Partial`, with step 1 `#[ignore]`d (`tests/it/config_version_spine.rs:910`, "blocked on IR-370"). IR-370 is Done, but the ignore stays until the IR pin is bumped. `spec/tests.md:262-265` records TC-510 to TC-513 as `🚧 Planned`, although `#[trace]` tags for them exist in `qsl-semantics/src/check/protocol_clause.rs` and `qsl-forms/tests/it/protocol_clause_forms.rs`. So the landed-behaviour claim in Context is backed by a matrix that says otherwise. Fix: say that TC-463 step 1 is still ignored and name the ticket that bumps the IR pin and un-ignores it. Reconcile the TC-510 to TC-513 rows in `spec/tests.md` with the tests that exist before citing them as landed. | ADR-017 §4 Protocol/frame row, Context; `spec/tests.md:245`, `:262-265`; `tests/it/config_version_spine.rs:910` |
| FND-002 | high | RF-4's oracle is written over `Result<Staged<CheckedPackage>, StageFailure>` with a `Fault(InternalFault)` arm. The gate calls spine `compile`, which returns `Result<Compiled, Box<CompileRefusal>>` (`qsl-replay/src/spine.rs:908-915`). `CompileRefusal` has thirteen per-stage variants (`spine.rs:62-143`). Its `code()` returns only the first cause's code (`spine.rs:148-170`). A stage limit and a runtime invariant appear as codes (`runtime_invariant` is the fallback), and `Omitted` maps to `unsupported_projection`. The landed `StageFailure` has no `Fault` variant (`qsl-foundation/src/diagnostic/stage.rs:235-240`). So "every cause's `CatalogCode` is `unsupported_construct`" and the `Limit` and `Fault` rows cannot be computed as RF-4 writes them, and the unsupported/refused boundary that RF-5 turns on has no defined oracle. Fix: restate RF-4 as an exhaustive match over the `CompileRefusal` variants. Read every cause in the `Check`, `Assembly` and `Profile` vectors, not `code()`. Name the codes for each class: `stage_limit_exceeded` → incomplete; `runtime_invariant` and `output_failure` → tool failure; and an explicit ruling for `unsupported_projection`. | ADR-017 §2 RF-4, RF-5; ADR-013 T-4; `qsl-replay/src/spine.rs:62-170`, `:908-915`; `qsl-foundation/src/diagnostic/stage.rs:235-240` |
| FND-003 | high | Nothing produces the recorded reference outcomes. RF-2 and RF-3 say what is stored, but not who records it, with which build, or when. For #191, the prior-edition outcome has to be captured by the build before the edition move, and no record step or command is named. For #192, FR-110 makes `root` the only header-selectable row, and no spine build compiles a case under a parent profile alone (for example `quire.state.core/v1` without `quire.value.complete/v1`). A "parent refused" record is then the author's assertion, not a measured outcome. V1-TOOL-012 is then checked against hand-written expectations. Fix: define the capture procedure: a gate `record` mode, the build it runs on, and a rule that a record changes only with a new reference side. For #192, name the build path that yields a parent-profile outcome. If none exists, state that #192's reference is authored expected data and that V1-TOOL-012's evidence is that corpus plus RF-6. | ADR-017 §2 RF-2, RF-3, RF-7; FR-110; QSpec AD-003 "Accepted profile hierarchy", V1-TOOL-011, V1-TOOL-012 |
| FND-004 | medium | The RF-6 seed and controls have no stated location, and the cases lack an input they need. "The rest of the corpus holds" reads as if the seed sits in the gate corpus. If the #191 seed is in `tests/fixtures/refinement/`, the real gate reports violation permanently. The "one-unit stage limit" control needs a stage-limit field per case, but RF-3's case record has none, although it says each case compiles "under the case's own stage limits". Fix: put the seeds and controls in a test-only corpus that the real gate run does not read. Add the case's `SpineLimits` to the RF-3 case record, with the default stated. | ADR-017 §2 RF-3, RF-6 |
| FND-005 | medium | The #198 decisions have no QSL evidence artifact. §4 maps QSpec FR-353-AC-1 to AC-5 to "#198's exit" without a TC and without citing QSpec TC-268, which verifies all five (QSpec `spec/complete-v1/tests.md:1135-1139`). FR-353-AC-2 (Kani and Verus consume the relation through AD-010 negotiation) is verified in CG, not QSL. The AR-3 and AR-4 refusals and AR-4's "other items continue" have no named test. Fix: name one QSL TC per decision: missing model key, duplicate binding, conflicting binding, malformed `RustPath` segment, an unbound element next to a bound sibling that continues, and rebinding producing a new `package_id`. Cite TC-268, and assign AC-2 to CG#84 and FR-196. | ADR-017 §3 AR-3, AR-4, §4 Abstraction relation row; QSpec FR-353, TC-268 |
| FND-006 | medium | TK-3 deletes evidence that exists. `check::identity::{Frame, FrameSubjects, ResolvedFrameSubjects}` is exercised by `frame_subjects_resolve_only_through_the_recorded_correspondence` (`qsl-semantics/src/check/identity.rs:768-790`). That test is TC-248's evidence for FR-088-AC-2 ("Frame identity's subject sets resolve to DeclarationKey", `spec/tests.md:139`). Deleting the types without moving the test leaves FR-088-AC-2 backed only by `qsl-package/src/checked.rs:534`. Fix: TK-3 re-homes TC-248 onto the resolution of `OperationEffect`/`CheckedOperationFrame` subjects, or amends FR-088-AC-2, in the same PR. | ADR-017 §1 PF-5 G-3, §6 TK-3; FR-088-AC-2; TC-248 |
| FND-007 | medium | TK-1 to TK-4 have no verification method; the only statement is "TK-1 to TK-4 add their own". Each needs a failure oracle. TK-1: an inherited operation selects its declaring frame through the typed key, and a formatted string no longer type-checks. TK-2: a frame packet whose envelope `clause_node` or `occurrence_key` differs from the recompiled frame refuses `revision-mismatch`, naming both. TC-515 and TC-187 also need amending. TK-4: a field the type does not declare is refused with a named catalog code. Today it is an internal fault (`fault("frame-declared-field-missing")`, `qsl-semantics/src/model/observation/frame.rs:101-102`), and G-4 names no code. Fix: add a TC and an expected outcome for each TK in §6, and name TK-4's catalog code and cause. | ADR-017 §1 G-1 to G-4, §4, §6; TC-515; TC-187; `qsl-semantics/src/model/observation/frame.rs:90-105` |
| FND-008 | medium | QSpec's qualification matrix traces V1-TOOL-011 and V1-TOOL-012 to FR-177 with TC-208 and TC-231 (QSpec `spec/complete-v1/tests.md:244-245`). TC-208 is "Protocol refinement relation". RF-1 says FR-177 is a different claim and makes RF-6 the gates' evidence. So the QSpec rows cannot be satisfied by #191 or #192 evidence. Fix: extend Q-3 to re-trace V1-TOOL-011 and V1-TOOL-012 away from FR-177 and TC-208, to the gate TC that #191 and #192 add. | ADR-017 §2 RF-1, §6 Q-3; QSpec TM-009 rows V1-TOOL-011, V1-TOOL-012; QSpec TC-208 |
| FND-009 | low | RF-5 says "The gate's exit code is the FR-301 code of its O-16 category". By the ADR's own convention a bare id is a QSL requirement, and QSL has no FR-301. The exit-code oracle is QSpec FR-301 (`spec/functional/tooling/FR-301-expose-complete-cli.md`). Fix: write "QSpec FR-301". | ADR-017 Status (id convention), §2 RF-5 |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: TC-463 is
marked partial with TK-4 moving the IR pin; TC-510 to TC-513 are cited as
specified only. FND-002: RF-2's classifications are exhaustive matches over
`ClauseDisposition` and `CompileRefusal`. FND-003: #191 compiles both sides
live; #192 waits on Q-5. FND-004: seeds and controls live in a test-only
corpus; each case carries the four limit sets `ClauseRunRequest` takes.
FND-005: AR-7 names one QSL test per decision, cites QSpec TC-268 and gives
AC-2 to CG. FND-006: TK-3 re-homes TC-248. FND-007: §6 has a verification
column per ticket; old TK-4 is dropped (G-4 is not a defect). FND-008: Q-3
re-traces V1-TOOL-011 and V1-TOOL-012 away from QSpec FR-177 and TC-208.
FND-009: "QSpec FR-301".
