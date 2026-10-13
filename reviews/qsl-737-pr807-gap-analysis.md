---
id: SR-8716
title: "Gap analysis of quire-spec-language PR #807 (QSL-737): TC-255 and TC-260 after child composites"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ff905e16d51205409fddb3834f869343bad65eed; spec/functional/FR-087-typestate-and-cross-package-node-key.md, spec/functional/FR-088-clause-name-and-type-identity.md, spec/test-cases/TC-260-value-type-ref-is-exactly-two-member-union.md, xtask/src/typestate_scan.rs, qsl-replay/src/execute/argument.rs, qsl-semantics/src/model"
review_set: subset
---
# Gap analysis of quire-spec-language PR #807

## Summary

Ticket: QSL-737. This is a planless audit of FR-087-AC-6, FR-087-CON-3 and FR-088-AC-8 (TC-255, TC-260) at the PR head. It reads the computed matrix from `quire matrix --scope . --format json`. FR-087-AC-6 is tagged by both TC-255 tests. FR-088-AC-8 is bound by both TC-260 tests (status `method-without-symbol`: an Inspection criterion with test binders). FR-087-CON-3 is untagged, which was already true before this PR. The TC-260 scan does not look at enum-variant fields.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-088-AC-8 scan (`bare_type_fields`) only looks at struct fields. Enum-variant fields are never checked. In the same file the PR reasons about, `RawNode::Record { type_identity: String }` (observation.rs:544-547) holds the same wire string that `RawRecord` views, and tree.rs:134-138 `Open::Record { type_identity: String }` does too. Neither appears in the allowance or the FR-088 note, because the scan never sees them. A model enum variant such as `Foo { field_type: NodeKey }` would also pass, although AC-8 says "no model field type is represented by a bare `NodeKey` or a raw string type name". The FR-088 note lists "a field type under another name" as not covered, but does not list enum-variant fields. Fix: scan `Enum.variants` fields with the same rule (the scanner already collects them, typestate_scan.rs:83), and give the arena node the same wire-shape treatment. Or state the enum-variant blind spot in the FR-088 note. | xtask/src/typestate_scan.rs:1344-1368; qsl-semantics/src/model/observation.rs:544-547; spec/functional/FR-088-clause-name-and-type-identity.md:255-259 |
| FND-002 | medium | FR-087-CON-3 says its validation is "Test (TC-255)", but the computed matrix shows it `untagged`: no binders. Both TC-255 tests tag only `FR-087-AC-6`. `tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id` asserts exactly CON-3's rule (no mint outside `check`, no mint fed by a wire type). This was already true before the PR, but it is the constraint the PR body cites for the TC-255 fix. Fix: add `"FR-087-CON-3"` to that test's `#[trace(...)]`. | xtask/src/typestate_scan.rs:1023; spec/functional/FR-087-typestate-and-cross-package-node-key.md:584 |

## Verdict

**FAIL**, by this skill's rule that any untagged criterion in scope fails (FR-087-CON-3, FND-002). The fix is one trace id. Apart from that, the in-scope behaviour is backed:
- FR-087-AC-6: tagged by `tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id`, `tc_255_scan_sees_every_mint_spelling` and two arch-lint api_surface tests. The production change in argument.rs is the fix that makes TC-255 pass. It is a lookup, not a mint, as AC-6 and CON-3 require.
- FR-088-AC-8: bound by `tc_260_value_type_ref_is_the_two_member_union` and the new `tc_260_scan_rejects_bare_type_fields_beyond_the_raw_wire_allowance`. The new test is a real negative test: each of its three bypass shapes makes the scan report.
- FR-359-AC-9 (context): the record conversion the TC-255 fix touches is tagged by the child_composite tests in qsl-replay and qsl-semantics. The unknown-declaration branch is not tested (SR-8715 FND-001).

Reverse gap: none. Every changed production line belongs to FR-087-AC-6 or FR-359-AC-9. Stubs or coverage inflation: none.

## Coverage

- Criteria examined: FR-087-AC-6 (tagged), FR-087-CON-3 (untagged), FR-088-AC-8 (bound, Inspection), FR-359-AC-9 (tagged, context).
- Matrix source: `quire matrix --scope . --format json` at ff905e16d.
- Semantic review: done by reading, for the four criteria above, as part of this reviewer pass. No fan-out.
- Plan completion: not assessed

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2c067d235: `bare_type_fields` now walks both struct fields and enum variant fields under `model`. There are three (file, type, field) allowances: `RawRecord` and `RawNode` in observation.rs, and `Open` in observation/tree.rs. Each one is needed. The scanner visits items inside function bodies (typestate_scan.rs:408-410), and the coder's probe run without the `Open` allowance failed with exactly `observation/tree.rs: Open.type_identity is a bare String`. A grep of qsl-semantics/src/model finds no other `*type*` field typed as a bare str, String or NodeKey; the other hits are function parameters. The negative test now covers an enum variant holding a bare NodeKey or String, and the look-alikes `Other` and `OtherNode`. xtask: 182 passed. |
| FND-002 | fixed | 2c067d235: `#[trace("TC-255", "FR-087-AC-6", "FR-087-CON-3")]` is now on `tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id`. |
