---
id: SR-15780
title: "spec review of STD-293 FR-106 admits set, bag and ordered-set values (QSL PR #1255)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@79687e8b288fcf785ef37fe0653e2d09bc56c6a6; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md, spec/test-cases/TC-465-admission-refuses-each-input-defect.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1171
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-465
    type: reviews
---
# SR-15780: spec review of STD-293 FR-106 admits set, bag and ordered-set values (QSL PR #1255)

## Summary

Ticket: STD-293 (QSL-814; related QSL-831 and STD-285). The diff is `48108c9c1...79687e8b2`, confirmed by `git ls-remote`. QSL main is now 43f4e2086, which is not an ancestor of the head; `git merge-tree` with main is clean, and main's new commits touch none of the three files. Spec Markdown only. FR-106 adds the `set`, `bag` and `ordered-set` values to the native snapshot grammar with QSpec FR-144's admission rules, narrows the set-field refusal (checks 6.2 and 12) to element types with no FR-181 form, and extends frame equality, the out-of-range witness index and value counting. It adds FR-106-AC-50 to AC-52 and TC-1171, and changes FR-106-AC-3 and TC-465 row 17 to a set of `String`.

These claims hold:

- The admission rules match QSpec FR-144 "Supplied collection values": bag as repeated elements; bound checked before any element is read with `cardinality_out_of_bound`; every element read before a duplicate is decided (AC-51's `x, 1, 1` bag is `invalid-value`, not `duplicate-member`); duplicates by FR-149 `=`, never deduplicated; set and bag array order not part of the value; ordered-set order kept.
- UUID and Timestamp elements use FR-106's native payload objects (`{"kind":"uuid","text":...}`, `{"kind":"timestamp","nanoseconds":...}`), whose payload grammar is FR-056's K2 grammar, the same one QSpec #372 adopts for FR-181. `Set<UUID>` is admitted only once #372 merges, and the PR body states that merge order.
- Narrowing the refusal to `String` is consistent with QSpec #372's FR-144 (string and population stay refused; UUID and Timestamp are read). TC-465 row 17 and FR-106-AC-3 match the narrowed check.
- Frame equality for `Set`, `Bag` and `OrderedSet`, the walk order, and `observation.values` counting are each extended consistently, and AC-52 can fail for each.
- FR-106-AC-50 to AC-52 and TC-1171 appear on no other open QSL PR head (#1241 holds TC-1170, #1258 holds TC-1172) and not on main. No status or changelog wording, and no compat layer.
- STD-285 item 3 (the Decimal element form) is correctly left undecided: FR-106 has no Decimal value form, so no spelling arises. It is recorded only in the PR body's "Ungrounded" section.
- Gate on the head merged with main 43f4e2086 (scratch merge, not pushed): `QSPEC_DIR=<QSpec #372 worktree> make check-spec-validation` exits 0 with no finding on FR-106, TC-1171 or TC-465.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: the new bullet says two documents whose set or bag elements differ only in order "give the same typed observation and digest". FR-106 defines a document's digest as `sha256-jcs` over the parsed document (line 113), and RFC 8785 keeps array order, so the two documents have different digests. Check 3 then compares the selected digest with that recomputed digest. The Text paragraph states the same case correctly: "different digests and one typed value". The next bullet also lists "its digest" among the values admission writes, but admission writes no digest of its own. <br>Fix: say the two documents give the same typed value and different document digests, and drop "its digest" from the written-values list, or define the typed-observation digest it means. | spec/functional/FR-106-admit-snapshots-and-invocations.md:240-247 |
| FND-002 | high | major: FR-106-AC-50 asserts that reversing the `ids` and `counts` elements "gives the same typed observation and digest". This contradicts the document digest definition, so the AC fails for every conforming implementation. TC-1171 step 1 and its expected result repeat the claim. <br>Fix: assert an equal typed observation and a different document digest, as the Text case does. | spec/functional/FR-106-admit-snapshots-and-invocations.md:957, spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md:21,29 |
| FND-003 | medium | major: "write each element in its canonical encoding ordered ascending by encoded bytes (QSpec FR-181)" does not say which encoding's bytes. FR-106's native spelling and FR-181's typed form order differently. A native reference JCS-encodes as `{"reference":{"key":...,"population":...}}` and orders by key, then population, while the FR-181 reference orders by universe, object-type digest, then identity hex. A native record keys `fields` by name, while FR-181 lists fields in declaration order. No vector checks the written order: `peers` is only ever empty. <br>Fix: say the order is the FR-181 typed canonical form's encoded bytes (or the native spelling's), and add a written-order vector with two references from different populations. | spec/functional/FR-106-admit-snapshots-and-invocations.md:244-247, spec/functional/FR-106-admit-snapshots-and-invocations.md:957 |
| FND-004 | medium | minor: no AC or TC-1171 case has a Timestamp element or an out-of-form UUID or Timestamp element. The fixture has only `Set<UUID>` with valid payloads. QSpec #372's FR-144-AC-22 admits `Set<Timestamp>` and refuses uppercase UUID and `01` as `invalid-value`, and FR-106 now relies on that reading. <br>Fix: add a `Set<Timestamp>` or `OrderedSet<Timestamp>` field to the fixture, and AC-51 cases for an uppercase-hex UUID element and a `"01"` Timestamp element, each `invalid_runtime_input`/`invalid-value` at the element. | spec/functional/FR-106-admit-snapshots-and-invocations.md:958, spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md:19 |
| FND-005 | low | minor: checks 6.2 and 12 refuse a field "whose element type has no QSpec FR-181 form", while the grammar bullet and QSpec FR-144 say "is, or contains at any depth". A `Set<Option<String>>`, or a set of a record with a `String` field, is refused under one reading and admitted under the other. <br>Fix: use "is, or contains at any depth, a type with no QSpec FR-181 form" in both checks. | spec/functional/FR-106-admit-snapshots-and-invocations.md:600-602, spec/functional/FR-106-admit-snapshots-and-invocations.md:700-702 |
| FND-006 | low | minor: the written-values bullet lists the out-of-range report among values written in canonical element order, but check 6.5 says the report carries "the supplied array index". For a bag or set the two orders differ, so the report's position for an element is unclear. <br>Fix: say the report's index is the supplied array index, and remove the out-of-range report from the canonical-order list. | spec/functional/FR-106-admit-snapshots-and-invocations.md:244-247, spec/functional/FR-106-admit-snapshots-and-invocations.md:636-638 |

## New findings (disposition pass 1)

Head bb5264aa6 (fix commit plus a merge of `origin/main` 423ac6aa0, which is an ancestor). `QSPEC_DIR=<QSpec #372 head 8f6263d74> make check-spec-validation` exits 0 with no finding on a touched file.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | minor: the TC-1171 fixture now has three `Tagged` objects (`t1`, `z1` in `T`, `a1` in `U`), but AC-50 gives field values only for `t1`. Every `Tagged` object must carry all five declared fields or check 6.4 refuses `missing-member`, so `z1` and `a1` need stated values for AC-50's snapshot to admit as written. <br>Fix: say `z1` and `a1` hold empty `ids`, `counts`, `order`, `stamps` and `peers` (each `{"set": []}`, `{"bag": []}` or `{"ordered-set": []}`). | spec/functional/FR-106-admit-snapshots-and-invocations.md:963, spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md:19 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The bullet now says reordered set or bag elements give "the same typed observation and different document digests, since the digest is `sha256-jcs` over the supplied document, which keeps array order", and the written-values bullet no longer lists a digest. |
| FND-002 | fixed | FR-106-AC-50 asserts "an equal typed observation and a different document digest" for reversed `ids`, `counts` and `peers`, and TC-1171 step 1 and its expected result match. |
| FND-003 | fixed | The written order is the JCS bytes of each element's FR-106 native encoding, taken from the admitted value, with references ordered by key then population and record `fields` keyed by name. AC-50's non-empty `peers` vector across `T` and `U` is written `a1` of `U` before `z1` of `T`; I recomputed the JCS bytes and `{"reference":{"key":"a1",...}}` sorts first. |
| FND-004 | fixed | The fixture gains `stamps: OrderedSet<Timestamp>[0, 2]` admitted in order `5, -1`, and AC-51 refuses an uppercase-hex UUID element and a `"01"` Timestamp element as `invalid_runtime_input`/`invalid-value` at element 0. |
| FND-005 | fixed | Checks 6.2 and 12 read "is, or contains at any depth, a type with no QSpec FR-181 form", and AC-51 adds a `Set<Option<String>>` vector. |
| FND-006 | fixed | Check 6.5 says the index is "the element's index in the supplied array, never its written order" for every kind; the out-of-range report is no longer in the written-values list; AC-52's witness vector is the bag `1, 10, 2` with index 1. |
| FND-007 | fixed | Disposition pass 2, head 70a50b675 (fix commit plus a merge of `origin/main` 4d34b5556, an ancestor; QSpec #372 is merged as c1033008): FR-106-AC-50 now says `z1` and `a1` each hold `ids` `{"set": []}`, `counts` `{"bag": []}`, `order` `{"ordered-set": []}`, `stamps` `{"ordered-set": []}` and `peers` `{"set": []}`, and the TC-1171 fixture says the same for every snapshot. `make check-spec-validation` against QSpec main c1033008 exits 0 with no finding on a touched file. |

## Verdict

Disposition pass 2: FND-007 is fixed and there is no new finding. Every finding is fixed. Mergeable: QSpec #372, its dependency, is merged, and main is an ancestor of the head. The owner questions (Timestamp form, native wire spelling and native-encoding write order) remain recorded in the PR body as lead defaults.

Disposition pass 1: FND-001 to FND-006 are fixed. FND-007 is a new low-severity fixture gap. The PR is mergeable once FND-007 is fixed, or with it deferred to the test author if the lead accepts that, and only after QSpec #372 merges. Owner note: the write order now follows the native encoding (key then population), which differs from QSpec FR-181 encoded-byte order for references and records; that is a lead default to confirm alongside the wire-spelling question.

Review pass: Not mergeable as is. FND-001 and FND-002 (the digest claim) and FND-003 (the unspecified byte order) are major and should be fixed in this PR. FND-004 to FND-006 are minor. The branch also needs a merge of QSL main 43f4e2086 (clean), and it must merge after QSpec #372. The FR-144 rule agreement, the K2 payload agreement for UUID and Timestamp elements, the String-only narrowing, the frame, walk and count extensions, id uniqueness and the spec-validation gate are verified. Owner decision: the native wire spelling `{"set"|"bag"|"ordered-set": [...]}` is a lead default, while QSpec FR-144 says the supplied wire form is FR-181's typed form "with no second grammar". The owner should confirm that the native snapshot grammar may spell these values its own way. Planner: STD-285 item 3 (Decimal element form) is still undecided and needs a Linear record.
