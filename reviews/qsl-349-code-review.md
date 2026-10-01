---
id: SR-929
title: "QSL-349 code review (with rust-review lane) of PR 549, enum member and declared-type generated-occurrence placement"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@de0ff633d58bb5b3e3434d62e3b0b5ad3458ba21; qsl-semantics/src/check/lowering.rs (reached_nodes, enum_members, enclosing_declarations, roots, relax); qsl-semantics/src/check/refusal.rs (Origin::TypeDeclaration doc); qsl-semantics/src/check/region.rs (module doc); qsl-package/src/emit/tests.rs (QSL-349 tests and helpers)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-349. PR: quire-spec-language#549 at de0ff633, diff `26095e55...de0ff633`.

Root cause confirmed. Before the change, `enclosing_declarations` relaxed only along
`named_nodes`. Nothing names an enum member except a literal, and a declared type no
function names is reached by no root at all. Measured: with this PR's tests applied over
the base `lowering.rs`, all three active tests and the ignored one fail with
`UnlocatedOccurrence { role: Generated, ordinal: 0 }` (members `499f…65ec`, `ec92…196d`;
record field type `07f6…4e32`).

The coder claimed "nodes that were already placed don't move". That claim holds, and I
checked it at both passes.
- Pass 1 adds only one new kind of edge: declaration to member. A member names only its
  declaration, and the declaration already holds an anchor at or below the one that
  reached it. So the only anchors that can change are member anchors. A member that a
  literal names carries located `expression` occurrences and no `generated` one. I
  measured this: `Status::READY` in a function body gives `[expression 0, expression 1]`
  and no generated entry. So an anchor that changes is never read for a node that used to
  emit. Every member that does read its anchor was refused before this PR.
- Pass 2 never relaxes into a node pass 1 placed (`placed` guard), and its roots exclude
  pass-1 keys. Anything a pass-1 node reaches was already placed by pass 1. So skipping
  placed nodes loses nothing.
- The existing emit and lowering tests that pin regions pass. I ran
  `emit::tests` (49 + the ignored one) on de0ff633, and the whole workspace on
  origin/main with this commit applied. The coder's gate log has make ci exit 0 on
  de0ff633. I did not re-run it.

Determinism of pass 2: the anchor is the minimum under the derived `Ord` of `Location`,
reached by a min-fixpoint relaxation over `BTreeMap`s. So the result does not depend on
iteration or source order. For `TypeDeclaration` the order is the declared name, as
`String` order (UTF-8 bytes). Measured: with `record Q { y: Int[0, 9]; }` declared
before `record P { x: Int[0, 9]; }`, the shared `Int[0, 9]` node is placed at `P`. The
spec does not state this order (SR-931 FND-001), and no test pins it (SR-930 FND-003).

Rust-review lane. The new helpers are small and private. `reached_nodes` borrows rather
than clones the member lists. There is no new `unwrap` or panic in production code. The
`is_none_or` relaxation and its termination argument are unchanged. The test helpers use
`unwrap` on wire JSON, which is fine in tests. The `usize::try_from` conversions on
region offsets are checked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `Origin::TypeDeclaration`'s new doc says the `generated` occurrence of "a node only this declaration reaches" is located here. That is not the rule the code applies. A node reached by two declared types (the shared `Int[0, 9]` of `P` and `Q`) is placed at the least of them, though neither is the only one that reaches it. A node only this type reaches is also not placed here when any function body, measure, state clause or attempt reaches it. Say "a node no body, measure, state clause or attempt reaches, when this is the least declared type name that reaches it". | qsl-semantics/src/check/refusal.rs:288-292 |
| FND-002 | low | The rewritten `region.rs` module doc says a generated occurrence is recorded "at the body or measure root of the least function declaration that reaches its node". `enclosing_declarations` also places at state clause and protocol attempt roots in the same first tier. The sentence the PR rewrote still leaves those two out. | qsl-semantics/src/check/region.rs:14-18 |

## Verdict

Correct. The fix is in the anchoring rule, where the root cause is. It places exactly
the nodes that used to be refused, and no node that used to emit changes region. Two low
doc-accuracy findings. Mergeable once the gap-analysis medium findings (SR-930) are
handled in the fix round.
