---
id: SR-936
title: "Gap analysis of QSL-351 (reopened): TerminalValue::Inconclusive, Declined code, typed RequestIndex"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@65e310bc3990a417ba1117e5e610c34304cd7a65; qsl-replay/src/proof_result.rs, qsl-route/src/request.rs, spec/functional/FR-069-implement-typed-proof-result-envelope.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md, spec/test-cases/TC-177-proof-result-category-preserving-map.md, spec/test-cases/TC-516-locate-a-function-call-site.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-351 (reopened, early review, no PR yet). Manual
AC-to-test-to-code check for the ACs the branch adds or edits. This file
replaces the clean SR-936 gap analysis of PR #551 (reviewed at `cfef8e79`).

Against the reopened ticket's four scope items:

1. `TerminalValue::Inconclusive` with `ReplayParity(DisagreementCause)` and
   `ReplayRefused(Code)`: present (proof_result.rs:111-121, :189).
2. Typed `request_index` in place of the `String` item: present
   (proof_result.rs:266-306, qsl-foundation/src/request_index.rs).
3. FR-121 keyed on timing: amended (FR-121:174-184); see the spec review
   for its wording.
4. ADR-013:409 matches C-09: amended (vacuous `Proved` stays
   `Proved { success_checks: 0 }`).

Traces:

- FR-069-AC-1 to `tc_177_every_fr331_value_maps_to_its_exact_category`
  (`#[trace("TC-177", "FR-069-AC-1")]`). The test builds the eleven records
  TC-177 step 1 lists, each with its own `RequestIndex`, and asserts per
  record the category, the value, the `request_index` and the envelope cause
  against independent literals (not values derived from the code under
  test). Correct binding and a sound oracle.
- FR-121-AC-16 to
  `a_replay_refusal_settles_inconclusive_with_its_code_and_a_fault_failed`
  (`#[trace("TC-516", "FR-121-AC-16")]`). The oracle uses
  `refusal.code()`, the same call the code makes, but then pins it to the
  literal `Code::InvalidRuntimeInput`, so it is not tautological. Both
  fault shapes are asserted to give `Failed`. One clause is unasserted:
  FND-001.
- tests.md TC-516 row gains FR-121-AC-16; TC-177 has no row change needed.
- FR-121's other half (a non-fault `CallSiteRefusal` settles `declined`
  with its code; a `CallSiteRefusal::Fault` settles `failed`) has no AC and
  no test, and no QSL function builds that settlement: FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-121-AC-16 requires category `inconclusive` for the mapped `UnboundParameter` refusal, and TC-516 step 16's expected result repeats it, but the test never calls `category()` on the value `from_replay_refusal` returns | qsl-replay/src/proof_result.rs:564-591 |
| FND-002 | medium | FR-121's amended statement now names two settlements: before any backend run a non-fault `CallSiteRefusal` settles `TerminalValue::Declined` with `CallSiteRefusal::code`, and a `CallSiteRefusal::Fault` settles `Failed`. Neither has an AC, a TC step or a test; only the after-run `ReplayRefusal` half (AC-16) is covered | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:174-184 |

## Verdict

Changes requested, both medium. The four reopened scope items are present
in code and spec, and the TC-177 binding is correct with independent
oracles. AC-16's test misses its category clause, and the `declined` half
of the FR-121 timing rule is untested and unanchored to any AC.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | FR-075 "Reading a `backend` member" says an empty member SHALL refuse, and FR-075-AC-6 / TC-433 step 2 test it. Now that the identity is the whole member, no QSL reader refuses an empty one: `BackendId::new` and `Candidate::new` are infallible, and the replay request (`Backend::new(wire.backend)`) and witness decoders accept `""`. An empty identity registers and routes as a backend that names nothing. The branch rewrote all three readers and marks AC-6 Partial; the refusal belongs in this PR | qsl-route/src/lib.rs:70, qsl-replay/src/request.rs:638, qsl-replay/src/witness.rs:932 |

## Dispositions

Disposition pass 1, reviewed at `43a2844c6d8c37d0b6c4eea2d04c5ea4d297234a`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 43a2844c: the AC-16 test now asserts `from_replay_refusal(&refusal).category() == Category::Inconclusive` |
| FND-002 | fixed | 43a2844c: FR-121-AC-17 and TC-516 step 17 added; `TerminalValue::from_call_site_refusal` built; `a_call_site_refusal_settles_declined_with_its_code_and_a_fault_failed` asserts `Compile` and `UnknownFunction` give `Declined{InvalidInput, code}` with category refusal, and `Fault` gives `Failed` |

Disposition pass 2, reviewed at `42fa27ad39406ee3537949a9322534991f18dce0`
(range `43a2844c..42fa27ad`). Focused run through `locked-build.sh`:
`cargo test -p qsl-route -p qsl-replay --lib -- empty_backend backend_member`:
4 passed, 0 failed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | 42fa27ad: `BackendId::from_wire` refuses an empty identity (`EmptyBackendIdentity`, TC-433 step 2 test traced to FR-075-AC-6); the replay request refuses `EmptyBackendIdentity` (`invalid_identifier`) and the witness decoder refuses `WitnessRefusal::EmptyBackendIdentity`, each tested. `from_wire` has no caller inside QSL; the driver that reads manifests lives in quire-driver and should read the identity through it |

Disposition pass 3, PR agent-ix/quire-spec-language#631, reviewed at
`6a0087d327a03ddeec3b6c815e0ee09562bc4533` (commit `6a0087d32` only; the
branch was rebased onto main `47dd209e7`). No finding was open, so this
round adds no disposition rows. `git range-diff 3dc4f522c..42fa27ad
47dd209e7..2410a6460` shows all four reviewed commits `=` (unchanged by the
rebase) and `2410a6460` touches only `reviews/`. `6a0087d32` adds
`DeclineCode { Qsl(Code) }` (owner ruling with IR/CG) as `Declined`'s code:
FR-121 (statement, AC-17), ADR-013 (O-16 refusal row, O-24 Public type) and
TC-516 step 17 state that a code stays in its issuing registry and IR's
`kani_*` codes are never remapped onto QSL codes; every `Declined`
construction in code and tests uses `DeclineCode::Qsl`, and the facade test
imports `DeclineCode` from the `qsl_replay` root. No new findings. No build
this round (coordinator's `make ci` exit 0 on the PR head).
