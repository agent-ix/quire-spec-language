---
id: SR-937
title: "Spec review of QSL-351 (reopened): ADR-013 C-09/O-19/O-24, FR-069, FR-121, TC-177, TC-516"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@65e310bc3990a417ba1117e5e610c34304cd7a65; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-069-implement-typed-proof-result-envelope.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md, spec/test-cases/TC-177-proof-result-category-preserving-map.md, spec/test-cases/TC-516-locate-a-function-call-site.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-351 (reopened, early review, no PR yet). Base review of the spec
diff, with the backend-member question the coder flagged measured against
QSpec and CG. This file replaces the clean SR-937 spec review of PR #551
(reviewed at `cfef8e79`).

Rulings taken as decided (not findings): the `ReplayInconclusiveCause` /
reporting `InconclusiveCause` split, vacuity as `Proved { success_checks: 0 }`,
FR-121's timing key, `Declined { cause, code }`, the typed `RequestIndex`,
and `ManifestDigest` kept for route registry conflict detection.

The backend-member tension, measured:

- ADR-013 O-19 (unchanged text): the `backend` wire form "is one member,
  `backend`: the provider identity string exactly as the FR-331 manifest
  states it", and "Two backend identities are equal iff their strings are
  equal." ADR-029 PL-4 and its follow-up ("one `qsl-foundation` `BackendId`
  replacing `qsl_route::BackendId` and `qsl_replay::identity::Backend`")
  agree.
