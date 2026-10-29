---
id: SR-1050
title: "Spec review of PR #584 (sum types and case, QSL-383)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@91eafc29b2fd21deb0d098def857cb07857c77e4; spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/functional/FR-046, FR-106, FR-315 to FR-324, spec/test-cases/TC-464, TC-815 to TC-834, spec/usecase/US-032, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-383. Review of `git diff origin/main...HEAD` at 91eafc29 (the
head includes the ADR-012 "ledger" rewording) against the wave-B and wave-C
owner rulings and the QSpec counterparts on QSpec branch
`spec/wave-b-q6-sumtypes` at 36fb68cd: FR-440, FR-441, FR-143, FR-144,
FR-146 ("Case result type"), FR-046-AC-7 and FR-208-AC-13.

Lenses applied: integrity (cross-FR and FR-to-ADR consistency), QSpec
consistency, EARS phrasing, AC-to-TC coverage, caps and limits, version
tracking, compat paths, undefined-is-refuted.

QSpec consistency on the four named points:

- Union member key preimage: FR-319 matches QSpec FR-441 exactly
  (`{version: "quire.union-member-node/v1", declaration_node_id, member}`,
  domain `quire.checked-semantic-node/v1`, `VariantId` is the key retyped).
- OrderedSet keeps first-occurrence order: FR-323 and TC-833 match QSpec
  FR-144 (`OrderedSet<T>` row: first-occurrence order plus uniqueness).
- Case refusal order: FR-318 (scrutinee, then arm bodies in source order,
  then exhaustiveness in `duplicate-arm`, `unknown-member`, `arm-arity`,
  `missing-arm` order; unresolved or wrong-arity arm bodies unchecked)
  matches QSpec FR-146 "Case result type" steps 1 to 3 and "Case
  exhaustiveness".
- A precondition ignores the post side: FR-106's new paragraph states it,
  matching QSpec FR-046 line 25 and FR-046-AC-7, but FR-106's numbered
  checks still read the post side (FND-001).

What is right: no depth cap anywhere (FR-316, FR-318, FR-321, FR-322 bound
only by caller-configured ceilings that name themselves; the 1..10,000
sequence cap is removed from FR-046); no pins, digests or ledgers; no compat
path; ticket ids only in References; every new AC has a behaviour TC; the
evaluator propagates an undefined scrutinee as QSpec FR-146 does.
`quire validate` on the 37 changed files exits 0 and
`tools/check-index-completeness.sh` passes.

## Verdict

**NOT MERGEABLE as it stands.** FND-001 is high: FR-106 contradicts itself
on what a precondition selected by `Invocation` reads. FND-002 and FND-003
leave ADR-012, which this PR implements and amends, contradicting FR-323 and
FR-318.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-106's new precondition paragraph says admission reads and checks nothing of an invocation's post snapshot, result or delta, but checks 1, 3, 4, 6 and 10 still read, digest-check, role-check, model-check, walk and result-check the post side for every invocation, so a missing or malformed post snapshot still stops a precondition, contradicting QSpec FR-046-AC-7. | spec/functional/FR-106-admit-snapshots-and-invocations.md:158-164, 175, 210-211, 215, 221, 260 |
| FND-002 | medium | ADR-012 §16.4 "S3 collection element types" row and §16.8 "Adverse, collections" row still say `Set<U>`, `Bag<U>` and `OrderedSet<U>` refuse `ill_typed`/`operator-ineligible` until SC-G5, and §16.10 SC-G5's Blocks cell says the same; FR-323 admits them under QSpec FR-144's union key. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1561, 1785, 1843 |
| FND-003 | medium | ADR-012 §16.5 "Result type" still types `case` by the `if` rule (`Branches::Inferred`) and the arm-body refusal row says "under the `if` rule above"; FR-318 and §16.10 SC-G6 use QSpec FR-146 "Case result type", which differs (expected-type positions such as the other operand of `=`, Integer widening of `Int[..]` arms). | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1604-1614, 1629 |
| FND-004 | low | FR-322 names charge points for "`case` selection", but QSpec FR-146 (and FR-146-AC-13) makes selection and payload binding charge nothing; the FR should state that a `case` charges its scrutinee then only the selected body. | spec/functional/FR-322-evaluate-union-construction-and-case.md:32-33, 66-69, 83 |
| FND-005 | low | FR-324 calls itself "QSL's reading of QSpec AD-015" that QSpec's definition "governs wherever it differs", and states what it does not add; QSpec FR-143 "Value profile" now defines this, so the FR should cite it as the rule (and list it in Dependencies and relationships). | spec/functional/FR-324-admit-unions-under-the-value-profile.md:23-28 |
| FND-006 | low | FR-318-AC-4 omits QSpec FR-146-AC-14's `arm-arity` arm whose body uses an unbound identifier (reports only `arm-arity`); only the `unknown-member` half of "body not checked" is tested. | spec/functional/FR-318-check-case-expressions-and-exhaustiveness.md:121 |
| FND-007 | low | TC-821's file name still says "applies-the-if-rule-to" while its title and FR-318 use QSpec FR-146 "Case result type". | spec/test-cases/TC-821-s3-checks-a-case-types-its-binders-per-arm-and-applies-the-if-rule-to.md |
| FND-008 | low | ADR-012 §16.10 SC-G2 row (rewritten here) and §16.3 SC-R3 say "FR-323 typed values" meaning QSpec FR-323, now ambiguous with this PR's QSL FR-323 (collections). | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1539, 1840 |

