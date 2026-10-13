---
id: SR-9010
title: "Base spec review — QSL-726 AC-2 TC-987 edit and operation-body spec text (PR #812)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0d399b03502e622104231c200cc2b2b7eb0a4172; spec/test-cases/TC-987-unit-compile-refuses-duplicate-operation-bodies.md (PR edit), read against FR-102, FR-103, FR-302 and QSpec FR-151, FR-429 and the complete-V1 shared grammar"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-987
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
---

## Summary

Ticket: QSL-726, slice QSL-INPUT-AC-2. The PR's only spec edit renames TC-987 step 2's labels `first`/`second` to `firstBody`/`secondBody`. `first` is a base-vocabulary reserved word in QSL S1 and in the QSpec common reserved list, so the original step could not parse. The edit is required and changes no expected result.

Reading the merged FR-102 and FR-302 text against QSpec while checking the PR surfaced three issues in that merged text.

## Verdict

**CONDITIONAL.** The TC-987 edit is clean. The three findings are in merged spec text, not in this PR's diff. They should be fixed in the owning ticket's spec slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The QSpec complete-V1 shared grammar says a package that does not select `quire.model.complete/v1` receives `unsupported_construct` for `operation-body`. QSL FR-102 and FR-302 are silent on that gate, and the implementation accepts `body` under `quire.value.complete/v1` (every new fixture does). Peer model forms carry the gate in QSL (FR-304, FR-308). | spec/functional/FR-102-build-state-clause-forms.md:28-29,76-89 |
| FND-002 | low | FR-102 adds an `OperationBodyForm` Output and a `Body` dispatch Behavior but has no acceptance criterion for the form's retained parts (label, alias, target segments, parameters, body, spans). AC-4 covers only the thin dispatch arm, so the new forms tests have no correct criterion to trace to. | spec/functional/FR-102-build-state-clause-forms.md:57-64,121 |
| FND-003 | low | FR-302 calls QSpec FR-429-AC-6 "the corresponding QSpec diagnostic precedent" for `conflicting_declaration`/`duplicate-declaration`. QSpec FR-429-AC-6 refuses a duplicate terminal with `ambiguous_declaration`/`ambiguous-name`. QSpec FR-151 now mandates the duplicate-body code directly and is the correct citation. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:77-85 |

## Review Notes

- TC-987 traces FR-302-AC-7, and its expected results match the refusal the code emits.
- TC-986 and TC-987 stay 🚧 in spec/tests.md, which is consistent with AC-3 and AC-4 being open.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The rewritten FR-302 note says QSpec FR-151 "does not specify body multiplicity within a unit" and that the rule is "pending a QSpec alignment follow-up tracked as QSL-727". QSpec FR-151 on main now requires at most one `operation-body` per operation identity across all linked source units, which includes within one unit. QSL-727 has landed, so the note is stale and the claim is false. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:88-96 |
| FND-005 | low | The new FR-102-AC-7 names the target `Config::Sub::resize` and its span `Config::Sub::resize`. The test bound to it builds `Config::ConfigVersion::resize` and asserts segments `Config`, `ConfigVersion`, `resize`, so the criterion's literal oracle and the test disagree. | spec/functional/FR-102-build-state-clause-forms.md:130; qsl-forms/tests/it/protocol_clause_forms.rs:1192-1212 |

## Dispositions

Round 1, reviewed at 8aac7cf6ba714f11ee9bc35f2d4f83f807040f78.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-750 (model-profile selection and gating) owns it. The planner accepted the deferral. The FR-102 and FR-302 sentences are accurate: `resolve_profiles` accepts only the root row (`quire.value.complete/v1`) and refuses every other identity `unknown_profile`, so no package can select `quire.model.complete/v1` today, and FR-304-AC-8 and FR-308 likewise have no enforcing code |
| FND-002 | fixed | 8aac7cf6b: FR-102-AC-7 is added, bound to TC-457 and to the spec/tests.md row. The id is unique across remote branches and open PRs |
| FND-003 | fixed | 8aac7cf6b: FR-302 now cites QSpec FR-151 as the source of the code and cause. Rewording the same sentence introduced FND-004 |
