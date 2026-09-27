---
id: SR-740
title: "PR 490 FR-096 TC-428 TC-500 spec review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@985e0d016fa50ef1da5dadff950941ccc7823e86; FR-096; FR-001; TC-428; TC-500; spec/spec.md; spec/tests.md; FR-093-AC-17; FR-100 (read only); QSpec 84ed5298 native-diagnostics.md and FR-145 (read via git show)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-428
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-500
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#490 at 985e0d01.
Base spec review of the FR-001, FR-096, TC-428, TC-500, spec.md and tests.md edits. Each was checked against QSpec 84ed5298 (catalog 1-draft.8 and FR-145) and against the code at HEAD.

These parts are sound:

- **FR-096 Status (AC-8, AC-13, AC-14):** matches the code. The ten payloads, `code()` and `cause()`, the three division causes, and the seed and addition locations and charges all agree. The empty-sum sentence also matches the code, and FR-100 on main rules the same way.
- **FR-001 Status:** now truthfully says `definition_source.rs` reads `1-draft.8`.
- **TC-428 and TC-500 Status, and their tests.md rows:** consistent with the tagged tests.
- **Protected files:** the diff touches no FR-100..111, TC-450..469 or `.github/workflows` file.
- **Remaining `1-draft.7` mentions:** all are either historical ("added at 1-draft.7": FR-035, FR-062, TC-113, TC-427, FR-096 OQ-1 and Status line 399, ADR-013, ADR-014, the boundedness reviews) or in another lane's files (FR-100:367, FR-110:101, FR-111-AC-7, TC-491). The one exception is FR-093-AC-17 (FND-002).

On the interim state for `sum-out-of-domain`: it is not described honestly, but the error runs the other way from the brief. FR-100 on main (c3632ca3, #488 fee4abfc) already has the row `Undefined::SumOutOfDomain` -> `sum-out-of-domain`, and AC-10 for the empty sum. The code's spelling matches that row. It is FR-096's text that is stale (FND-001).

## Verdict

**Changes requested** (two medium findings, both small text fixes).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-096 says FR-100's kernel undefined-reason table does not list `sum-out-of-domain` ("which does not list it today (planned, QSL-245)" in Behavior, and "has no `sum-out-of-domain` row yet (planned, FR-100's owner)" in the Status text this PR wrote). That is false at HEAD. FR-100:192 on main has the row, and FR-100-AC-10 specifies the empty-sum case. Fix both places to say FR-100 lists it. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:282-285; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:471-473 |
| FND-002 | medium | FR-093-AC-17 still says `qsl_package::diagnostics_catalog` writes `quire.native.diagnostics/v1` "at revision `1-draft.7`". This PR changed `emit.rs:119` to write `1-draft.8`, so the AC now contradicts the code. FR-093 is not in a protected lane. Update the AC's revision (TC-416 compares the accessor to the emitted bytes, not the literal, so no test change is needed). | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:698; qsl-package/src/emit.rs:119 |
| FND-003 | low | The spec.md FR-096 row says "each of the twelve kernel refusals with a record carries its target domain or width". Only the ten value refusals carry a domain or width. `CardinalityOutOfBound` and `ForeignReference` carry their own fields. It should say "ten", as FR-096 Status does. | spec/spec.md:510 |
| FND-004 | low | FR-100 Status is now stale, but this is a routing item, not a fix for this PR. "Remaining work: the code renders the ten kernel refusals ... as a bare `{"kind": "refused"}`, and refuses a `sum` running total ... as `IntegerOutOfDomain`" is no longer true after this PR. FR-100 is agent-a's lane (not edited here), so route it to FR-100's owner. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:374-378 |
