---
id: SR-2334001
title: "spec-review of S7 bytes-route slot naming and equality file (quire-spec-language PR 2334)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2334 at 8297ec81fae767c8842f821e4eccbc59fb977118; FR-533, FR-531, FR-390, FR-517, FR-383, TC-1333, TC-1331, TC-1070"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-533
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: references
---

# Spec review of S7 bytes-route slot naming and equality file (quire-spec-language PR 2334)

## Summary

One pass over `git diff origin/main...HEAD` (head 8297ec81f), read-only, planner tiering (no mutation runs).
Verdict: NOT mergeable until FND-001 and FND-002 are fixed; FND-003 to FND-007 should be fixed in the same round.

Verified true. (a) The fixture L vectors: in `qsl-semantics/tests/support/tc1070.rs` `SOURCE` (2836 bytes) the
expressions `got.ready` (visible), `got.ready` (case yes) and `not got.ready` (case no) sit at byte offsets
2641..2650, 2676..2685 and 2735..2748 exactly, so the three origins are the real spans (I computed them from
the source text; I did not run the emitter). Cases are held sorted by label (FR-383 line 128), so
`cases.guard[0]` is `no` and `[1]` is `yes`; the table's assignment (`guard[0]`=2735, `visible[0]`=2641,
`guard[1]`=2676) is the source-route assignment. (b) Rule 2 is right for the three: only the origin at 2735
names the `not` application. (c) PR 2333's `bare_slot.rs` (B1) implements none of rules 2 to 5: `origin_of`
filters rows by the root's own application and returns every name in a `BTreeSet<String>` (bytes order, several
origins); the spec describes the B2 target and correctly forbids that behavior. (d) Pinned leaf encodings match
Contract IR at the locked rev (35a9649): `CheckedNodeId` `{package, path, ordinal}` with path segments text or
`{ "node": i }`; `CheckedOccurrence` `{role, ordinal}` with `role` snake_case (declaration, type, expression,
anchor, claim, generated); `CheckedSourceMapEntry` `{node_id, role, ordinal, regions[{source{authority,identity},
start, end}]}`; `CheckedNodeOwner` tag `kind` (`model_root` `{identity}`, `source` `{authority, identity}`,
`model` `{identity, node}` with `node` text); `CheckedRational` `{numerator, denominator}` as strings; the
vector's members and order match those derives. (e) The new Package-locus reasons sit after the three identity
reasons, consistent with their order. (f) `quire-eval` (rev 4e59f72, result.rs:96) has `CallAdmissionLocus`
`Node(NodeKey)`, `Population`, `Member { name, index: Option<usize> }`, `Binder`: no `Package` variant, as the
spec says. (g) FR-322 (QSpec) writes a case guard's site name from its span only (lines 1165-1176), so the
registry-owner statement in FR-533-AC-13 and Dependencies is right. (h) New ids FR-533-AC-11..17, FR-531-AC-14..17,
FR-390-AC-65..67, FR-517-AC-21..22 are unused on main and in every open PR; PRs 2309, 2311 and 2333 (the
IR-1325 lines of FR-531/FR-533) each trial-merge with this head with no conflict; no ticket id in any new AC
text; `quire validate` exit 0 on all 8 files.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-531-AC-14 and TC-1331 row 13 test the equality file on **R**'s directory ("a slot call whose walk holds an equality application evaluates"), but FR-517-AC-21 says **R**'s unit has no equality application and its file holds three empty lists. The test cannot pass as written. It is also fragile: **R**'s `Full` recover is "the receipt sum equals the captured target" (FR-517-AC-14), an equality, so the statement that **R** has none becomes false when that AC lands. Use a named fixture (or a unit added to the test) with an equality for AC-14/AC-16 and AC-21's non-empty case, and do not assert that **R** has none. | FR-531-AC-14; TC-1331 row 13; FR-517-AC-21, AC-14 | wrong-requirement |
| FND-002 | high | FR-390-AC-67 reports `MissingTypeRepresentation` (a `Node`-locus reason) before the `Package`-locus witness reasons, but FR-390 orders unsupported projections by package identity, then locus (`Package` before `Node`), then node key, then reason. A package with a `MissingTypeRepresentation` and a `ForeignIdentityWitness` must report the witness first. The reason list in the paragraph is only the tie-break within one locus. The AC states the opposite and TC-1070 C17 repeats it. (The identity-first behavior of the real code, `bare_slot` decides identity rows before any node is read, agrees with locus order, not with AC-67.) | FR-390 lines 439-450; FR-390-AC-67; TC-1070 C17 | wrong-requirement |
| FND-003 | medium | "Equality slots" says the equality rows are "not derivable from the package's nodes". The package carries `checked_term_data.equality_decisions` (QSpec FR-322 lines 1085-1126, AC-288): operator, operand positions, source types, conversion targets, schedule, unit and enum facts keyed by the application node and occurrence. IR's `SelectedEqualityFactRow` also pairs each `NodeKey` with its `CheckedNodeId` (`base`/`base_node`, `target`/`target_node`), and `bare_slot` already recovers keys with `WireGraph::key`, so the keys, occurrences and source-map entries are derivable from the package. What the rows add is the producer's witness (authored `Location`, checker-held `value_type`, producer-selected fact), which IR's `admitted_equality` demands and refuses to reconstruct. "The handoff carries it" is sound as the same mechanism as `identities.json`, but the stated reason is overclaimed; word it as "IR's read requires producer witnesses it does not reconstruct", name what is and is not derivable, and keep the removal clause. | FR-533 "Equality slots"; FR-531 "The equality file"; IR equality_projection.rs:245-356 | wrong-requirement |
| FND-004 | medium | FR-531 says the writer takes the rows "from the in-process unit, never from the emitted bytes", but every node and key member is written as the package's wire node id (`{ "package", "path": [..., { "node": i }], "ordinal" }`), and an in-process `CheckedNodeId`/`NodeKey` carries structural (nested) path segments whose serde form is not the wire form. Producing the wire id needs a join through the emitted package (`WireGraph::of_package_bytes`, as FR-517's inventory section now says; FR-517-AC-22 `Unnameable` already presumes it). Restate as "rows from the unit, node members joined to the wire id through the emitted bytes". | FR-531 "The equality file" intro and leaf encodings; FR-517 writer paragraph | wrong-requirement |
| FND-005 | medium | FR-533-AC-12's byte-order clause is wrong: "ordered by the UTF-8 bytes of the names they would take `1000..1004` and `1001..1002`". By rule 2 the candidates for `visible[0]` and `cases.guard[1]` are only `999..1005` and `1000..1004` (`1001..1002` names `not`). Byte order gives `visible[0]` `1000..1004` and `cases.guard[1]` `999..1005`. The test still fails under byte order, but the sentence names the wrong values. | FR-533-AC-12; TC-1333 row 12 | wrong-requirement |
| FND-006 | medium | Rule 4 ranks tied slots by kind ("`visible` before `cases.guard`") but is silent on `related.from` against `related.to` and on several `related` entries, which are also span-named, can have non-leaf roots (`related by Owns (a.f, a.f)`) and share a walk. Their pairing is undefined, while the section says no slot is ambiguous. Add the kind order (`from` before `to`, by index) or state that a `related` end with a non-leaf root is covered by AC-13's unmet parity. | FR-533 rule 4 | wrong-requirement |
| FND-007 | medium | FR-533-AC-16 uses "a String, UUID, Timestamp, enum and quantity equality in the added slots of TC-1333", but TC-1333 defines no such slot or fixture, so the AC cannot be executed from the spec. It also lists the integer equality of `Refund`'s retry relation and recovery predicate as needing equality rows, while FR-533-AC-1 and AC-2 evaluate those same slots on the bare route over TC-1070/TC-1315 supplies that carry no equality rows; one of the two must change (state whether integer `=` needs an admitted decision; if yes, AC-1/2's supplies need `equality`). | FR-533-AC-16, AC-1, AC-2; TC-1333 row 16 | wrong-requirement |
| FND-008 | low | FR-517-AC-21 and AC-22 name verification `Test` with no TC and TC-1312 is unchanged (scope still AC-1 to AC-20, no row, no trace tag), so the matrix has two criteria no test row covers. | FR-517-AC-21, AC-22; TC-1312 | wrong-requirement |
| FND-009 | low | Dependencies says the `Package` variant is added "through the first-party dependency, or asks the Contract IR lane"; `CallAdmissionLocus` lives in `quire-eval`, a separate repository, so name that owner. The "unmet until it exists, no cut" wording itself is right. | FR-533 Dependencies | wrong-requirement |

