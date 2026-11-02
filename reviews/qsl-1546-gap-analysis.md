---
id: SR-2160002
title: "Gap analysis of the checked-package handoff writer (quire-spec-language PR 2160, QSL-1546)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-517, FR-518, TC-1312, TC-1313 against src/checked_handoff*, tests/it/checked_handoff.rs"
review_set: subset
evaluated_revision: "103461726b52387c4c4922c1b707b1fd3e400997"
---

# Gap analysis, quire-spec-language PR 2160

## Summary

Ticket: QSL-1546. Planless analysis of FR-517-AC-1..10 and FR-518-AC-1..10 (tagged tests under TC-1312, TC-1313)
against the code. Every AC has a tagged test. Running value-asserting tests exist for FR-517-AC-1 (path list differs
from spec, see FND-001), AC-2 (partial), AC-3, AC-4 (extrema half), AC-5, AC-6, AC-7, AC-9 (clock axis), AC-10 and
FR-518-AC-1 to AC-8 and AC-10. No stubs and no unowned production code were found; the module is the owner of
FR-517/FR-518.

Spec and IR gaps (not defects of this PR, filed as findings below only where a PR-side action is needed):
1. IR-1273 (In Progress): the reader does not authenticate source bytes, so `write(..., Standard)` through the real
   `RealEnvironment` always returns `CaseMismatch` for 28m (`source_integrity()` is `None`). The corpus cannot ship
   from the real reader yet; FR-518-AC-1/AC-9 are gated.
2. QSL-1522 (Specced, FR-505): no bounded Rational value type, so FR-517-AC-4's rational half and the recipe's
   `Exact` and `Ratio` scalars are absent.
3. QSL-1512 (PR 2140 open): `qsl_inventory`, so `inventory.json` is not written.
4. The checked package carries no declared clause name (so `clause.name` has no package-side value) and Contract
   IR refuses any other profile identity or revision as `unknown_profile`; the selection axes `clause.name`,
   `definition.identity`, `definition.revision` are therefore unreachable through the reader as written. A spec
   or IR ruling is needed (see SR-2160003 FND-002).

## Verdict

Coverage gaps are tracked except FND-001 (inventory) and FND-003.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-517-AC-1 ("exactly the paths of the Outputs table") has a passing test that asserts a path list without `inventory.json`; the gap is tracked by QSL-1512 in the coder's note only, with no `#[ignore]`d test carrying the full list and no FR note. | tests/it/checked_handoff.rs:217; FR-517 Outputs | correct-requirement-no-evidence |
| FND-002 | low | FR-518-AC-9 is traced only to an ignored test and FR-518-AC-1's "with IR-1273 reader" half only to an ignored test; the running variants use a `Double` that returns the declared 28m refusal itself, so 28m is checked against its own constants. Spec-sanctioned and tracked by IR-1273, recorded so the matrix is not read as executed coverage. | tests/it/checked_handoff.rs:1068,822 | correct-requirement-no-evidence |
| FND-003 | low | Public API paths without any test: `Selection::read` rejecting a wrong format or an unknown field; `Error::Label`, `Error::Dependency`, `Error::Unnameable`, `Error::Encode` and the `ErrorCode` mapping beyond `Exists`, `Compile`, `Reader`, `Io`, `ReadBackDiffers`, `CaseMismatch`. | src/checked_handoff.rs; src/checked_handoff/selection.rs:Selection::read | correct-requirement-no-evidence |


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Same as SR-2160001 FND-002: ignored full-list test gated on QSL-1512. |
| FND-002 | deferred | AC-9 is untagged until IR-1273 lands; the real-reader half of AC-1 stays an ignored test; the 28m double remains a double. Tracked by IR-1273. |
| FND-003 | fixed | `a_sidecar_that_does_not_parse_is_refused` (wrong format, malformed, unknown field) and `bad_requests_get_their_own_error` (UnknownClause, DuplicateClause, Label, Exists); Dependency, Unnameable and Encode remain unreachable from a request. |

## Verification

Read `quire matrix` expectations from the AC tables by hand (tags `#[trace("FR-517-AC-n", "TC-1312")]`); ran the
tests; two mutation runs in SR-2160001 show which behaviours the tests protect.
