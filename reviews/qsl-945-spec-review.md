---
id: SR-12290
title: 'Spec review of PR #1170: FR-396 amendment and TC-1090 step 6 (unions), spec-review'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@f3eb22fe0; spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md, spec/test-cases/TC-1090-the-replay-facade-returns-the-admitted-record-and-tuple-keys.md; ticket QSL-945'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-945. Spec review of the in-place FR-396 amendment (title, Description, Outputs, Behavior, AC-2, AC-3, new AC-6) and TC-1090 step 6. Sub-analyses applied: integrity (consistency with the measured key semantics) and criterion strength. `quire validate` on both files: exit 0.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-396 Description | spec/functional/FR-396-*.md | examined |
| FR-396 Outputs | spec/functional/FR-396-*.md | examined |
| FR-396 Behavior (witness naming) | spec/functional/FR-396-*.md | examined |
| FR-396-AC-2 | spec/functional/FR-396-*.md | examined |
| FR-396-AC-3 | spec/functional/FR-396-*.md | examined |
| FR-396-AC-6 | spec/functional/FR-396-*.md | examined |
| TC-1090 step 6 | spec/test-cases/TC-1090-*.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Description now covers unions but still says the kernel key is what "every replayed composite value carries". For a union that is false: a replayed union value's member names the settled key, which is the `composite_type` node id (measured). The Behavior bullet mentions only the witness wire form. The spec never states the value-level asymmetry, or that the node id is the union's settled key, i.e. the key a consumer seals with and that every `UnionMember` carries. A CG implementer reading FR-396 will pick the wrong key. | spec/functional/FR-396-*.md Description, Behavior |
| FND-002 | medium | FR-396-AC-6's consumer clause, "declares a closed `TypeEnvironment` from them", is met by an environment that admits no union value: an unsealed union declaration. The AC should require the consumer to seal the union with the admitted bindings and admit a value of each member, without constructing a key. AC-1 sets that bar for records and tuples. | spec/functional/FR-396-*.md AC-6; spec/test-cases/TC-1090-*.md step 6 |
| FND-003 | low | FR-396-AC-6 is compound. It has five independently failing clauses: lookup and form; members vs the producer's; distinct keys and ids; the root-only consumer; the replay witness. Splitting it would give failures a precise trace. | spec/functional/FR-396-*.md AC-6 |

## Verdict

FND-001 and FND-002 are mediums and do not block on their own. They are the spec side of SR-12288 FND-001/FND-002 and should be fixed in the same round. The AC-2/AC-3 amendments and the removal of union `WrongKind` are consistent with the code and the tests.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | In the FR-396 Description, the sentence after the new union clause begins "That key is not the id of the declaration's `composite_type` node". It now follows "…are that node id", so "That key" reads as the union's node id, which contradicts the clause just before it. Say "The kernel key is not…". This is wording only and does not block. | spec/functional/FR-396-*.md:32 |

## Dispositions

Round 1, reviewed at 84af1b5a0 (fix commit f37c88d1b). These rows replace the coder's proposed outcomes; the reviewer checked each against the code. Re-run for this round: `admitted_composite_facade` (4 pass), `admitted_composite_root_only` (2 pass), clippy for qsl-replay with all targets and features (clean), and `quire validate` on FR-396 and TC-1090 (exit 0). Mutation probe on `a_consumer_seals_a_union_and_admits_a_value_of_each_member`: (A) with the seal loop removed, the test fails at the first `union()`, line 199. (B) With another union's member id (`Wrap.Plain`) used for `Shape.Empty`, it fails at line 201. Both mutants were reverted.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f37c88d1b: the FR-396 Description and Outputs now say a union is settled under its `composite_type` node id, which is `union_key(key)`, the `UnionMember` declaration and the seal `final_key`, and that an unsealed union admits no value. A new wording low on the next sentence is FND-004 below. |
| FND-002 | fixed | f37c88d1b: the new FR-396-AC-7 requires sealing with `AdmittedUnionMember::member` and admitting a value of every member, including a nested union, without constructing a key. TC-1090 step 7 matches it. |
| FND-003 | fixed | f37c88d1b: AC-6 is split into AC-6 (members), AC-7 (root-only seal and admit), AC-8 (distinct and stable) and AC-9 (witness). The tests' trace tags were moved to match. |
