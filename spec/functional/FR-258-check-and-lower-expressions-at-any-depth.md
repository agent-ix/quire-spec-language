---
id: FR-258
title: "Check, lower and emit expressions and types at any depth"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-258: Check, lower and emit expressions and types at any depth

## Description

The S3 checker SHALL type, check obligations for, lower and key expressions
and types of any depth, bounded only by the checking node limit, the
per-declaration preimage byte limit and the checking work budget (ADR-030
D-4.3, NFR-011). QSpec FR-146 defines no expression depth or nesting limit
for checking; this requirement carries that rule through QSL's checker.

## Behavior

1. **Limits.** `CheckingLimits` SHALL hold the node limit (`s3.nodes`), the
   declaration preimage byte limit (`s3.input_bytes`) and the work budget
   (`s3.work_units`), each set through its builder method. Constructing
   `CheckingLimits` SHALL always succeed. The checked-family contract's
   per-declaration stage limit SHALL be input bytes, with the work budget
   charged through the contract meter (FR-062). The node limit SHALL be
   charged per node, at the node whose entry crosses it; no per-declaration
   node count is compared before typing.
2. **Arena checked nodes.** The checker SHALL store the checked `Node` as one
   vector of nodes per checked body, with each child named by a typed index, so
   that cloning, comparing, hashing, formatting for debug and dropping a
   checked body visit its nodes without native recursion.
3. **Iterative walks.** Every walk over parsed forms, checked nodes or
   `ValueType` SHALL keep its pending steps on a heap stack. The walks over
   checked nodes (definedness facts and their interval and outcome
   evaluation, claims collection, the observation walk, `descendants` and
   `catalogued_step`) and the FR-093 text-leaf walk SHALL run on the walker
   toolkit (FR-356). Typing and expression lowering run
   frame machines of their own, each frame the continuation of one typed
   form with the results it still waits on; their stacks are heap stacks, and
   moving them onto the toolkit is the remainder of this requirement
   (Status). The checker is in the qualified core, so no checker walk SHALL
   grow or switch the native stack. Each stack SHALL grow by at most a
   constant per node or composite already charged against `s3.nodes` or
   `s3.work_units`.
4. **Iterative text-leaf walk.** FR-093's text-leaf walk SHALL run on the
   walker toolkit with one typed frame per entered type, holding the
   type being walked and, for a composite, its field cursor and the open-list
   length on entry. It SHALL charge one node unit per leaf it appends and one
   work unit per composite it enters, as FR-093 defines, and nothing per
   level.
5. **Stratified semantic terms.** QSL's semantic term SHALL be a set of
   stratified types matching the v2 body grammar of QSpec FR-322: a leaf
   term, a group term, a tuple term, a member term and a body term, none
   naming itself or a higher stratum. A binding never holds a binding. Lowering SHALL place every composite subterm in its own
   node, reached by reference. Node keying over a body term SHALL be a
   fixed-depth match, so every node-key preimage has a depth fixed by that
   grammar.
6. **Limit outcomes.** When the next checking charge would exceed a
   `CheckingLimits` bound, the checker SHALL stop with `stage_limit_exceeded`
   naming that limit, its configured value, the count reached and its
   setting (FR-255), at the locus FR-096 gives the producer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-258-AC-1 | On a thread with a 512 KiB stack, a function whose body is a 100,000-term sum, one whose body is a 100,000-long `else if` chain and one whose body is 100,000 nested `let`s each check, lower and emit under S1 and S3 limits raised to fit them. Each emitted node's key recomputed from its preimage equals its `node_id`. Every checked function body and the lowered semantic graph clone and compare equal to their clones, the checked package formats for debug, and all of them drop on the same thread. | Test (TC-725) |
