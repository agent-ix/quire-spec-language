---
id: SR-15870
title: "spec-review of the state-graph certificate package_id and StaleDependency cause (quire-spec-language PR 1246)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3735549c3; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md; spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md; spec/test-cases/TC-1141-a-stale-package-identity-settles-stale-dependency-naming-both.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1141
    type: references
---

# Spec review of the certificate package_id and StaleDependency (quire-spec-language PR 1246)

## Summary

Ticket: STD-266 (QSL half; unblocks QSL-538 and QSL-891; paired with
quire-specification PR 373, SR-15860, which must merge first). One
spec-review pass over the PR diff at 3735549. Oracles: QSL FR-121, FR-131,
FR-170, FR-289, FR-290, FR-338, FR-339, FR-357, FR-358, ADR-013 O-26 and
ADR-015, and QSpec FR-331 at the #373 head.

Gates: `QSPEC_DIR=<#373 head worktree> make check-spec-validation` exits 0,
with no diagnostic on FR-127, FR-169, FR-282 or TC-1141.
`cargo test -p qsl-package --lib checked_v2` with the same `QSPEC_DIR`
passed 42.

Examined, clean:

- The certificate carries its own `package_id`. `StateGraphCertificate.package_id`
  is written by the engine at Production, and the checker compares it with
  the recompiled `package_id` "before any rule", returning
  `PackageMismatch{certificate, recompiled}` and evaluating no rule.
  FR-282-AC-9 and FR-169-AC-12 test that ordering.
- One cause. The certificate mismatch (FR-169 V-6 row), a witness or trap
  replay `PackageIdMismatch` (FR-169) and a model-check counterexample
  replay `PackageIdMismatch` (FR-127 row, placed before the catch-all
  `ReplayRefused` row) all settle `Inconclusive(StaleDependency{requested,
  recompiled})`. `requested` is the certificate's or the request's
  `package_id`. The `ReplayRefused` row of FR-169 now excludes
  `PackageIdMismatch`.
- The wire spelling `stale-dependency{requested, recompiled}` agrees with
  the QSpec #373 schema arm and FR-331 table.
- FR-127-AC-12, FR-169-AC-12 and FR-282-AC-9 are each testable, and TC-1141
  steps 1 to 4 back them. The edited-target package in step 2 is concrete.
- Ids. FR-127-AC-12 skips AC-11, which open #723 adds (lead default, sound).
  FR-169-AC-12, FR-282-AC-9 and TC-1141 are unique on main and in the other
  44 open PRs. #1216's `StaleDependency` is the existing refusal code, not
  this cause.
- Lead defaults (a) a checker-side rejection, (b) one cause, a new
  `StateGraphCheck` variant rather than a `CertificateRule`, and
  `DependencyIdentityMismatch` kept as `ReplayRefused` are each coherent.
  See FND-002 for the last one.
- No status wording and no compat layer.

Mergeability: GitHub reports UNKNOWN, and origin/main (423ac6a) is an ancestor
of the head. Depends on #373.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [major] FR-290's plugin cause reader is not updated. Step 3 refuses only `replay-parity`, `replay-refused` and `kani_vacuous_proof` as `QslOnlyCause`. Step 4 refuses only spellings outside FR-331's closed set. With #373, `stale-dependency` is in that closed set and is core-only, so `read_plugin_inconclusive_cause` passes it on as a plugin cause. That contradicts QSpec FR-331-AC-35 and BP-26 (a plugin's `stale-dependency` is refused `invalid_result`/`unknown-cause` at `/cause/type`). Add `stale-dependency` to step 3's list and to FR-290-AC-9. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:134-138, :506 | wrong-requirement |
| FND-002 | high | [major] One refusal now settles two causes depending on the path, and the spec does not say so. FR-121 says every non-fault `ReplayRefusal` "settles `TerminalValue::Inconclusive` with `InconclusiveCause::ReplayRefused`" through `from_replay_refusal` (FR-121-AC-16). FR-289-AC-6 and FR-358 use that mapping, and FR-357-AC-11 settles a stale `package_id` as `ReplayRefused(code)`. The new FR-127 and FR-169 rows settle the same `PackageIdMismatch` as `StaleDependency`. So FR-121's statement is now false for those rows, and the scalar-parity and plugin-replay paths contradict QSpec FR-331's sentence that a replay recompiling another package settles `stale-dependency`. The PR body calls this ungrounded. Fix (a planner decision, the same one as SR-15860 FND-001): either `from_replay_refusal` maps `PackageIdMismatch` to `StaleDependency` (FR-121, FR-289, FR-357-AC-11 and FR-358-AC-3 change with it), or FR-121 states the exception and QSpec FR-331 is narrowed to match. | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:132; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:236-240, :266; spec/functional/FR-357-replay-scalar-parity-claims.md:179 | wrong-requirement |
| FND-003 | low | [minor] FR-282 requires every certificate checker to reject a recompiled `package_id` that differs from the certificate's. The added sentence gives a settlement only for the state-graph checker's `PackageMismatch`. QSpec FR-331 says any core certificate check mismatch settles `stale-dependency`, but the FR-338 and FR-339 checkers' mismatch rejection has no stated settlement, and their certificates carry no `package_id` of their own. State which settlement the other checkers' mismatch gets, or defer it to a named ticket. The PR body calls it out of scope but names no ticket. | spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md:62-66 | correct-requirement-no-evidence |

## Verdict

CHANGES REQUESTED. FND-001 and FND-002 block merge. FND-001 is a
mechanical fix in FR-290. FND-002 needs the same planner decision as QSpec
#373's FND-001 (how far `stale-dependency` reaches), applied to both PRs
together. The certificate `package_id`, the checker order, the one-cause
mapping, the ACs, TC-1141 and the gate are clean. Merge after #373.