- QSpec FR-290 (quire-specification `origin/main`, "Candidate set and
  negotiation"): "A candidate is the backend identity of a registered
  backend. Candidates are ordered by backend identity"; two registrations
  conflict when their FR-331 manifests "differ in any member", refused "once
  per distinct manifest". FR-331 `candidates` is a set of identities.
- CG (`quire-contract-codegen` `origin/main`): `ReplayInputs::backend_manifest`
  is built into the replay request's `backend` member today
  (src/replay/function.rs:228, :386-387). CG AD-004 deletes it at step 5
  "with the tool pin QSL-351 drops" (AD-004:540-541, :776-777).
- QSL code: in `qsl-replay`, `Backend.manifest_digest` is domain-checked,
  carried and round-tripped (request.rs:638, witness.rs:942,
  proof_result.rs:400) and compared to nothing; `call_site` fills it with a
  `[0; 32]` placeholder (call_site.rs:659). In `qsl-route`, the registry
  keys its `duplicate-backend` refusals by digest, "once per distinct
  manifest" (lib.rs:479-560), which is a real use.
- The branch's new O-19 paragraph says the replay request and witness carry
  `ManifestDigest` "as the `backend` wire member CG builds". That
  contradicts the two O-19 sentences directly above it: FND-001.

Other checks: FR-069's three new Behavior bullets and AC-1 are single,
testable statements and match the code; TC-177 step 1 counts eleven records
and the test builds eleven; ADR-013:409 now states vacuity as C-09 does; the
O-24 Public type row lists the new types accurately.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new ADR-013 O-19 paragraph says the replay request and witness carry `ManifestDigest` "as the `backend` wire member CG builds". The same section says that member is the identity string alone and that two identities are equal iff their strings are equal; QSpec FR-290 makes a candidate the identity alone; CG AD-004 deletes `backend_manifest` at step 5 on the expectation that QSL-351 drops it. The registry use (one refusal per distinct manifest) justifies the digest inside `qsl-route` only. State that, and drop the digest from `qsl_replay::Backend`, the request and witness wire and `Candidate` | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:754 |
| FND-002 | medium | FR-121-AC-16 (and the FR-121 statement at :180) names `InconclusiveCause::ReplayRefused`, which does not exist. The terminal value is `Inconclusive(ReplayInconclusiveCause::ReplayRefused(code))`; the envelope's reporting cause is `InconclusiveCause::Replay(ReplayInconclusiveCause::ReplayRefused(code))`. ADR-013 C-09 was updated to the split; FR-121 was not | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:207 |
| FND-003 | medium | FR-121's "before any backend run" half says a non-fault `CallSiteRefusal` settles `TerminalValue::Declined` with its code, but `Declined` also requires a `ProofRefusalCause` (`Refused`, `InvalidInput`, `IncompleteInput`), and nothing says which one each `CallSiteRefusal` variant takes. Two implementers would pick differently. State the cause per variant (or one cause for all) | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:174-178 |
| FND-004 | medium | ADR-018 §1, ADR-020 RE-4, ADR-022, ADR-023 HV-4, ADR-025 MV-1 and FR-182 each say "`InconclusiveCause` gains" a cause (`BoundReached`, `MappingUndetermined`, `MatchUndetermined`, `MemoryBoundReached`, ...) that a non-replay run settles as an inconclusive terminal value. After this change `InconclusiveCause` is the envelope's reporting enum and `TerminalValue::Inconclusive` carries only `ReplayInconclusiveCause`, so as written those causes have no terminal-value carrier. ADR-013 C-09 should say which enum later causes extend | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:668 |
| FND-005 | low | QSL FR-331's temporal replay rule still names `InconclusiveCause::ReplayParity`, a path that no longer exists after the split (now `ReplayInconclusiveCause::ReplayParity`, reported as `InconclusiveCause::Replay(..)`) | spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:108 |
| FND-006 | low | TC-516's Scope line still reads "FR-121-AC-1 to FR-121-AC-15" although the branch adds step 16 and its tests.md row for FR-121-AC-16 | spec/test-cases/TC-516-locate-a-function-call-site.md:32 |

## Verdict

Changes requested. The C-09, O-24, FR-069 and TC-177 amendments are
accurate and match the code. The O-19 paragraph must not say the digest is
the `backend` wire member (FND-001). Recommendation on the tension: the
specs are right. Keep `ManifestDigest` inside `qsl-route`, held on
`BackendDescriptor` as the distinct-manifest key for FR-290's "once per
distinct manifest" refusals (the ruling's reason). Delete it from
`qsl_replay::Backend`, the replay request and witness `backend` member and
the proof-result envelope, where it is carried and never compared (a tool
pin, filled with zeros by `call_site`). Make `Candidate` the identity alone,
as FR-290 and FR-331 `candidates` state, with the digest kept on the
`duplicate-backend` refusal record. Land the wire half with CG AD-004
step 5, which already plans to delete `ReplayInputs::backend_manifest`. Fix
the stale `backend{identity, manifest_digest}` doc comments in
`qsl-route/src/lib.rs` and `qsl-replay/src/identity.rs` in the same change.
FR-121 needs its type names corrected (FND-002) and its `declined` cause
stated (FND-003).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | After the rename, ADR-013 C-09 still says the vacuous proof's cause is "`InconclusiveCause::KaniVacuousProof`", carried in the envelope. That variant is now `ReportedInconclusiveCause::KaniVacuousProof`; `InconclusiveCause` has no vacuous member, as the same paragraph says two sentences later | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:668 |

## Dispositions

Disposition pass 1, reviewed at `43a2844c6d8c37d0b6c4eea2d04c5ea4d297234a`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 43a2844c: ADR-013:754 now says the `backend` member is the identity string alone everywhere, `ManifestDigest` stays in `qsl-route` on the descriptor and the refusal only, and `Candidate` and `qsl_replay::Backend` carry no digest; the code matches |
| FND-002 | fixed | 43a2844c: the carried enum is renamed `InconclusiveCause`, so FR-121's and AC-16's `InconclusiveCause::ReplayRefused` now names the real variant |
| FND-003 | fixed | 43a2844c: FR-121 states `ProofRefusalCause::InvalidInput` for every non-fault `CallSiteRefusal` variant, listed by name |
| FND-004 | fixed | 43a2844c: ADR-013 C-09 says `InconclusiveCause` is the one enum `TerminalValue::Inconclusive` carries and the one the ADR-018/020/022/023/025 amendments extend |
| FND-005 | fixed | 43a2844c: with the rename, QSL FR-331:108's `InconclusiveCause::ReplayParity` names the real variant |
| FND-006 | fixed | 43a2844c: TC-516 Scope reads "FR-121-AC-1 to FR-121-AC-17" |