| FR-258-AC-2 | On a thread with a 512 KiB stack, structural equality over a parameter typed with 100,000 nested `Option`s around `Text[0, 8; nfc]`, and over a chain of 100,000 records each holding the next in an optional field `next` and the last holding a `Text[0, 8; nfc]` field `label`, each lowers with one text leaf, under S1 and S3 limits raised to fit it: the `Option` chain's path has one `inner` segment per level, and the record chain's has two per level (`field:next`, `inner`) and ends `field:label`. A parameter typed with 100,000 nested `Option`s around `Boolean` is keyed by FR-092's type-keying walk on the same thread. | Test (TC-726) |
| FR-258-AC-3 | At the default limits, a function whose body is the longest `a and (…)` chain S1's defaults admit checks, lowers and emits with no outcome naming a depth. `CheckingLimits::default()` holds 100000 nodes, 16777216 preimage bytes and 16777216 work units, and constructing `CheckingLimits` with any node count succeeds. | Test (TC-727) |
| FR-258-AC-4 | A function whose body is a 1,000-term sum (1,999 expression nodes), checked with `s3.nodes` at 1,500, stops with `stage_limit_exceeded`/`node-count-exceeded` naming bound 1,500, count 1,501 and setting `s3.nodes`, at the node whose entry failed. Checked again with `s3.nodes` at 5,000 through the library builder, the replay request's `stage_limits` entry and FR-255's settings operation given `s3.nodes=5000`, it checks each time. With `s3.work_units` one below the declaration's measured work `w`, it stops with `stage_limit_exceeded`/`work-budget-exceeded` naming bound `w - 1`, count `w` and setting `s3.work_units`, at the declaration's span: the work charge is made once per declaration, not per node. At `w` the declaration's own charge is admitted, and the stop is a later lowering charge. | Test (TC-727) |
| FR-258-AC-5 | Every node body lowering writes is in QSpec FR-322's five-stratum v2 body grammar (Leaf, Group, Tuple, Member, Body): across the checked packages of TC-415 and of AC-1, no `application` argument, `aggregate` member or `binding` value holds a term of its own stratum or a higher one, no `binding` holds a `binding`, and every composite subterm is a `reference` to its own node. | Test (TC-725) |
| FR-258-AC-6 | The text-leaf walk's heap stack never holds more entries than the visits it has entered, and enters two visits per charged record: over a chain of 1,000 records ending in one `Text` field it enters `2 × 1,001 + 1` visits and charges at least one work unit per record. | Test (TC-726) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.3 and D-6.
- [NFR-011](../non-functional/NFR-011-bound-value-checking-work.md) sets the
  checking ceilings and their defaults.
- [FR-062](FR-062-implement-checked-family-contract.md) defines the
  checked-family contract and its stage limits.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) and
  [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  define type keying, lowering and the text-leaf walk.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names
  each S3 setting.
- [FR-356](FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md)
  defines arena order, the walker toolkit and the deep tests on every
  public core entry point.

## Status

- FR-258-AC-1, AC-2, AC-3, AC-5 and AC-6: backed (TC-725, TC-726, TC-727).
- FR-258-AC-4: the node stop and the work stop through package checking and
  the library builder are backed (TC-727). Remainder: B5 (the setting names
  `s3.nodes` and `s3.work_units` in an outcome, the replay request's
  `stage_limits` entry and FR-255's settings operation).
- FR-258 Behavior 3: the checked-node walks are on the toolkit. Remainder:
  `Typer::run` (typing.rs) and expression lowering's frame stacks
  (lowering.rs), each a continuation machine over many heterogeneous frames
  whose results the toolkit's enter/exit protocol would carry on side stacks;
  they keep heap stacks, so nothing recurses, and move onto the toolkit in a
  follow-up slice of their own.

## References

- QSpec FR-146 (no checking depth limit; no host stack outcome) and FR-322
  (the v2 body grammar).
- QSpec FR-460, the ecosystem depth rule, and FR-322 AC-39 to AC-42, the
  stratified v2 body grammar (Linear STD-143, which supersedes STD-125).
- Linear QSL-381.