## New findings (disposition pass 2)

Delta reviewed: 6643a381..c9c549c2 (1d6c4bde FR-324 positive profile rule; c9c549c2 ADR-012 §16.10 rewritten as the QSpec rules §16 relies on).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | ADR-012 §16.3's closing paragraph says SC-R3's admission refusals, SC-R2, SC-R4 and SC-R5 "can be tested in process before those spellings land", and the §16.4 "S3 lowering" row's dependency cell reads "spelling proposed with SC-G1 to SC-G3 (§16.10)". c9c549c2 rewrote §16.10 to state QSpec FR-440 and FR-441 as the rules §16 relies on, with no proposal, so both still describe a QSpec proposal that has not landed and point at a section that no longer has one. State the dependency as QSpec FR-440/FR-441 and drop "before those spellings land". | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1543-1545, 1563 |
| FND-010 | medium | Pin and version-tracking wording remains in §16. The §16.4 S4 row says a node "the pinned IR cannot decode" is omitted, and §16.9's Catalog bullet compares revision `1-draft.4` with "the revision QSL claims (`1-draft.8`)" and concludes "No catalog re-pin is needed". c9c549c2 removed "the pinned IR" from §16.8's Identity row and §16.10 but left these two. Write the S4 row as "a node whose (tag, form) the IR cannot decode", as §16.10's closing paragraph does, and delete the re-pin comparison (the catalog code exists; state that, if anything). | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1564, 1822-1824 |
| FND-011 | low | The rewritten §16.10 table's "QSpec rule" column restates each QSpec rule (FR-440's tag and form spellings and `member: null`, FR-441's preimage fields and domain, FR-143's `composite.result-retain` charge and order, FR-144's union key composition, FR-146's result-type rule) instead of citing it. The owner rule is that QSL cites QSpec and restates none of it; FR-318 to FR-324 already cite these FRs. Reduce each cell to the citation (QSpec FR and section name). | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1833-1839 |

## New findings (disposition pass 3)

Delta reviewed: c9c549c2..b1c1f749 (b1c1f749 fixes FND-009 to FND-011). QSpec FR-440 and FR-441 are on QSpec main (`spec/objects/interfaces/FR-440-checked-package-union-and-case-nodes.md`, `FR-441-union-member-key.md`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | ADR-012 §16.9's Profiles bullet still says QSL uses the value-profile selections "with the same profile identity and profile version". A header profile is selected by identity only (QSpec shared grammar `profile = 'profile', ident, '=', string, ';'`), so there is no profile version to match, and the phrase is the version wording FND-010 removed elsewhere in §16. End the sentence at "the same profile identity". | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1818-1821 |

## Dispositions

Round 1, reviewed at 6643a381 (fix commit 6643a381).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6643a381 |
| FND-002 | fixed | 6643a381 |
| FND-003 | fixed | 6643a381 |
| FND-004 | fixed | 6643a381 |
| FND-005 | still-open | The hedge is gone and QSpec FR-143 "Value profile" is cited, but the description still says the compiler "SHALL add no profile identity, profile version or per-feature edition gate", which names what is not; QSpec FR-143 and FR-324's own Behavior state the positive rule (the same profile and definition selections as for records and tuples). |
| FND-006 | fixed | 6643a381 |
| FND-007 | fixed | 6643a381 |
| FND-008 | fixed | 6643a381 |

Round 2, reviewed at c9c549c2 (fix commit 1d6c4bde).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 1d6c4bde |

Round 3, reviewed at b1c1f749cb486594edd6ac03ff3e687e1813c758.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-009 | fixed | b1c1f749: §16.3 and the §16.4 S3 lowering row cite QSpec FR-440 and FR-441, with no proposal wording. |
| FND-010 | fixed | b1c1f749: The S4 row says "the IR cannot decode"; the Catalog bullet reads "`unproved-exhaustiveness` is in the catalog". |
| FND-011 | fixed | b1c1f749: §16.10 cites each QSpec requirement by number and title. |
