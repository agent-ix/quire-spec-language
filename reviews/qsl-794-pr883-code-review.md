---
id: SR-11625
title: "Code review (with rust-review) of quire-spec-language PR #883"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@353a23f03780b9c584f60784228073e76b2f6e84; PR #883 own commit (HEAD~1 = PR #880 head fe2519e2a): qsl-package/src/emit.rs, qsl-replay/src/spine/clause/tests.rs, qsl-semantics/src/check/{lowering.rs,lowering/model.rs,lowering/state.rs,mod.rs,node_key/mod.rs}, qsl-semantics/src/model/{index.rs,intake.rs,normalize.rs,refusal.rs}, qsl-semantics/tests/it/{model_operations.rs,relationship_snapshots.rs}"
review_set: subset
---
# Code review of quire-spec-language PR #883

## Summary

Ticket: QSL-794 slice (c). One pass of code-review with the rust-review
lane, scoped to PR #883's own commit 353a23f03 over PR #880's head
fe2519e2a. Reviewer-only: no builds, no edits. Blocking bar: wrong result,
panic, failed AC, vacuous or weakened test, broken gate, overclaiming
trace tag, unupdated exhaustive match, deleted variant still named.

Examined, clean:
- Intake (FR-103 Behavior, FR-103-AC-2): `modifies` admits Field or
  navigation Relationship; a systems connection (no `declared_at` slot)
  classifies Other and refuses `invalid_model_binding`/
  `malformed-declaration`, which is FR-103's "declaration of any other
  meaning" bullet (FR-103 resolves `modifies` to a field or a *navigation*
  relationship only). `creates`/`deletes` naming a relationship refuse
  `malformed-declaration`. Unknown key still `missing-name`.
- `FrameEntryUnsupported` / `unsupported_frame_feature` deleted: no
  remaining reference in any crate, test or doc (only historical
  `reviews/` text). All exhaustive matches in refusal.rs updated.
- normalize.rs: an every-object `modifies` entry that is a navigation
  relationship skips the `UnknownFieldWrite` check; receiver-scoped entries
  stay field-only.
- node_key `FrameModifies`: a Field entry serializes through the unchanged
  `FrameField` Serialize (`{kind:"field", declaration, name}`), and the
  depth is max of two equal-depth wires, so a field-only frame's bytes and
  node digest are unchanged. Relationship entry is
  `{kind:"relationship", declaration}` per QSpec FR-340.
- lowering/state.rs: sort key `(NodeKey, Option<String>)` with `None`
  sorting below any name matches QSpec FR-340's "relationship entry's name
  empty"; dedupe preserved; key-visit and key-map cover both arms.
- Check 11.5 reach: production callers (qsl-eval protocol_clause, qsl-replay
  spine/execute) pass `frame.operation().declaration.effect()`, and
  `first_link_change` skips any relationship whose key is in
  `effect.modifies`. The Link witness's `modifies` carries field and
  relationship entries alike (FR-115).
- Other `effect.modifies` consumers (conformance effect axis,
  field_refinement, observation field-write check, population) only use
  `contains`/lineage lookups; a relationship key never matches a field and
  never panics.
- Tests: TC-458 step 2 parts 4 and 5a, TC-1037 rows 1, 2, 3, 5 over the
  package-declared frame. Mutations checked mentally: relationship refused
  again (admit test fails), grant ignored in 11.5 (row 1/5 Holds fails),
  connection admitted (intake unit test fails), creates admitted (creates
  test fails), frame node missing the Relationship entry (slice pattern
  fails).

Recorded, non-blocking:
- medium: no end-to-end systems Connection fixture; the connection case is
  covered by a unit test over `frame_entry_kind`/`resolve_frame` only.
- medium: no test of a mixed field-plus-relationship frame node's order;
  the tuple sort is trivially correct.
- low: index.rs sets `navigation_relationship` with `=` rather than `|=`.
- low: the connection unit test's `FR-103-AC-2` tag covers more than AC-2
  states (AC-2 says nothing about connections).
- note (pre-existing, not this diff): QSpec FR-340 main now carries a
  field entry `scope` and `deletes_scoped`; the field wire here has neither.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** No blocking finding. Intake, lowering, wire form, sort order and
the check 11.5 path agree with FR-103-AC-2, TC-458 step 2, FR-115 and QSpec
FR-340, and field-only frame nodes keep their digests.
