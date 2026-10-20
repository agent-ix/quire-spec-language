---
id: SR-13592
title: "base review of QSL-545 refinement history spec follow-ups (PR #1112)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@cc890d9998cd4ca09d16522620b1ef03907ba528; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-138-check-and-compute-history-fields.md, spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md, spec/functional/FR-363-type-navigation-over-a-relationship-end.md, spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/test-cases/TC-543-history-fields-check-and-compute-without-changing-the-concrete-model.md, spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-363
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-540
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-543
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-556
    type: reviews
---
# SR-13592: base review of QSL-545 (PR #1112)

## Summary

Ticket: QSL-545. Base checklist over `git diff origin/main...cc890d999`
(8 files, +60/-19). The diff touches FR-120, FR-135, FR-138, FR-149 and
FR-363, and the test cases TC-540, TC-543 and TC-556. The PR changes only
spec files, so code-review and gap-analysis do not apply. Sub-analyses:
integrity (SR-13594) and ears-conformance (SR-13593).

Gates: `make check-spec-validation` exits 0, with no warning on a touched
file (all warnings are elsewhere and were there before this diff).
`tools/check-index-completeness.sh` passes. Tools: quoin 0.28.3.

ID formats and AC numbering: new ids FR-135-AC-9, FR-138-AC-9 and
FR-149-AC-11 follow the series. TC scopes (TC-540 AC-1 to AC-9, TC-543 adds
AC-9, TC-556 AC-1 to AC-11) and the new TC steps line up with them.
Wording: no changelog, status or ticket-owner wording was added. FR-120's
"(FR-149 relies on this label)" is a cross-reference, and TC-540 step 5's
"rewritten when the feature lands" was there before this diff.

Test↔AC bindings checked against origin/main:
`qsl-semantics/tests/it/refinement_declarations.rs:1273` (FR-135-AC-5),
`qsl-replay/tests/it/refinement_history.rs:793` (FR-138-AC-9, decimal, tagged
TC-543 only),
`qsl-analyze/tests/it/refinement_auxiliary.rs:386` (FR-149-AC-11, tagged
TC-556 only), `qsl-replay/tests/it/refinement_history.rs:249` and `:891`
(FR-363-AC-2's exception, tagged FR-138-AC-2).

On the brief's questions. (1) An AC the code cannot yet meet is
acceptable in this repo: AGENTS.md says to "retain unmet acceptance criteria
for the later testing/assurance campaign". It also says to "update existing
artifacts and tickets for ... remaining work". The PR body's "Flags" list
is not a ticket, and the only tracker for the internal list, QSL-887, is
Done. So the remaining work needs a tracker record, but no spec text change
(FND-002). (2) The AC-tag follow-ups for FR-138-AC-9 and FR-149-AC-11 also
need a tracker record. FR-138-AC-9 needs more than a tag, because its enum
and option-of-enum cases have no test at all (FND-001, FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-138-AC-9 states five cases: decimal, enum and `Option` of enum refuse, and `Int[0, 1]`, Boolean and `Option<Int[0, 1]>` check. The only test, `a_history_type_with_no_key_form_is_unsupported`, covers decimal alone, is tagged `#[trace("TC-543")]` with no AC id, and does not assert that Boolean or `Option<Int>` rows check. The enum and option-of-enum cases are untested, so the matrix shows the AC as untraced, and a regression in `unkeyable`'s `Option` recursion would go unseen. This is code-side remaining work, and it must be recorded on a ticket. | spec/functional/FR-138-check-and-compute-history-fields.md:158; qsl-replay/tests/it/refinement_history.rs:790-815 |
| FND-002 | medium | No open ticket tracks FR-135-AC-9 (the `refinement-internal-list` refusal) or FR-135-AC-6 (the `nested-reference-row` refusal). Neither is built: `UnsupportedFeature` holds only `ProtocolAbstractSide` and `HistoryRow` (refinement.rs:89-98), and QSL has no internal-list input. QSL-887 carried the internal-list work and is Done, and the only other mention is the PR body's Flags list. Keeping the ACs is allowed here (AGENTS.md, "retain unmet acceptance criteria"), but the remaining work must be recorded on a ticket, on QSL-545 or a reopened QSL-887. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:236; spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:239 |
| FND-003 | low | FR-149-AC-11's test exists (`a_certificate_position_carries_its_history`) and asserts both halves, but its tag is `#[trace("TC-556")]` with no AC id, so the matrix shows the AC as untraced. Separately, FR-135-AC-5 now requires "no `subject` member", and its test (refinement_declarations.rs:1273) binds `feature, profile, ..` and never checks `subject`. Both are code-side tag and assert follow-ups, and they should be recorded with FND-001. | qsl-analyze/tests/it/refinement_auxiliary.rs:386; qsl-semantics/tests/it/refinement_declarations.rs:1310-1316 |
| FND-004 | low | FR-149-AC-11's tampered half requires only `Rejected`. FR-149's History bullet says the rejection comes from the comparison rule (`InitialMismatch` or `PostDiffers`) and never from any other rule. A checker that rejected the tampered certificate as `Malformed` would pass this AC while breaking the statement. Name the expected rule in the AC. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:242 |
| FND-005 | low | FR-363-AC-2 is verified by "Test (TC-1031)", and TC-1031 has no step for the new history-name exception. The case is tested only under FR-138-AC-2 in TC-543 (refinement_history.rs:249, :891). Add a TC-1031 step, or add the TC-543 test as a second verification for this AC. | spec/functional/FR-363-type-navigation-over-a-relationship-end.md:132 |

## Verdict

Not mergeable as is, together with the integrity review's high finding
(SR-13594 FND-001). This base review has two medium findings and three low
ones. FND-001 and FND-002 need no spec change. They need a tracker record of
the remaining test and build work, which the brief asked about. FND-004
and FND-005 are small AC and TC edits.

Examined and clean: FR-120's incomplete-clause bullet; FR-135-AC-5's
narrowing and the new AC-9's wording; FR-138-AC-9's wording; FR-149's
`Undetermined` row; TC-540 step 5, TC-543 step 7 and TC-556 step 12 against
their ACs; ID formats and the index.
