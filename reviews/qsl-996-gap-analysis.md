---
id: SR-16368
title: "Gap analysis of quire-spec-language PR 1423 (QSL-996): constraint readers count toward weak sharing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@abcee1ae35d6e8cecc4e267bc6efc8b6e21b94aa; PR #1423 diff against origin/main 3b19ec5fe; FR-220 Sharing and classification and Access shape, FR-220-AC-15, TC-665 step 14; qsl-eval/src/simulation/protocol/access.rs, qsl-semantics/src/check/footprint.rs, qsl-eval/src/simulation/model/footprints.rs; trace tags at the reviewed sha"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-220
---
# Gap analysis of quire-spec-language PR #1423

## Summary

Ticket: QSL-996. This is a planless audit of the changed requirement surface:
FR-220's reader statement and FR-220-AC-15, checked against the trace tags,
the tagged test and the source. The spec was not changed by the PR.

Examined:
- FR-220-AC-15 (examined). One test, `weak_memory::a_constraint_reading_a_shared_location_settles_the_multi_location_shape`, is tagged `#[trace("TC-665", "FR-220-AC-15")]`. It passes at the head (`cargo test -p qsl-analyze --test it`), and the reviewer's red check fails it with the production reader loop disabled.
- FR-220 "Sharing and classification" reader statement (examined)
- FR-220 "Access shape" reader refusal statement (examined)
- TC-665 step 14 and its expected result (examined)

Reverse gap: every new production path is owned by FR-220's reader
statements: `derive_node_reads`, `ModelSystem::node_reads`, the reader loop
and touches in `classify`, `first_shape`'s reader fallback, and
`WeakAccesses::node_reads`. Two behaviours have no requirement text: the
settlement order between attempt and reader shapes, and `transition: None`
for a reader. No stubs, `todo!` or placeholder bodies were found in the diff.
The test asserts real outcomes, not tautologies.

Semantic review was run, because the dispatching brief asked for an
intent-versus-test check. The test matches AC-15's intent for the `tso`/`ra`
refusal, the `sc` admission and the `finish` admission. The unaccessed-`z`
leg is realised with a `Spare` type, not a `Cell`, because a `Cell` z is
accessed by every `Cell::set` attempt under EN-1 instantiation. The reviewer
reproduced this. See FND-001.

## Verdict

CONDITIONAL. AC-15 is tagged and its test discriminates, but the test
departs from the AC's literal premise, and that needs a spec ruling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Intent and test disagree in letter on FR-220-AC-15's unaccessed-`z` leg. The AC asks for "a `Cell` `z` that no attempt accesses", but under EN-1 every `Cell::set` attempt instantiates on every `Cell`, so no such Cell exists (reviewer probe: a Cell z gives shared {x.v, y.v, z.v} and the reader is refused). The test uses a `Spare` object and retypes the `over` input. The AC text (and TC-665 step 14) should be amended by the spec owner, or the owner should rule that instantiation honours `pre` contracts. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:202; qsl-analyze/tests/it/weak_memory.rs:859-873 |
| FND-002 | low | Behaviour with no owning requirement text: reader shapes settle after every attempt `MultiLocation` shape, and a reader's `transition` is `None`. The tagged test asserts `transition == None` under FR-220-AC-15, which states nothing about it. Both are the lead's defaults (ticket QSL-996), not owner answers. | qsl-eval/src/simulation/protocol/access.rs:196-214; qsl-analyze/tests/it/weak_memory.rs:821 |

## Coverage

- `quire matrix --scope .` at the head: FR-220-AC-15 `tagged`, binder `weak_memory.rs:799`.
- Criteria in scope: 1 (FR-220-AC-15); tagged 1; untagged 0; ignored-only 0.
- Tagged test passes at the head; reviewer red check fails it without the production change (no reader refusal under `tso`, `weak_memory.rs:817`).
- Reverse gap: 1 unspecified behaviour pair (FND-002).
- Stubs / coverage inflation: none.
- Semantic review: run (dispatching brief), FR-220-AC-15 only.
- Plan completion: not assessed

## Dispositions

Round 1, verified against `9e04f511976f8279747935487477475f379859fd`. `quire matrix --scope .` at that sha: FR-220-AC-15 `tagged`, binder `weak_memory.rs:806`. The tagged test and the new tests pass (`qsl-analyze --test it` 728 passed).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-1040 (spec ruling on AC-15's "Cell z"). The test doc now records that it realises the AC's intent, not its letter. |
| FND-002 | fixed | The AC-15 test no longer asserts `transition`. The order and the reader's `None` are pinned by an untagged test documented as lead defaults pending QSL-1040, so no AC tag binds them. |
