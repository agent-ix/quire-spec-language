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

## Verdict

Not mergeable as is. FND-001 and FND-002 (the digest claim) and FND-003 (the unspecified byte order) are major and should be fixed in this PR. FND-004 to FND-006 are minor. The branch also needs a merge of QSL main 43f4e2086 (clean), and it must merge after QSpec #372. The FR-144 rule agreement, the K2 payload agreement for UUID and Timestamp elements, the String-only narrowing, the frame, walk and count extensions, id uniqueness and the spec-validation gate are verified. Owner decision: the native wire spelling `{"set"|"bag"|"ordered-set": [...]}` is a lead default, while QSpec FR-144 says the supplied wire form is FR-181's typed form "with no second grammar". The owner should confirm that the native snapshot grammar may spell these values its own way. Planner: STD-285 item 3 (Decimal element form) is still undecided and needs a Linear record.