## Disposition

Not mergeable. Fix FND-001 and FND-002 first; FND-003 to FND-007 in the same round; FND-008 and FND-009 may ride along.


## Disposition pass 1 (head 50f7f198096599b86b3f6198101b811df5cb06ca)

Confined re-check of the fix round. `quire validate` exit 0 on all 9 changed spec files; trial merges of PRs 2309, 2311 and 2333 each merge cleanly with this head (re-run); the new ids FR-533-AC-11..17, FR-531-AC-14..17, FR-390-AC-65..67 and FR-517-AC-21..22 are unused on main and in every other open PR; no ticket id in any AC text.

| ID | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | resolved | FR-531-AC-14/16, FR-517-AC-21 and TC-1331/TC-1312 now use TC-1070 fixture E (`Refund`'s retry relation holds the integer `=` `later.tally = earlier.tally + 1`, TC-1070 line 51; recovery predicate `recovered.total = target`) and the empty-lists case on the inventory vector's `unit.native`, whose only expression `if p then x + 1 else x + 1` holds no `==`/`!=`. FR-517-AC-21 no longer says **R** has none. |
| FND-002 | resolved | FR-390-AC-67 and TC-1070 C17 now follow FR-390's stated order (package identity, locus `Package` before `Node`, node key, reason). Checked each statement: `ForeignIdentityWitness` over a Node `MissingTypeRepresentation` and `MissingEqualityWitness` over a Node `MissingLiteralProjection` (locus order); `ForeignIdentityWitness` over `MissingEqualityWitness` (same locus, reason list order); Missing, Unknown, Foreign for the equality reasons (reason list order). The real `bare_slot` decides the identity rows before any node is read, in agreement. |
| FND-003 | resolved | "Equality slots" now says the package carries `checked_term_data.equality_decisions`, that keys, occurrences and source-map entries are recoverable from the wire graph, and that the rows add the producer's witness (authored `Location`, `value_type`, selected fact) which `admitted_equality` does not reconstruct; whether IR can derive it is stated as an open question and the removal clause stays. Consistent with FR-322 lines 1085-1126. |
| FND-004 | resolved | FR-531 and FR-517 now state the same join as the inventory section: nodes and keys are joined to the wire id through `WireGraph::of_package_bytes`, a node the package does not name is `Error::Unnameable`, with the reason (structural vs wire path segments). |
| FND-005 | resolved | AC-12 now says byte order (`1000..1004` before `999..1005`) gives `visible[0]` `1000..1004` and `cases.guard[1]` `999..1005`. |
| FND-006 | partly resolved | Rule 4 now orders tied slots by member (`visible`, `cases.guard`, `related.from`, `related.to`) with the interleaved `from[0], to[0], from[1], to[1]` order for `related`; AC-13 covers `related` records in emitted record order (low: "ordered by member then index" and the interleaved related order are both stated, the second overrides; say so once). Two defects remain, see FND-010 and FND-011. |
| FND-007 | resolved | Verified in `reader_protocol_projection.rs`: the `eq`/`ne` identities of string, uuid, timestamp, boolean, integer, rational, decimal, quantity and structural (lines 1036-1054) all go through `equality()`, which returns an internal fault `reader-equality-has-producer-identity-row` when `equality_producer_identity_rows` has no row (lines 1111-1118) and then reads `admitted_equality`. FR-533-AC-1/2 now state the bare supply carries the rows read from `equality.json`. TC-1289 (finish.constraint over String/UUID/Timestamp record fields) and TC-1290 (finish.constraint over Quantity `s`/`ms`) exist and hold the equalities AC-16 names. The enum equality was dropped, see FND-012. |
| FND-008 | resolved | TC-1312 scope is AC-1 to AC-22, rows 21 and 22 and the trace tag are added, and FR-517-AC-21/22 name `Test (TC-1312)`. |
| FND-009 | resolved | Dependencies names `quire-eval` as a separate repository with its own lane. |
| FND-010 | medium (new, in the fix) | The FR-533-AC-14 row is malformed: the `related` vector was appended after the Verification cell (`... | Test (TC-1333) In a package copy ... | Test (TC-1333) |`), so the row has six fields against three columns; `quire matrix` lists FR-533-AC-14 `untagged`. Move the sentence into the Criteria cell. |
| FND-011 | medium (new) | The `related` vector's "with the origin names swapped in the rows the two slots take the other order" cannot change anything: both origins carry rows for the same single application, so the candidate set is the same two names and rank 0 is `100 105` whichever row carries which name. The clause cannot fail. Replace it with a distinguishing case (two different applications, or a different numeric/byte order such as `99 100` and `100 105`). Also the vector is a hand-edited package (ends replaced by one application) with no source-route counterpart; rows with `term_ordinal` 0 and 1 for one application are valid under FR-322 lines 1138-1146, but a compiled fixture (`related by Owns (view.f, view.f)`) would let AC-12's source-route equality apply. |
| FND-012 | low (new) | AC-16 drops the enum equality (FR-390; the equality file encodes `enum_member` facts and AC-17 round-trips every fact tag) with no recorded gap and no TC fixture; per the no-scope-cut rule either keep a fixture for enum equality on the bare route or list it as unmet in Dependencies. |

Verdict after pass 1: not mergeable only for FND-010 (mechanical) and FND-011; FND-012 and the FND-006 wording nit may ride along. Everything else is resolved.
