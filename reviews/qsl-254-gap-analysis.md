---
id: SR-7523
title: "QSL-254 gap analysis of PR #676"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e5045663914af15985f19a51ab6a859f90606431; PR #676 diff from f6adf1fbccca4b2a8db00ba142300537228bebd7: FR-056, FR-082, FR-093 implementation and test coverage; QSL-59 ticket acceptance"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
# QSL-254 gap analysis of PR #676

## Summary

Ticket: QSL-254. Plan completion: not assessed. This is a change-scoped audit of the four tickets associated with PR #676, not a whole-repository completion claim. I computed the Quoin matrix at the reviewed checkout and inspected the complete source/test diff. The matrix marks FR-056-AC-1, FR-056-AC-5, FR-056-AC-10, FR-082-AC-8 and FR-093-AC-9 as tagged, with no recorded run evidence. It marks FR-208-AC-1, AC-2 and AC-3 untagged. The QSL-59 ticket additionally requires the full TC-235 M01-M22 outcome set; that test set is not present in this diff or the current relevant integration tests.

The primary compiler intake path now retains provenance and the new tests exercise it. However, the benchmark intake helper also reads source records and constructs `DomainPackage` without preserving original declaration metadata. This leaves the full FR-056-AC-1 criterion unmet for that source-intake path. QSL-254 and QSL-324 are addressed in the changed implementation and tagged tests. QSL-134 is only partially covered because of the alternate provenance path. QSL-59 remains incomplete: this PR adds reusable fixture helpers, not the promised M01-M22 vector assertions or the remaining FR-208 coverage.

## Verdict

FAIL for full completion of the four-ticket stack. The PR contains substantial implementations, but QSL-59 acceptance is unmet and one FR-056-AC-1 intake route does not retain originals. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL-59's required TC-235 M01-M22 conformance outcomes and remaining FR-208 coverage are not delivered by this diff. | qsl-semantics/tests/it/model_operations.rs: fixture additions |
| FND-002 | medium | The qsl-bench source intake path built a DomainPackage from read_records without carrying OriginalDeclaration provenance required by FR-056-AC-1. | qsl-bench/src/model.rs:264-273 |

## Coverage and evidence

- `FR-056-AC-1` (examined): primary `qsl-semantics` intake retains meaning/origins and tagged tests cover it; the benchmark intake route in FND-002 does not retain original declaration records.
- `FR-056-AC-5` (examined): relationship export name/span, missing-target refusal and all-or-nothing result are covered by changed intake behavior and tests.
- `FR-056-AC-10` (examined): field presence is taken from the declared presence member and tested independently of `[0,1]` multiplicity.
- `FR-082-AC-8` (examined): field-presence obligation is separated from multiplicity narrowing, with positive and unchanged-presence controls.
- `FR-093-AC-9` (examined): the parameter occurrence maps to its own declaration span; new test checks exact selected source slices and per-read occurrences.
- `FR-208-AC-1`, `FR-208-AC-2`, `FR-208-AC-3` (examined): all are untagged in the computed matrix. `qsl-semantics/tests/it/model_operations.rs` adds shared fixture builders but no M01-M22 case tests. Search of the relevant integration tests found only unrelated TC-235 references in `model_systems.rs` for FR-086-AC-3.
- `QSL-59 ticket acceptance` (examined): requires each TC-235 M01-M22 vector's complete typed outcome, including code/cause/member path and related charges/conflicts/cascades. No such full vector suite is added here, so QSL-59 cannot be marked complete from this PR.
- Reverse gap: `qsl-bench::model::intake` admits an offer, reads records and constructs `DomainPackage` but does not transfer originals; see FND-002.
- Evidence limits: `quoin matrix --repo . --json` and `quire matrix --scope . --format json` were computed; Quoin reports no run evidence for the tagged criteria. No build/test was run by this reviewer.

## Findings detail

### FND-001 — QSL-59 acceptance is incomplete

The ticket requires the full TC-235 M01-M22 population and complete expected outcomes. This diff's `qsl-semantics/tests/it/model_operations.rs` additions are helper constructors/fixture setup; they do not define a test for each M vector or assert the required per-vector result fields. The computed matrix also leaves FR-208-AC-1..3 untagged. Therefore, this stack cannot close QSL-59, even if portions of FR-208 behavior are already implemented elsewhere.

### FND-002 — benchmark intake drops original declaration provenance

This finding was fixed by `bc34c9d8a0f7473e5b995eb7178c0ffebf9adbdb`: `qsl-bench/src/model.rs` now calls `read_domain_package(selection, &document)` after offer admission. The new source-byte test checks retained type/population meaning, field member name, and FCD origin; the refusal test checks that the reader refusal remains typed. I inspected the full two-file fix diff and confirmed the route now uses the same provenance-preserving intake API as the compiler path. No runtime verification was run.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The current PR body says the broader QSL-59 program is incomplete, but does not explicitly name M01-M22 as unmet. QSL-59 remains In Progress and its latest comments do not yet record that exact partial-slice boundary. Keep the full M01-M22 acceptance outstanding and do not call it met until the PR body and per-ticket scope comment explicitly say so. |
| FND-002 | fixed | bc34c9d8a0f7473e5b995eb7178c0ffebf9adbdb |

FND-002 after-excerpt: `read_domain_package(selection, &document).map_err(IntakeFailure::Read)` in `qsl-bench/src/model.rs` replaces the path that built `DomainPackage` directly from `read_records`. The new `source_byte_intake_retains_original_meaning_and_origin` test asserts original meaning, FCD origin, and member name for type, field, and population records; the companion refusal test checks the reader's typed refusal is preserved.

### Round 2

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-59 remains In Progress; PR #676 per-ticket scope comment explicitly says TC-235 M01-M22 outcomes are absent, this is a partial slice, and the ticket must remain open. This is unmet acceptance outside this PR scope; no M01-M22 outcome is treated as met. |
