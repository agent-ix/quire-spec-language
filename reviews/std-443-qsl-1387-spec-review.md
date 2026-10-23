---
id: SR-16308
title: "spec-review of STD-443 value-parity profile_selections ruling (quire-spec-language PR 1387)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9cdc3a8d; spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md; spec/functional/FR-357-replay-scalar-parity-claims.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; spec/test-cases/TC-1252-function-value-parity-obligation-identity.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1252
    type: references
---

# Spec review of STD-443 value-parity profile_selections ruling (quire-spec-language PR 1387)

## Summary

Ticket: STD-443. One spec-review pass over the diff against origin/main (an ancestor of head 9cdc3a8d), read with QSpec main and CG agent-ix/quire-contract-codegen #553 head 3b871fee (read-only).

Examined, clean:

- Ruling (a) is sound. FR-071 decodes semantic profile selections against a closed known set (`qsl-replay/src/request.rs:423` holds only `quire.profile.v1`), so no lock row `{definition, role}` maps to a `ProfileSelection` that decodes. QSpec FR-323 says no replay member is inferred from the package (QC-8); fixing the list empty obeys that, and copying lock rows did not. QSL FR-290's plugin replay context already carries `profile_selections` empty for the same reason. The lock rows are `package_id` preimage members, so a changed row changes `package_id` and refuses `PackageIdMismatch`.
- No capability is removed and nothing is deferred. Every value-parity claim, over any package, can be built. The ticket offered (a) or (b), so not adding an accessor does not narrow its scope.
- `UnusedProfileSelections` (`invalid_runtime_input`/`unknown-member`) follows the FR-098 `UnusedObjectEnvironment` pattern. FR-484 Behavior, FR-357, AC-12, VP-12 and the PR body all state the same order: after the replay-target check and before the recompile. That puts it ahead of `PackageIdMismatch`, `UnknownFunction`, `NotAValueFunction`, `ClaimOccurrence` and `Obligation`. FR-098 has the replay-target refusal come before the recompile, so the order is achievable.
- AC-11 canonical bytes, derived by hand: RFC 8785 sorts `definition` before `role` and `authority` before `identity`, and nothing needs escaping (`/` is not escaped). That gives `{"definition":{"authority":"agent-ix","identity":"quire.temporal.infinite-trace/v1"},"role":"temporal_profile"}` and `{"definition":{"authority":"agent-ix","identity":"quire.protocol.finite-global/v1"},"role":"protocol_profile"}`. These match the AC and VP-11, and they match emit.rs `split_laws` with `CLAUSE_PROFILE_AUTHORITY` "agent-ix". The fixture `UC` follows existing precedent: the temporal clause matches TC-1089, and the protocol `Fill using v over (input: Boolean)` with an empty `run sequence` matches `qsl-replay/tests/it/compile_protocol_profile.rs`.
- AC-9's amendment and the claim-identity paragraph agree with the packet table. CG #553 fills and checks `profile_selections` as empty in `src/replay/function.rs`, which is consistent.
- The ids FR-484-AC-11/AC-12, TC-1252 VP-11/VP-12 and `UnusedProfileSelections` are unique in the tree and appear on no other open QSL PR head. Of the files this PR touches, only ADR-013 is also touched by another open PR (#1384), in O-03, with no textual overlap.

Gate at 9cdc3a8d: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exits 0. Its warnings are all pre-existing, none in touched files.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The amended ADR-013 O-25 says a packet's semantic profile selections are FR-071 closed-set selections of the proving run and that no packet member copies a lock row. ADR-014 §10 scenario 5 still says the temporal counterexample envelope carries "profile selections (including the temporal profile)". A temporal profile identity is outside FR-071's closed set, so it refuses at decode, and the two ADRs now contradict each other. Fix: amend ADR-014 §10 scenario 5 to say the temporal profile is bound through `package_id` (the lock's `temporal_profile` row) and that the packet's `profile_selections` are the proving run's FR-071 selections. | spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:449-452; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:988-994 | wrong-requirement |
| FND-002 | medium | FR-484 and FR-357 place `UnusedProfileSelections` after the replay-target check. AC-12 and VP-12 pin only its order against `Obligation` and `PackageIdMismatch`, which come later. No criterion pins the earlier half: a request whose replay target refuses and that also carries selections must refuse with the replay-target refusal. An implementation that checks the selections first would pass every AC. Fix: add that case to AC-12 and VP-12. | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:264-274,340 | correct-requirement-no-evidence |
| FND-003 | low | AC-12 depends on `quire.profile.v1` being in FR-071's closed known set, but no spec names the members of that set (only `qsl-replay/src/request.rs:423` does). AC-12 and VP-12 also name the decode refusal by its Rust variant, `UnknownSemanticProfile`, and not by the code FR-071-AC-10 specifies, `unknown_profile`/`unsupported-selection`. Fix: name the set member's source, or name the spec code next to the variant. | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:340 | wrong-requirement |

## Verdict

Mergeable after FND-001 and FND-002 are fixed. No finding is high and no finding is a scope deferral. The ruling is sound against FR-071, QSpec FR-323 and FR-290, and the refusal order is stated the same way everywhere. FND-003 is a wording fix that can go in the same round.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
